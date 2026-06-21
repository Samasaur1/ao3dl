{ lib, rustPlatform, pkg-config, openssl }:

let
  cargoToml = (builtins.fromTOML (builtins.readFile ./Cargo.toml)).package;
in
  rustPlatform.buildRustPackage {
    pname = cargoToml.name;
    version = cargoToml.version;

    src = ./.;

    nativeBuildInputs = [
      pkg-config
    ];

    buildInputs = [
      openssl
    ];

    cargoHash = "sha256-1JXkiiFZIALI6MD1tccFm2uG5GdaZ9aA4Yx3EPM1AnA=";

    meta = {
      mainProgram = "ao3dl";
    };
  }
