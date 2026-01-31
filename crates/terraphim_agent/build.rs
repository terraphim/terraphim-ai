use rustc_version::{Version, version};

fn main() {
    println!("cargo:rustc-check-cfg=cfg(rust_has_unsafe_env_setters)");

    let rustc_version = version().expect("Failed to retrieve rustc version");

    if rustc_version >= Version::new(1, 92, 0) {
        println!("cargo:rustc-cfg=rust_has_unsafe_env_setters");
    }
}
