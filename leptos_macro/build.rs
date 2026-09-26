use rustc_version::{Channel, version_meta};

fn main() {
    // Set cfg flags depending on release channel
    if matches!(version_meta().unwrap().channel, Channel::Nightly) {
        println!("cargo:rustc-cfg=rustc_nightly");
    }
}
