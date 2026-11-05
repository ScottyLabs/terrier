{ pkgs, inputs, ... }:

let
  seaOrmCli = pkgs.sea-orm-cli.overrideAttrs (
    finalAttrs: _: {
      version = "1.1.20";
      src = pkgs.fetchCrate {
        pname = "sea-orm-cli";
        inherit (finalAttrs) version;
        hash = "sha256-n7QkCnMF15UMLPPEF093ylzzDESGKKG/q4Y2jvdTcUo=";
      };
      cargoDeps = pkgs.rustPlatform.importCargoLock {
        lockFile = "${finalAttrs.src}/Cargo.lock";
      };
    }
  );
in
{
  imports = [
    inputs.scottylabs.devenvModules.default
  ];

  scottylabs = {
    enable = true;
    project.name = "terrier";

    secrets.enable = true;

    rust.enable = true;
    deno = {
      enable = true;
      svelte.enable = true;
      dirs = [ "app" ];
    };

    garage.enable = true;
    postgres.enable = true;
    valkey.enable = true;

    kennel = {
      services.terrier.customDomain = "api.terrier.scottylabs.org";
      sites.docs = {
        spa = false;
        customDomain = "docs.terrier.build";
      };
    };
    ricochet = {
      enable = true;
      appUrl = "http://localhost:5173";
    };
  };

  packages = with pkgs; [
    # Focused source search and structured output
    ripgrep
    fd
    jq

    # Native libraries for samael (SAML)
    libxml2
    xmlsec
    libtool
    pkg-config
    openssl

    # Database tooling
    seaOrmCli

    # General tooling
    git
    inputs.collar.packages.${pkgs.stdenv.hostPlatform.system}.collar
  ];

  env = {
    HOST = "127.0.0.1";
    PORT = "3000";

    # saml-proxy configuration
    SAML_PROXY_BASE_URL = "http://localhost:8443";
    SAML_PROXY_ENTITY_ID = "https://saml-proxy.example.com";
    SAML_PROXY_IDP_CERT_PATH = "crates/saml-proxy/certs/idp-cert.pem";
    SAML_PROXY_IDP_KEY_PATH = "crates/saml-proxy/certs/idp-key.pem";

    LIBCLANG_PATH = "${pkgs.llvmPackages.libclang.lib}/lib";

    NIX_ENFORCE_PURITY = "0";
  };

  treefmt.config.programs.mdformat.excludes = [ "sites/docs/src/content/**" ];

  tasks."docs:install" = {
    exec = "cd sites/docs && deno install --frozen && deno task astro sync";
    before = [
      "devenv:enterShell"
      "devenv:git-hooks:run"
    ];
  };

  scripts = {
    migration.exec = ''sea-orm-cli migrate generate "$1" -d crates/terrier-server/migration'';
    migrate.exec = "cargo run -p migration -- up";
    generate-entities.exec = "${seaOrmCli}/bin/sea-orm-cli generate entity -o crates/terrier-server/src/entities --with-serde both";
    generate-api.exec = "cd app && deno task generate-api";
  };
}
