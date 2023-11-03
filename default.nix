{}:
let
  rev = "c082856b850ec60cda9f0a0db2bc7bd8900d708c";
  nixpkgs = fetchTarball "https://github.com/NixOS/nixpkgs/archive/${rev}.tar.gz";
  pkgs = import nixpkgs { };
in
pkgs.mkShell {
  buildInputs = with pkgs; [
    cacert
    cargo-make
    nodejs_20
    openssl
    pkg-config
    rustup
    trunk
  ];
}
