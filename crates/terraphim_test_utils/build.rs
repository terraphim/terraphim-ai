use rustc_version::version;

fn main() {
    println!("cargo:rustc-check-cfg=cfg(rust_has_unsafe_env_setters)");

    let rustc_version = version().expect("Failed to retrieve rustc version");

    // Rust 2024 edition makes std::env::set_var and remove_var unsafe
    // This affects all Rust versions when using edition 2024
    // Additionally, Rust 1.92+ makes them unsafe regardless of edition
    // See: https://doc.rust-lang.org/std/env/fn.set_var.html
    let (major, minor, patch) = (
        rustc_version.major,
        rustc_version.minor,
        rustc_version.patch,
    );

    // Enable unsafe env setters for:
    // 1. Rust 1.92+ (any edition)
    // 2. Rust 2024 edition (this crate uses edition 2024)
    // Since this crate uses edition 2024, we always need the unsafe blocks
    let is_rust_1_92_or_later = (major, minor, patch) >= (1, 92, 0);
    let is_edition_2024 = true; // This crate uses edition.workspace = true which is 2024

    if is_rust_1_92_or_later || is_edition_2024 {
        println!("cargo:rustc-cfg=rust_has_unsafe_env_setters");
    }
}
