{
  description = "Terrier";

  nixConfig = {
    extra-substituters = [ "https://scottylabs.cachix.org" ];
    extra-trusted-public-keys = [
      "scottylabs.cachix.org-1:hajjEX5SLi/Y7yYloiXTt2IOr3towcTGRhMh1vu6Tjg="
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
              version = (builtins.fromJSON (builtins.readFile ./web/package.json)).version;
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

            terrierDocs = b2n.mkDerivation {
              pname = "terrier-docs";
              version = (builtins.fromJSON (builtins.readFile ./docs/package.json)).version;
              src = ./docs;

              bunDeps = b2n.fetchBunDeps {
                bunNix = ./docs/bun.nix;
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
              name = "codeberg.org/scottylabs/terrier";
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
            inherit terrier terrierWeb terrierDocs terrierImage;
            default = terrier;
          }
        ))
      );
    };
}
