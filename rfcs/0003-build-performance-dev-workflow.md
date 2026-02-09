# RFC 0003: Build Performance & Developer Workflow

- **Status:** Accepted
- **Author(s):** @ap-1
- **Created:** 2026-02-09
- **Updated:** 2026-02-09

## Overview

This RFC defines Terrier's build optimization strategy and developer workflow, including hot reloading tooling, compilation acceleration techniques, and the separation between development and production build configurations.

## Motivation

Rust compile times directly impact developer productivity. Additionally, the development and production environments have different optimization goals: development prioritizes fast iteration while production prioritizes runtime performance. We need a strategy that optimizes both without creating maintenance burden or configuration drift.

## Goals

- Minimize developer iteration time through hot reloading and compilation optimization
- Optimize CI and production build times without sacrificing runtime performance  
- Maximize cache reuse across local development, CI, and production builds
- Maintain clear separation between development and production build configurations
- Use a single source of truth for build settings within each environment

## Non-Goals

- Workspace structure and organization (deferred to future RFC)
- Benchmarking actual performance improvements (implementation phase)
- Optimizing test execution time (separate concern)

## Detailed Design

### Build Tool

We use crate2nix through devenv's `languages.rust.import` interface. This was chosen by the devenv team after evaluating available Rust packaging tools.

crate2nix generates per-crate Nix derivations, providing the finest caching granularity. This means Nix rebuilds only the specific crates that changed, rather than rebuilding the entire project or dependency closure.

```nix
# devenv.nix
{ config, ... }:
let
  terrier = config.languages.rust.import ./. {};
in
{
  packages = [ terrier ];
  outputs = { inherit terrier; };
}
```

### Hot Reloading

We use Subsecond (via the `dx` CLI with `--hotpatch` flag) for hot reloading during active development. This tool patches the running process at runtime via jump tables rather than performing full rebuilds.

```nix
# devenv.nix
packages = [ pkgs.dioxus-cli ];
```

Hot reloading works at a different layer than compilation, so it's compatible with incremental compilation. Incremental compilation speeds up the initial build, while hot patching handles runtime updates.

### Compilation Optimization

#### Fast Linkers

**Linux:** mold

```nix
# devenv.nix
languages.rust.mold = true;
```

**macOS:** lld

```toml
# .cargo/config.toml
[target.aarch64-apple-darwin]
rustflags = ["-C", "link-arg=-fuse-ld=lld"]
```

Fast linkers reduce link time without affecting the final binary, so they're used in both development and production builds.

#### Parallel Frontend

```nix
# devenv.nix
languages.rust = {
  channel = "nightly";
  rustflags = "-Zthreads=8";
};
```

The parallel frontend flag (`-Zthreads=8`) parallelizes rustc's frontend compilation stages. We use 8 threads as performance testing has shown diminishing returns beyond that value. This is a compile-time optimization that doesn't affect the output binary, so it's used in both development and production.

#### Codegen Backend: Cranelift vs LLVM

Cranelift is a faster code generator than LLVM but produces slower runtime code. It's available as a nightly component and has an active Rust Project Goal (2025h2) to reach production-ready status.

For development builds, we use Cranelift to prioritize compilation speed:

```toml
# .cargo/config.toml
[profile.dev]
codegen-backend = "cranelift"

[unstable]
codegen-backend = true
```

For production builds, we use the default LLVM backend to prioritize runtime performance.

### Build Profiles

#### Development Profile

Development builds optimize for fast iteration:

```nix
# devenv.nix
env = {
  CARGO_PROFILE_DEV_DEBUG = "0";  # No debug symbols (faster linking)
  CARGO_PROFILE_DEV_CODEGEN_BACKEND = "cranelift";  # Faster codegen
};
```

Configuration is done via environment variables rather than Cargo.toml to consolidate settings in devenv.nix.

#### Production Profile

Production builds optimize for runtime performance and debuggability:

```nix
# flake.nix
env = {
  CARGO_PROFILE_RELEASE_DEBUG = "1";  # Line numbers for production debugging
  CARGO_PROFILE_RELEASE_OPT_LEVEL = "3";  # Full optimization
};
```

Production builds use the LLVM backend (default) rather than Cranelift.

Both development and production builds use the same compile-time optimizations (fast linkers, parallel frontend) since these don't affect the output binary.

### Caching Strategy

#### Local Development

