# Rust User Shell
let
  # Unstable Channel | Rolling Release
  pkgs = import (fetchTarball("channel:nixpkgs-unstable")) { };

  packages = with pkgs; [
    pkg-config
    rustc
    cargo
    rustfmt
    rust-analyzer
    trunk
  ];
in
pkgs.mkShell {
  buildInputs = packages;
}
