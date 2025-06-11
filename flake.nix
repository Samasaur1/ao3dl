{
  description = "Download fics from AO3";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs";
  };

  outputs = { self, nixpkgs, ... }:
    let
      allSystems = nixpkgs.lib.systems.flakeExposed;
      forAllSystems = nixpkgs.lib.genAttrs allSystems;
      define = f: forAllSystems (system:
        let
          pkgs = import nixpkgs {
            inherit system;
            config = {
            };
            overlays = [
            ];
          };
        in
          f pkgs
      );
    in {
      packages = define (pkgs: {
        default = pkgs.callPackage ./. { };
      });

      devShells = define (pkgs: {
        default = pkgs.mkShell {
          name = "ao3dl dev shell";

          nativeBuildInputs = [
            pkgs.pkg-config
          ];

          buildInputs = [
            pkgs.cargo
            pkgs.rustc
            pkgs.openssl
          ];
        };
      });
    };
}