Local development uses incremental compilation (Cargo's default) and benefits from Nix's per-crate derivation caching through crate2nix.

We do not use sccache because it conflicts with incremental compilation. sccache caches individual rustc invocations, but incremental compilation artifacts are session-specific and constantly changing, resulting in poor cache hit rates.

#### CI and Production

Garnix CI reads flake outputs and builds all packages:

```bash
nix eval .#packages.x86_64-linux --apply builtins.attrNames
# ["terrier", "terrierImage"]

nix build .#terrier
nix build .#terrierImage
```

Results are cached to cache.garnix.io and shared across:
- All PRs and branches in the repository
- Other repositories using Garnix
- Local development machines

The cache stores per-crate derivations from crate2nix, so only changed crates need to rebuild across CI runs.

### Configuration Layout

Configuration is split across three files:

#### devenv.nix (Development Environment)

Primary configuration for development environment and settings:

```nix
{ config, ... }:
let
  terrier = config.languages.rust.import ./. {};
in
{
  env = {
    CARGO_PROFILE_DEV_DEBUG = "0";
    CARGO_PROFILE_DEV_CODEGEN_BACKEND = "cranelift";
  };

  languages.rust = {
    enable = true;
    channel = "nightly";
    mold = true;
    rustflags = "-Zthreads=8";
  };

  packages = [ terrier pkgs.dioxus-cli ];
  outputs = { inherit terrier; };
}
```

#### .cargo/config.toml (Platform-Specific)

Platform-specific settings that devenv cannot express:

```toml
[target.aarch64-apple-darwin]
rustflags = ["-C", "link-arg=-fuse-ld=lld"]

[profile.dev]
codegen-backend = "cranelift"

[unstable]
codegen-backend = true
```

devenv's `rustflags` setting applies globally, so platform-specific flags must be in .cargo/config.toml.

#### Cargo.toml (Package Metadata)

Only package metadata and dependencies. Profile settings are handled via environment variables in devenv.nix and flake.nix.

### Production Builds

Production builds are defined in a separate flake.nix:

```nix
{
  description = "Terrier production builds";
  
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    nix2container.url = "github:nlewo/nix2container";
  };
  
  outputs = { self, nixpkgs, nix2container, ... }:
    let
      system = "x86_64-linux";
      pkgs = nixpkgs.legacyPackages.${system};
      
      terrier = pkgs.rustPlatform.buildRustPackage {
        pname = "terrier";
        version = "0.1.0";
        src = ./.;
        cargoLock.lockFile = ./Cargo.lock;
        
        CARGO_PROFILE_RELEASE_DEBUG = "1";
        CARGO_PROFILE_RELEASE_OPT_LEVEL = "3";
        RUSTFLAGS = "-Zthreads=8";
      };
      
      terrierImage = nix2container.packages.${system}.nix2container.buildImage {
        name = "terrier";
        config = {
          entrypoint = [ "${terrier}/bin/terrier" ];
        };
      };
    in
    {
      packages.${system} = {
        inherit terrier terrierImage;
        default = terrier;
      };
    };
}
```

### Container Images: nix2container

Container images are built using nix2container rather than dockerTools.buildImage. nix2container separates image metadata from layer metadata and builds layers just-in-time during export, avoiding materialization of the full tar in the Nix store.

```nix
packages.terrierImage = nix2container.buildImage {
  name = "terrier";
  tag = "latest";
  config = {
    entrypoint = [ "${packages.terrier}/bin/terrier" ];
  };
  
  # Optional: explicit layer separation
  layers = [
    (nix2container.buildLayer {
      deps = [ /* dependencies */ ];
    })
  ];
};
```

## Alternatives Considered

### Build Tools

- **buildRustPackage:** Single derivation for entire project. Poor caching granularity - any change rebuilds everything.
- **naersk/crane:** Two derivations (dependencies and application). Better than buildRustPackage but still coarser than crate2nix.
- **crate2nix:** Per-crate derivations. Chosen for finest caching granularity.

### Hot Reloading

- **bacon:** TUI-based continuous build tool. Provides background compilation feedback with keyboard shortcuts for switching between tasks (clippy, tests, docs). Could be useful for passive monitoring but deferred since Subsecond provides active hot reloading and Rust Analyzer provides editor feedback.
- **cargo-watch:** Predecessor to bacon, now on life support. Not considered.

### Caching

- **sccache:** Caches individual rustc invocations. Conflicts with incremental compilation because incremental artifacts are session-specific. Not needed since crate2nix provides crate-level caching.

### Container Tools

- **dockerTools.buildImage:** Standard Nix container builder. Materializes full tar in Nix store.
- **dockerTools.streamLayeredImage:** Streams layers but less flexible than nix2container.
- **nix2container:** Chosen for just-in-time layer building and separated metadata.

## Open Questions

- Should we add bacon for passive background checking? Deferred but could revisit if there's a specific use case for TUI-based continuous feedback alongside Subsecond hot reloading.

## Implementation Phases

1. Configure devenv.nix with development optimizations (mold, parallel frontend, Cranelift)
2. Configure .cargo/config.toml for platform-specific settings
3. Create flake.nix for production builds
4. Set up nix2container for container images
5. Configure Garnix CI to build and cache production outputs
6. Document hot reloading workflow with Subsecond
