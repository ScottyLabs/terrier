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
    scottylabs = {
      url = "git+https://codeberg.org/ScottyLabs/devenv";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, devenv, scottylabs, ... }:
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
            pkgs = nixpkgs.legacyPackages.${system};
            lib = scottylabs.mkLib pkgs;

            app = lib.buildDenoTask {
              src = ./app;
              pname = "terrier-app";
              version = (builtins.fromJSON (builtins.readFile ./app/package.json)).version;
            };

            docs = lib.buildMdbook {
              src = ./sites/docs;
              pname = "terrier-docs";
            };

            terrier = lib.buildRustService {
              src = ./.;
              pname = "terrier-server";
              nativeBuildInputs = [ pkgs.pkg-config ];
              buildInputs = with pkgs; [ xmlsec libxml2 libtool openssl libxslt ];
              buildArgs = {
                cargoExtraArgs = "-p terrier-server";
                preBuild = ''
                  mkdir -p assets
                  cp -r ${app}/* assets/
                '';
              };
            };

            saml-proxy = lib.buildRustService {
              src = ./.;
              pname = "saml-proxy";
              nativeBuildInputs = [ pkgs.pkg-config ];
              buildInputs = with pkgs; [ xmlsec libxml2 libtool openssl libxslt ];
              buildArgs.cargoExtraArgs = "-p saml-proxy";
            };
          in
          {
            inherit terrier saml-proxy app docs;
            default = terrier;
            devenv = devenv.packages.${system}.devenv;
          }
        ))
      );
    };
}
