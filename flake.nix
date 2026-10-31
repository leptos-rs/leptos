{
  description = "A basic Rust devshell for NixOS users developing Leptos";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      nixpkgs,
      rust-overlay,
      ...
    }:
    let
      overlays = [ (import rust-overlay) ];
      supportedSystems = [
        "x86_64-linux"
        "aarch64-linux"
        "x86_64-darwin"
        "aarch64-darwin"
      ];
      forEachSupportedSystem =
        f:
        nixpkgs.lib.genAttrs supportedSystems (
          system:
          f {
            pkgs = import nixpkgs { inherit overlays system; };
          }
        );
    in
    {
      devShells = forEachSupportedSystem (
        { pkgs }: {
          default = pkgs.mkShell {
            buildInputs =
              with pkgs;
              [
                gcc
                glib
                openssl
                pkg-config
                cacert
                cargo-make
                trunk
                (rust-bin.selectLatestNightlyWith (
                  toolchain:
                  toolchain.default.override {
                    extensions = [
                      "rust-src"
                      "rust-analyzer"
                    ];
                    targets = [ "wasm32-unknown-unknown" ];
                  }
                ))
              ]
              ++ pkgs.lib.optionals pkg.stdenv.hostPlatform.isDarwin [
                darwin.apple_sdk.frameworks.SystemConfiguration
              ];
          };
        }
      );
    };
}
