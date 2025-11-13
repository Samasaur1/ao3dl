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

    cargoHash = "sha256-tcn3fBSR/f194mkjtiun1yVV64qYRpwZSbPTlQa3gA4=";

    meta = {
      mainProgram = "ao3dl";
    };
  }
