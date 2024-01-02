{
  description = "Build Pavex tools";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";

    flake-utils.url = "github:numtide/flake-utils";
    
    cargo-px-git = {
      url = "github:/LukeMathWalker/cargo-px";
      flake = false;
    };
    cargo-pavex-git = {
      url = "github:LukeMathWalker/pavex";
      flake = false;
    };

    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs = {
        nixpkgs.follows = "nixpkgs";
        flake-utils.follows = "flake-utils";
      };
    };
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay, ... } @inputs:
    flake-utils.lib.eachDefaultSystem
      (system:
        let
          pkgs = import nixpkgs {
            inherit system;
            overlays = [ (import rust-overlay) ];
          };
          inherit (pkgs) lib;
          rustTarget = pkgs.rust-bin.selectLatestNightlyWith( toolchain: toolchain.default.override {
            extensions = [ "rust-src" "rust-analyzer" "rustc-codegen-cranelift-preview" "rust-docs-json"];
            targets = [ "wasm32-unknown-unknown" ];
          });


          cargo-pavex_cli-git = pkgs.rustPlatform.buildRustPackage rec {
            pname = "cargo-pavex-cli";
            version = "0.2.22";
            #buildFeatures = ["no_downloads"]; # cargo-leptos will try to download Ruby and other things without this feature

            src = inputs.cargo-pavex-git;
            sourceRoot = "source/libs";
            cargoLock = {
              lockFile = inputs.cargo-pavex-git + "/libs/Cargo.lock";
              outputHashes = {
                "matchit-0.7.3" = "sha256-1bhbWvLlDb6/UJ4j2FqoG7j3DD1dTOLl6RaiY9kasmQ=";
                #"pavex-0.1.0" = "sha256-NC7T1pcXJiWPtAWeiMUNzf2MUsYaRYxjLIL9fCqhExo=";
              };
            };
            #buildAndTestSubdir = "libs";
            cargoSha256 = "";
            nativeBuildInputs = [pkgs.pkg-config pkgs.openssl pkgs.git];

            buildInputs = with pkgs;
              [openssl pkg-config git]
              ++ lib.optionals stdenv.isDarwin [
              Security
            ];

            doCheck = false; # integration tests depend on changing cargo config

            meta = with lib; {
            description = "An easy-to-use Rust framework for building robust and performant APIs";
            homepage = "https://github.com/LukeMatthewWalker/pavex";
            changelog = "https://github.com/LukeMatthewWalker/pavex/blob/v${version}/CHANGELOG.md";
            license = with licenses; [mit];
            maintainers = with maintainers; [benwis];
          };
      };
          cargo-px-git = pkgs.rustPlatform.buildRustPackage rec {
            pname = "cargo-px";
            version = "0.2.22";
            #buildFeatures = ["no_downloads"]; # cargo-leptos will try to download Ruby and other things without this feature

            src = inputs.cargo-px-git;

            cargoSha256 ="sha256-+pyeqh0IoZ1JMgbhWxhEJw1MPgG7XeocVrqJoSNjgDA=";

            nativeBuildInputs = [pkgs.pkg-config pkgs.openssl pkgs.git];

            buildInputs = with pkgs;
              [openssl pkg-config git]
              ++ lib.optionals stdenv.isDarwin [
              Security
            ];

            doCheck = false; # integration tests depend on changing cargo config

            meta = with lib; {
            description = "A cargo subcommand that extends cargo's capabilities when it comes to code generation.";
            homepage = "https://github.com/LukeMatthewWalker/cargo-px";
            changelog = "https://github.com/LukeMatthewWalker/cargo-px/blob/v${version}/CHANGELOG.md";
            license = with licenses; [mit];
            maintainers = with maintainers; [benwis];
          };
      };
        in
        {
          
          devShells.default = pkgs.mkShell {

            # Extra inputs can be added here
            nativeBuildInputs = with pkgs; [
              #rustTarget
              rustup
              openssl
              pkg-config
              clang
              tailwindcss
              mold-wrapped
              cargo-px-git
              cargo-pavex_cli-git
            ];
            #RUST_SRC_PATH = "${rustTarget}/lib/rustlib/src/rust/library";
            MOLD_PATH = "${pkgs.mold-wrapped}/bin/mold";

            shellHook = ''
            sed -i -e '/rustflags = \["-C", "link-arg=-fuse-ld=/ s|ld=.*|ld=${pkgs.mold-wrapped}/bin/mold"]|' .cargo/config.toml
            '';
            LIBCLANG_PATH = "${pkgs.llvmPackages.libclang.lib}/lib";
          };
        });
}
