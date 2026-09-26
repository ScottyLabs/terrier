{ pkgs, inputs, ... }:

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
      svelte.dir = "app";
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
    sea-orm-cli
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

  tasks."app:install" = {
    exec = "cd app && deno install --frozen";
    before = [
      "devenv:enterShell"
      "devenv:git-hooks:run"
    ];
  };

  tasks."docs:install" = {
    exec = "cd sites/docs && deno install --frozen && deno task astro sync";
    before = [
      "devenv:enterShell"
      "devenv:git-hooks:run"
    ];
  };

  scripts = {
    fetch.exec = ''exec cargo run --quiet --manifest-path "$DEVENV_ROOT/Cargo.toml" -p fetch -- "$@"'';
    migration.exec = ''sea-orm-cli migrate generate "$1" -d crates/migration'';
    migrate.exec = "sea-orm-cli migrate up -d crates/migration";
    generate-entities.exec = "sea-orm-cli generate entity -o crates/entity/src --with-serde both --lib --model-extra-derives 'utoipa::ToSchema' --enum-extra-derives 'utoipa::ToSchema'";
    generate-api.exec = "cd app && deno task generate-api";
  };
}
