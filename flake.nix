{
  description = "Terrier";

  nixConfig = {
    extra-substituters = [ "https://scottylabs.cachix.org" ];
    extra-trusted-public-keys = [
      "scottylabs.cachix.org-1:hajjEX5SLi/Y7yYloiXTt2IOr3towcTGRhMh1vu6Tjg="
    ];
  };

  inputs = {
    scottylabs.url = "git+https://git.cmu.dev/ScottyLabs/kennel";
    nixpkgs.follows = "scottylabs/nixpkgs";
    flake-parts.follows = "scottylabs/flake-parts";
  };

  outputs =
    inputs:
    inputs.flake-parts.lib.mkFlake { inherit inputs; } {
      imports = [ inputs.scottylabs.flakeModules.default ];
      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "aarch64-darwin"
      ];

      perSystem =
        {
          pkgs,
          lib,
          helpers,
          ...
        }:
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

          samlLibraries = with pkgs; [
            xmlsec
            libxml2
            libtool
            openssl
            libxslt
          ];

          app = helpers.buildDenoTask {
            src = ./app;
            pname = "terrier-app";
            inherit (lib.importJSON ./app/package.json) version;
          };
        in
        {
          scottylabs = {
            project.name = "terrier";

            rust.enable = true;
            deno = {
              enable = true;
              svelte.enable = true;
              dirs = [ "app" ];
            };

            garage.enable = true;
            postgres.enable = true;
            valkey.enable = true;

            ricochet = {
              enable = true;
              appUrl = "http://localhost:5173";
            };

            shell = {
              packages =
                with pkgs;
                [
                  # Focused source search and structured output
                  ripgrep
                  fd
                  jq

                  # Native libraries for samael (SAML)
                  pkg-config

                  # Database tooling
                  seaOrmCli

                  # General tooling
                  git
                ]
                ++ samlLibraries;

              env = {
                HOST = "127.0.0.1";
                PORT = "3000";
                INSTANCE_ADMIN_SUBJECTS = "[]";

                # saml-proxy configuration
                SAML_PROXY_BASE_URL = "http://localhost:8443";
                SAML_PROXY_ENTITY_ID = "https://saml-proxy.example.com";
                SAML_PROXY_IDP_CERT_PATH = "crates/saml-proxy/certs/idp-cert.pem";
                SAML_PROXY_IDP_KEY_PATH = "crates/saml-proxy/certs/idp-key.pem";

                LIBCLANG_PATH = "${pkgs.llvmPackages.libclang.lib}/lib";

                NIX_ENFORCE_PURITY = "0";
              };

              # the Astro docs site is outside deno.dirs
              hook = ''
                (cd "$SCOTTYLABS_ROOT/sites/docs" && deno install --frozen && deno task astro sync)
              '';

              scripts = {
                migration = ''sea-orm-cli migrate generate "$1" -d crates/terrier-server/migration'';
                migrate = "cargo run -p migration -- up";
                generate-entities = "sea-orm-cli generate entity -o crates/terrier-server/src/entities --with-serde both";
                generate-api = "cd app && deno task generate-api";
              };
            };
          };

          treefmt.programs.mdformat.excludes = [ "sites/docs/src/content/**" ];
          pre-commit.settings.hooks.oxlint.excludes = [ "^app/src/lib/api/schema[.]d[.]ts$" ];

          services.terrier = {
            package = helpers.buildRustService {
              src = ./.;
              pname = "terrier-server";
              nativeBuildInputs = [ pkgs.pkg-config ];
              buildInputs = samlLibraries;
              buildArgs.cargoExtraArgs = "--locked -p terrier-server";
            };
            customDomain = "api.terrier.scottylabs.org";
            env.STATIC_DIR = "${app}";
          };

          sites.docs = {
            package = helpers.buildDenoTask {
              src = ./sites/docs;
              pname = "terrier-docs";
              inherit (lib.importJSON ./sites/docs/package.json) version;
            };
            customDomain = "docs.terrier.build";
          };

          packages.migrator = helpers.buildRustService {
            src = ./.;
            pname = "migration";
            nativeBuildInputs = [ pkgs.pkg-config ];
            buildInputs = [ pkgs.openssl ];
            buildArgs.cargoExtraArgs = "--locked -p migration";
          };
        };
    };
}
