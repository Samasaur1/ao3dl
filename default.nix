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

    cargoHash = "sha256-Ftskdt/mbP+cGx4yvo8Nb5U74xbhG3EXMQeGDk37csA=";

    meta = {
      mainProgram = "ao3dl";
    };
  }
