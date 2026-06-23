{ pkgs, inputs, ... }:

{
  imports = [
    inputs.scottylabs.devenvModules.default
  ];

  scottylabs = {
    enable = true;
    project.name = "terrier";

    rust.enable = true;
    secrets.enable = true;
    deno = {
      enable = true;
      svelte.enable = true;
    };
    garage.enable = true;
    postgres.enable = true;
    valkey.enable = true;
    kennel.services.terrier = {
      customDomain = "api.terrier.scottylabs.org";
    };
    kennel.services.saml-proxy = {
      customDomain = "auth.terrier.build";
    };
    kennel.sites.docs = {
      spa = false;
      customDomain = "docs.terrier.build";
    };
    ricochet = {
      enable = true;
      appUrl = "http://localhost:5173";
    };
  };

  cachix.pull = [ "scottylabs" ];

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
    REDIS_URL = "redis+unix://$REDIS_UNIX_SOCKET";
    HOST = "127.0.0.1";
    PORT = "3000";

    # saml-proxy configuration
    SAML_PROXY_BASE_URL = "http://localhost:8443";
    SAML_PROXY_ENTITY_ID = "https://saml-proxy.example.com";
    SAML_PROXY_IDP_CERT_PATH = "crates/saml-proxy/certs/idp-cert.pem";
    SAML_PROXY_IDP_KEY_PATH = "crates/saml-proxy/certs/idp-key.pem";

    LIBCLANG_PATH = "${pkgs.llvmPackages.libclang.lib}/lib";
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
      };
      rustfmt.enable = true;
      mdformat = {
        enable = true;
        excludes = [ "sites/docs/src/content/**" ];
      };
    };
  };

  git-hooks.hooks.treefmt.enable = true;

  scripts = {
    migration.exec = ''sea-orm-cli migrate generate "$1" -d crates/migration'';
    migrate.exec = "sea-orm-cli migrate up -d crates/migration";
    generate-entities.exec = "sea-orm-cli generate entity -o crates/entity/src --with-serde both --lib --model-extra-derives 'utoipa::ToSchema' --enum-extra-derives 'utoipa::ToSchema'";
    generate-api.exec = "cd app && deno task generate-api";
  };
}
