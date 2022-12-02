use rustc_version::{version, version_meta, Channel};

fn main() {
    assert!(version().unwrap().major >= 1);

    match version_meta().unwrap().channel {
        Channel::Stable => {
            println!("cargo:rustc-cfg=feature=\"stable\"")
        }
		_ => {}
    }
}