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

    cargoHash = "sha256-Wz3h3UOg5xraVBnms/9BpbL5K/BnYNlCFRbxPOyiJ5w=";

    meta = {
      mainProgram = "ao3dl";
    };
  }
