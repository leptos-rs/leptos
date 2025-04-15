use rustc_version::{version_meta, Channel};

fn main() {
    let target = std::env::var("TARGET").unwrap_or_default();

    // Set cfg flags depending on release channel
    if matches!(version_meta().unwrap().channel, Channel::Nightly) {
        println!("cargo:rustc-cfg=rustc_nightly");
    }
    // Set cfg flag for getrandom wasm_js
    if target == "wasm32-unknown-unknown" {
        // Set a custom cfg flag for wasm builds
        println!("cargo:rustc-cfg=getrandom_backend=\"wasm_js\"");
    }
}
