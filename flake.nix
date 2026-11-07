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
      url = "git+https://git.cmu.dev/ScottyLabs/kennel";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    { nixpkgs, scottylabs, ... }:
    let
      supportedSystems = [
        "x86_64-linux"
        "aarch64-linux"
      ];
      forAllSystems = nixpkgs.lib.genAttrs supportedSystems;
    in
    {
      packages = forAllSystems (
        system:
        let
          pkgs = nixpkgs.legacyPackages.${system};
          lib = scottylabs.mkLib pkgs;

          app = lib.buildDenoTask {
            src = ./app;
            pname = "terrier-app";
            inherit ((builtins.fromJSON (builtins.readFile ./app/package.json))) version;
          };

          docs = lib.buildDenoTask {
            src = ./sites/docs;
            pname = "terrier-docs";
            inherit ((builtins.fromJSON (builtins.readFile ./sites/docs/package.json))) version;
          };

          terrier = lib.buildRustService {
            src = ./.;
            pname = "terrier-server";
            nativeBuildInputs = [
              pkgs.pkg-config
              pkgs.makeWrapper
            ];
            buildInputs = with pkgs; [
              xmlsec
              libxml2
              libtool
              openssl
              libxslt
            ];
            buildArgs = {
              cargoExtraArgs = "-p terrier-server";
              postInstall = ''
                wrapProgram "$out/bin/terrier" --set-default STATIC_DIR ${app}
              '';
            };
          };
        in
        {
          inherit terrier app docs;
          migrator = lib.buildRustService {
            src = ./.;
            pname = "migration";
            nativeBuildInputs = [ pkgs.pkg-config ];
            buildInputs = [ pkgs.openssl ];
            buildArgs.cargoExtraArgs = "--locked -p migration";
          };
        }
      );
    };
}
