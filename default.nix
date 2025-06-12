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
    cargoHash = "sha256-/x1FBCvU+Mei3RKL8jmMLFE0L2wDRq7da4AELk0Ahzo=";

    meta = {
      mainProgram = "ao3dl";
    };
  }
