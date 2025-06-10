{ lib, rustPlatform, }:

let
  cargoToml = (builtins.fromTOML (builtins.readFile ./Cargo.toml)).package;
in
  rustPlatform.buildRustPackage {
    pname = cargoToml.name;
    version = cargoToml.version;

    src = ./.;

    useFetchCargoVendor = true;
    cargoHash = "sha256-fo/KDiqa+bKdni0xWMB9gFEJtZxKa2IyMb8pQPd/eJI=";

    meta = {
      mainProgram = "ao3dl";
    };
  }
