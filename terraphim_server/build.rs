use std::{
    fs::{self, Metadata},
    path::PathBuf,
    time::SystemTime,
};

macro_rules! p {
    ($($tokens: tt)*) => {
        println!("cargo:warning={}", format!($($tokens)*))
    }
}

struct Dirs {
    js_dist_source: PathBuf,
    js_dist_tmp: PathBuf,
    src_browser: PathBuf,
    browser_root: PathBuf,
}

fn main() -> std::io::Result<()> {
    const BROWSER_ROOT: &str = "../desktop/";
    let dirs: Dirs = {
        Dirs {
            js_dist_source: PathBuf::from("../desktop/dist"),
            js_dist_tmp: PathBuf::from("./dist"),
            src_browser: PathBuf::from("../desktop/"),
            browser_root: PathBuf::from(BROWSER_ROOT),
        }
    };
    println!("cargo:rerun-if-changed={}", BROWSER_ROOT);

    // Ensure dist directory exists for RustEmbed even if desktop is not built
    // RustEmbed in lib.rs expects ../desktop/dist, so create placeholder there too
    for dist_dir in [&dirs.js_dist_source, &dirs.js_dist_tmp] {
        if !dist_dir.exists() {
            p!(
                "Creating dist directory for RustEmbed: {}",
                dist_dir.display()
            );
            fs::create_dir_all(dist_dir)?;
            // Create minimal placeholder index.html
            let index_path = dist_dir.join("index.html");
            if !index_path.exists() {
                fs::write(
                    &index_path,
                    "<!DOCTYPE html><html><body>Terraphim Server</body></html>",
                )?;
                p!("Created placeholder index.html at {}", index_path.display());
            }
        }
    }

    if should_build(&dirs) {
        let build_succeeded = build_js(&dirs);
        if build_succeeded && dirs.js_dist_source.exists() {
            let _ = fs::remove_dir_all(&dirs.js_dist_tmp);
            dircpy::copy_dir(&dirs.js_dist_source, &dirs.js_dist_tmp)?;
        } else {
            p!("JS build did not produce dist folder, using existing or placeholder");
        }
    } else if dirs.js_dist_tmp.exists() {
        p!("Found {}, skipping copy", dirs.js_dist_tmp.display());
    } else if dirs.js_dist_source.exists() {
        p!(
            "Could not find {} , copying from {}",
            dirs.js_dist_tmp.display(),
            dirs.js_dist_source.display()
        );
        dircpy::copy_dir(&dirs.js_dist_source, &dirs.js_dist_tmp)?;
    } else {
        p!("Neither source nor tmp dist found, using placeholder");
    }

    // Makes the static files available for compilation
    static_files::resource_dir(&dirs.js_dist_tmp)
        .build()
        .unwrap_or_else(|_e| {
            panic!(
                "failed to open data browser assets from {}",
                dirs.js_dist_tmp.display()
            )
        });

    Ok(())
}

fn should_build(dirs: &Dirs) -> bool {
    if !dirs.browser_root.exists() {
        p!(
            "Could not find browser folder, assuming this is a `cargo publish` run. Skipping JS build."
        );
        return false;
    }

    // On Windows, skip JS build via build.rs - use pre-built assets from CI
    #[cfg(target_os = "windows")]
    {
        if dirs.js_dist_source.exists() {
            p!(
                "Windows detected, using pre-built assets from {}",
                dirs.js_dist_source.display()
            );
            return false;
        }
    }

    // Check if any JS files were modified since the last build
    if let Ok(tmp_dist_index_html) =
        std::fs::metadata(format!("{}/index.html", dirs.js_dist_tmp.display()))
    {
        let has_changes = walkdir::WalkDir::new(&dirs.src_browser)
            .into_iter()
            .filter_entry(|entry| {
                entry
                    .file_name()
                    .to_str()
                    .map(|s| !s.starts_with(".DS_Store") && s != "dist" && s != "node_modules")
                    .unwrap_or(false)
            })
            .any(|entry| is_older_than(&entry.unwrap(), &tmp_dist_index_html));

        if has_changes {
            return true;
        }

        p!("No changes in JS source files, skipping JS build.");
        false
    } else if dirs.src_browser.exists() {
        p!(
            "No JS dist folder found at {}, but did find source folder {}, building...",
            dirs.js_dist_tmp.display(),
            dirs.src_browser.display()
        );
        true
    } else {
        p!(
            "Could not find index.html in {}. Skipping JS build.",
            dirs.js_dist_tmp.display()
        );
        false
    }
}

/// Runs JS package manager to install packages and build the JS bundle
/// Returns true if build succeeded, false if it failed (graceful fallback)
fn build_js(dirs: &Dirs) -> bool {
    let pkg_manager = "./scripts/yarn_and_build.sh";
    p!("install js packages...");
    p!("build js assets...");
    let out = std::process::Command::new("/bin/bash")
        .arg(pkg_manager)
        .current_dir(&dirs.browser_root)
        .output();

    match out {
        Ok(output) if output.status.success() => {
            p!("js build successful");
            true
        }
        Ok(output) => {
            p!(
                "js build failed (gracefully continuing): {}",
                String::from_utf8_lossy(&output.stderr)
            );
            false
        }
        Err(e) => {
            p!("js build command failed (gracefully continuing): {}", e);
            false
        }
    }
}

fn is_older_than(dir_entry: &walkdir::DirEntry, dist_meta: &Metadata) -> bool {
    let dist_time = dist_meta
        .modified()
        .unwrap()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap();

    if dir_entry.path().is_file() {
        let src_time = dir_entry
            .metadata()
            .unwrap()
            .modified()
            .unwrap()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap();
        if src_time >= dist_time {
            p!(
                "Source file modified: {:?}, rebuilding...",
                dir_entry.path()
            );
            return true;
        }
    }
    false
}
