{
  description = "A basic Rust devshell";

  inputs = {
    nixpkgs.url      = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url  = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
      in
      with pkgs;
      {
        devShells.default = mkShell {
          buildInputs = [
            openssl
            pkg-config
            llvmPackages_latest.llvm
            caddy
            insomnia
            postgresql
            sqlx-cli
            nss_latest
            gdb
            cacert
            llvmPackages_latest.bintools
            zlib.out
            protobuf
            llvmPackages_latest.lld
            exa
            fd
            ripgrep
            (rust-bin.selectLatestNightlyWith( toolchain: toolchain.default.override {
              extensions= [ "rust-src" "rust-analyzer" ];
              targets = [ "wasm32-unknown-unknown" ];
            }))
            cargo-watch
          ];

          shellHook = ''
            alias ls=exa
            alias find=fd
            alias grep=ripgrep
            '';
        };
      }
    );
}
