{ pkgs, config, inputs, ... }:

let
  cargoNix = pkgs.callPackage ./Cargo.nix { };
  terrier = cargoNix.workspaceMembers.terrier-server.build;
in
{
  imports = [
    inputs.scottylabs.devenvModules.default
  ];

  scottylabs = {
    enable = true;
    project.name = "terrier";
    garage.enable = true;
    postgres.enable = true;
    valkey.enable = true;
    kennel.services.terrier = {
      customDomain = "api.terrier.scottylabs.org";
    };
  };

  processes.terrier = {
    exec = "${terrier}/bin/terrier-server";
    process-compose.readiness_probe = {
      http_get = {
        host = "localhost";
        port = 3000;
        path = "/health";
      };
      initial_delay_seconds = 1;
      period_seconds = 1;
    };
  };

  services.redis.port = 0;

  cachix.pull = [ "scottylabs" ];

  packages = [
    terrier
    inputs.bun2nix.packages.${pkgs.stdenv.system}.default
  ] ++ (with pkgs; [
    # Project tooling
    dioxus-cli
    just
    bun

    # Native libraries for samael (SAML)
    libxml2
    xmlsec
    libtool
    pkg-config
    openssl

    # Database tooling
    sea-orm-cli
  ]);

  outputs = { inherit terrier; };

  env = {
    CARGO_PROFILE_DEV_DEBUG = "0";

    REDIS_URL = "redis+unix://$REDIS_UNIX_SOCKET";
    HOST = "127.0.0.1";
    PORT = "3000";
    RUST_LOG = "debug";

    # saml-proxy configuration
    SAML_PROXY_BASE_URL = "http://localhost:8443";
    SAML_PROXY_ENTITY_ID = "https://saml-proxy.example.com";
    SAML_PROXY_IDP_CERT_PATH = "crates/saml-proxy/certs/idp-cert.pem";
    SAML_PROXY_IDP_KEY_PATH = "crates/saml-proxy/certs/idp-key.pem";

    LIBCLANG_PATH = "${pkgs.llvmPackages.libclang.lib}/lib";
  };

  languages.rust = {
    enable = true;
    channel = "nightly";
    components = [
      "rustc"
      "cargo"
      "clippy"
      "rustfmt"
      "rust-analyzer"
      "rust-src"
      "llvm-tools-preview"
    ];
    mold.enable = pkgs.stdenv.isLinux;
    rustflags = "-Zthreads=8";
  };

  services.postgres.initialScript = ''
    CREATE USER terrier WITH PASSWORD 'terrier';
    GRANT ALL PRIVILEGES ON DATABASE terrier TO terrier;
    ALTER DATABASE terrier OWNER TO terrier;
  '';

  treefmt = {
    enable = true;
    config.programs = {
      nixpkgs-fmt = {
        enable = true;
        excludes = [ "Cargo.nix" "bun.nix" ];
      };
      rustfmt.enable = true;
      mdformat = {
        enable = true;
        excludes = [ "sites/docs/src/content/**" ];
      };
    };
    # TODO: treefmt-nix's built-in biome program doesn't support pointing to an
    # existing biome.jsons. We use a custom formatter so biome.json remains the
    # single source of truth for both treefmt and editor integration.
    config.settings.formatter.biome = {
      command = "${pkgs.biome}/bin/biome";
      options = [ "check" "--write" "--no-errors-on-unmatched" "--config-path" "${config.devenv.root}/biome.json" ];
      # TODO: biome check --write doesn't format .svelte files yet, only lints
      includes = [ "*.js" "*.ts" "*.mjs" "*.mts" "*.cjs" "*.cts" "*.jsx" "*.tsx" "*.d.ts" "*.d.cts" "*.d.mts" "*.json" "*.jsonc" "*.css" ];
    };
  };

  git-hooks.hooks = {
    treefmt.enable = true;
    clippy = {
      enable = true;
      packageOverrides.cargo = config.languages.rust.toolchainPackage;
      packageOverrides.clippy = config.languages.rust.toolchainPackage;
    };
    cargo-nix-update = {
      enable = true;
      name = "cargo-nix-update";
      entry = "${pkgs.writeShellScript "cargo-nix-update" ''
        if git diff --cached --name-only | grep -q '^Cargo\.\(toml\|lock\)'; then
          ${pkgs.crate2nix}/bin/crate2nix generate
          git add Cargo.nix
        fi
      ''}";
      files = "Cargo\\.(toml|lock)$";
      language = "system";
      pass_filenames = false;
    };
  };
}
