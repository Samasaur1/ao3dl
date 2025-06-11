{ lib, rustPlatform, }:

let
  cargoToml = (builtins.fromTOML (builtins.readFile ./Cargo.toml)).package;
in
  rustPlatform.buildRustPackage {
    pname = cargoToml.name;
    version = cargoToml.version;

    src = ./.;

    useFetchCargoVendor = true;
    cargoHash = "sha256-4QwWRfYm3XPgBfXwhNc4+pWleQdwg2Cm3LQsEi8Fi0Y=";

    meta = {
      mainProgram = "ao3dl";
    };
  }
