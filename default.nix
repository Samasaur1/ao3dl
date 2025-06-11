{ lib, rustPlatform, }:

let
  cargoToml = (builtins.fromTOML (builtins.readFile ./Cargo.toml)).package;
in
  rustPlatform.buildRustPackage {
    pname = cargoToml.name;
    version = cargoToml.version;

    src = ./.;

    useFetchCargoVendor = true;
    cargoHash = "sha256-9Gl9+7e98n9uX+WbR3OPK7M+j93MyiQAA7QlGYKq8ZM=";

    meta = {
      mainProgram = "ao3dl";
    };
  }
