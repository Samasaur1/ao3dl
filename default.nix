{ lib, rustPlatform, }:

let
  cargoToml = (builtins.fromTOML (builtins.readFile ./Cargo.toml)).package;
in
  rustPlatform.buildRustPackage {
    pname = cargoToml.name;
    version = cargoToml.version;

    src = ./.;

    useFetchCargoVendor = true;
    cargoHash = "sha256-ndmaHPRBTkx+Fbl51Zx76Cc1YB6NYR4YPUSZ9RtanTE=";

    meta = {
      mainProgram = "ao3dl";
    };
  }
