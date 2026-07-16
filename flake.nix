{
  description = "Terrier";

  nixConfig = {
    extra-substituters = [ "https://scottylabs.cachix.org" ];
    extra-trusted-public-keys = [
      "scottylabs.cachix.org-1:hajjEX5SLi/Y7yYloiXTt2IOr3towcTGRhMh1vu6Tjg="
    ];
  };

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    scottylabs = {
      url = "git+https://codeberg.org/ScottyLabs/devenv";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { nixpkgs, scottylabs, ... }:
    let
      supportedSystems = [ "x86_64-linux" "aarch64-linux" ];
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
              name = "terrier-docs";
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
          in
          {
            inherit terrier app docs;
          }
        ))
      );
    };
}
