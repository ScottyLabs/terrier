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

  packages = (with pkgs; [
    # Native libraries for samael (SAML)
    libxml2
    xmlsec
    libtool
    pkg-config
    openssl

    # Database tooling
    sea-orm-cli
  ]);

  env = {
    HOST = "127.0.0.1";
    PORT = "3000";

    # saml-proxy configuration
    SAML_PROXY_BASE_URL = "http://localhost:8443";
    SAML_PROXY_ENTITY_ID = "https://saml-proxy.example.com";
    SAML_PROXY_IDP_CERT_PATH = "crates/saml-proxy/certs/idp-cert.pem";
    SAML_PROXY_IDP_KEY_PATH = "crates/saml-proxy/certs/idp-key.pem";

    LIBCLANG_PATH = "${pkgs.llvmPackages.libclang.lib}/lib";
  };

  treefmt.config.programs.mdformat.excludes = [ "sites/docs/src/content/**" ];

  scripts = {
    migration.exec = ''sea-orm-cli migrate generate "$1" -d crates/migration'';
    migrate.exec = "sea-orm-cli migrate up -d crates/migration";
    generate-entities.exec = "sea-orm-cli generate entity -o crates/entity/src --with-serde both --lib --model-extra-derives 'utoipa::ToSchema' --enum-extra-derives 'utoipa::ToSchema'";
    generate-api.exec = "cd app && deno task generate-api";
  };
}
