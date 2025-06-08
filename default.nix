{ lib, rustPlatform, }:

let
  cargoToml = (builtins.fromTOML (builtins.readFile ./Cargo.toml)).package;
in
  rustPlatform.buildRustPackage {
    pname = cargoToml.name;
    version = cargoToml.version;

    src = ./.;

    useFetchCargoVendor = true;
    cargoHash = "sha256-T7En/qxIMbiVeQOCJI+mFs4cZlVIj72UmSVaBSuLC8Q=";

    meta = {
      mainProgram = "ao3dl";
    };
  }
