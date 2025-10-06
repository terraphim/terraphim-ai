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
    if !dirs.js_dist_tmp.exists() {
        p!("Creating dist directory for RustEmbed: {}", dirs.js_dist_tmp.display());
        fs::create_dir_all(&dirs.js_dist_tmp)?;
        // Create minimal placeholder index.html if source doesn't exist
        let index_path = dirs.js_dist_tmp.join("index.html");
        if !index_path.exists() && !dirs.js_dist_source.exists() {
            fs::write(index_path, "<!DOCTYPE html><html><body>Terraphim Server</body></html>")?;
            p!("Created placeholder index.html for RustEmbed");
        }
    }

    if should_build(&dirs) {
        build_js(&dirs);
        let _ = fs::remove_dir_all(&dirs.js_dist_tmp);
        dircpy::copy_dir(&dirs.js_dist_source, &dirs.js_dist_tmp)?;
    } else if dirs.js_dist_tmp.exists() {
        p!("Found {}, skipping copy", dirs.js_dist_tmp.display());
    } else {
        if dirs.js_dist_source.exists() {
            p!(
                "Could not find {} , copying from {}",
                dirs.js_dist_tmp.display(),
                dirs.js_dist_source.display()
            );
            dircpy::copy_dir(&dirs.js_dist_source, &dirs.js_dist_tmp)?;
        } else {
            p!("Neither source nor tmp dist found, using placeholder");
        }
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
        p!("Could not find browser folder, assuming this is a `cargo publish` run. Skipping JS build.");
        return false;
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
                    .map(|s| !s.starts_with(".DS_Store"))
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
fn build_js(dirs: &Dirs) {
    let pkg_manager = "./scripts/yarn_and_build.sh";
    p!("install js packages...");
    p!("build js assets...");
    let out = std::process::Command::new("/bin/bash")
        .arg(pkg_manager)
        .current_dir(&dirs.browser_root)
        .output()
        .expect("Failed to build js bundle");
    // Check if out contains errors
    if out.status.success() {
        p!("js build successful");
    } else {
        panic!(
            "js build failed: {}",
            String::from_utf8(out.stderr).unwrap()
        );
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
