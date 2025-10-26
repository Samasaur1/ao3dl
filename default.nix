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

    useFetchCargoVendor = true;
    cargoHash = "sha256-gQP/o2xNJa5G/v69tGli7fblnQLQJcn+mYtuNWU03d8=";

    meta = {
      mainProgram = "ao3dl";
    };
  }
