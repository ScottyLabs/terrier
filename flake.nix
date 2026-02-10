{
  description = "Terrier";

  nixConfig = {
    extra-substituters = [ "https://cache.garnix.io" ];
    extra-trusted-public-keys = [
      "cache.garnix.io:CTFPyKSLcx5RMJKfLo5EEPUObbA78b0YQ2DTCJXqr9g="
    ];
  };

  inputs = {
    nixpkgs.url = "github:cachix/devenv-nixpkgs/rolling";
    devenv.url = "github:cachix/devenv";
    nix2container = {
      url = "github:nlewo/nix2container";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    bun2nix = {
      url = "github:nix-community/bun2nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, devenv, nix2container, bun2nix, ... }:
    let
      supportedSystems = [ "x86_64-linux" "aarch64-linux" "aarch64-darwin" "x86_64-darwin" ];
      forAllSystems = nixpkgs.lib.genAttrs supportedSystems;
      pkgsFor = system: nixpkgs.legacyPackages.${system};
    in
    {
      formatter = forAllSystems (system: (pkgsFor system).nixpkgs-fmt);

      checks = forAllSystems (system:
        let
          pkgs = pkgsFor system;
          src = ./.;
        in
        {
          nixpkgs-fmt = pkgs.runCommand "check-nixpkgs-fmt" { nativeBuildInputs = [ pkgs.nixpkgs-fmt ]; } ''
            find ${src} -name '*.nix' -not -path '*/node_modules/*' -not -name 'Cargo.nix' -not -name 'bun.nix' | xargs nixpkgs-fmt --check
            touch $out
          '';

          biome = pkgs.runCommand "check-biome" { nativeBuildInputs = [ pkgs.biome ]; } ''
            biome ci --config-path ${src}/biome.json ${src}/web/src
            touch $out
          '';

          cargo-nix-stale = pkgs.runCommand "check-cargo-nix-stale" { nativeBuildInputs = [ pkgs.crate2nix pkgs.diffutils ]; } ''
            mkdir work && cd work
            cp ${src}/Cargo.toml ${src}/Cargo.lock .
            cp -r ${src}/crates .
            crate2nix generate
            diff -q Cargo.nix ${src}/Cargo.nix || (echo "Cargo.nix is stale — run 'crate2nix generate'" && exit 1)
            touch $out
          '';

          bun-nix-stale = pkgs.runCommand "check-bun-nix-stale" { nativeBuildInputs = [ bun2nix.packages.${system}.default pkgs.diffutils ]; } ''
            mkdir work && cd work
            cp ${src}/web/bun.lock .
            bun2nix
            diff -q bun.nix ${src}/web/bun.nix || (echo "web/bun.nix is stale — run 'cd web && bun2nix'" && exit 1)
            touch $out
          '';
        }
      );

      packages = forAllSystems (system:
        let
          pkgs = pkgsFor system;
        in
        {
          devenv = devenv.packages.${system}.devenv;
        }
        // (nixpkgs.lib.optionalAttrs (system == "x86_64-linux") (
          let
            nix2containerPkgs = nix2container.packages.${system};
            b2n = bun2nix.packages.${system}.default;

            terrierWeb = b2n.mkDerivation {
              pname = "terrier-web";
              version = "0.1.0";
              src = ./web;

              bunDeps = b2n.fetchBunDeps {
                bunNix = ./web/bun.nix;
              };

              buildPhase = ''
                bun run build
              '';

              installPhase = ''
                mkdir -p $out
                cp -r dist/* $out/
              '';
            };

            cargoNix = pkgs.callPackage ./Cargo.nix { };

            terrier = cargoNix.workspaceMembers.terrier-server.build.override {
              crateOverrides = pkgs.defaultCrateOverrides // {
                terrier-server = attrs: {
                  CARGO_PROFILE_RELEASE_DEBUG = "1";
                  CARGO_PROFILE_RELEASE_OPT_LEVEL = "3";
                  RUSTFLAGS = "-Zthreads=8";

                  preBuild = ''
                    mkdir -p assets
                    cp -r ${terrierWeb}/* assets/
                  '';
                };
              };
            };

            terrierImage = nix2containerPkgs.nix2container.buildImage {
              name = "ghcr.io/scottylabs/terrier";
              tag = "latest";
              config = {
                entrypoint = [ "${terrier}/bin/terrier" ];
                env = [
                  "HOST=0.0.0.0"
                  "PORT=3000"
                ];
                exposedPorts = {
                  "3000/tcp" = { };
                };
              };
            };
          in
          {
            inherit terrier terrierWeb terrierImage;
            default = terrier;
          }
        ))
      );
    };
}
