use rustc_version::version;

fn main() {
    println!("cargo:rustc-check-cfg=cfg(rust_has_unsafe_env_setters)");

    let rustc_version = version().expect("Failed to retrieve rustc version");

    // Compare only major.minor.patch, ignoring pre-release tags.
    // This ensures 1.92.0-nightly is treated as >= 1.92.0, since
    // semver considers pre-release versions smaller than the release
    // (e.g., 1.92.0-nightly < 1.92.0), but nightly already has the
    // unsafe env setter requirement.
    let (major, minor, patch) = (
        rustc_version.major,
        rustc_version.minor,
        rustc_version.patch,
    );
    if (major, minor, patch) >= (1, 92, 0) {
        println!("cargo:rustc-cfg=rust_has_unsafe_env_setters");
    }
}
