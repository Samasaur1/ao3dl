{ mkShell, pkg-config, cargo, rustc, rust-analyzer, rustfmt, openssl }:

mkShell {
  name = "ao3dl dev shell";

  nativeBuildInputs = [
    pkg-config
  ];

  buildInputs = [
    cargo
    rustc
    rust-analyzer
    rustfmt
    openssl
  ];
}
