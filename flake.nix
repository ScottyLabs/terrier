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
              packages = with pkgs; [
                # Focused source search and structured output
                ripgrep
                fd
                jq

                # OpenSSL for sqlx's native-tls
                pkg-config
                openssl

                # Database tooling
                seaOrmCli

                # Documentation
                zensical

                # General tooling
                git
              ];

              env = {
                HOST = "127.0.0.1";
                PORT = "3000";
                INSTANCE_ADMIN_SUBJECTS = "[]";

                NIX_ENFORCE_PURITY = "0";
              };

              scripts = {
                migration = ''sea-orm-cli migrate generate "$1" -d crates/terrier-server/migration'';
                migrate = "cargo run -p migration -- up";
                generate-entities = "sea-orm-cli generate entity -o crates/terrier-server/src/entities --with-serde both";
                generate-api = "cd app && deno task generate-api";
                docs = "cd sites/docs && zensical serve";
              };
            };
          };

          pre-commit.settings.hooks.oxlint.excludes = [ "^app/src/lib/api/schema[.]d[.]ts$" ];

          services.terrier = {
            package = helpers.buildRustService {
              src = ./.;
              pname = "terrier-server";
              nativeBuildInputs = [ pkgs.pkg-config ];
              buildInputs = [ pkgs.openssl ];
              buildArgs.cargoExtraArgs = "--locked -p terrier-server";
            };
            customDomain = "api.terrier.scottylabs.org";
            env.STATIC_DIR = "${app}";
          };

          sites.docs = {
            package = helpers.buildZensical {
              src = ./sites/docs;
              name = "terrier-docs";
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
