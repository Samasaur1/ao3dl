{ lib, rustPlatform, }:

let
  cargoToml = (builtins.fromTOML (builtins.readFile ./Cargo.toml)).package;
in
  rustPlatform.buildRustPackage {
    pname = cargoToml.name;
    version = cargoToml.version;

    src = ./.;

    useFetchCargoVendor = true;
    cargoHash = "sha256-ichutRIIu8p0rMR/+l02YMYCMe0jyAMTd/U8cWELHc0=";

    meta = {
      mainProgram = "ao3dl";
    };
  }
