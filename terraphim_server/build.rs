use std::{env, fs, path::PathBuf};

macro_rules! p {
    ($($tokens:tt)*) => {
        println!("cargo:warning={}", format!($($tokens)*))
    };
}

fn main() -> std::io::Result<()> {
    let dist_dir = env::var("TERRAPHIM_SERVER_DIST")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("./dist"));

    println!("cargo:rerun-if-env-changed=TERRAPHIM_SERVER_DIST");
    println!("cargo:rerun-if-changed={}", dist_dir.display());

    if !dist_dir.exists() {
        p!(
            "Creating missing web assets directory at {}",
            dist_dir.display()
        );
        fs::create_dir_all(&dist_dir)?;
    }

    let index_path = dist_dir.join("index.html");
    if !index_path.exists() {
        p!(
            "No index.html found in {}, writing placeholder",
            dist_dir.display()
        );
        fs::write(
            &index_path,
            r#"<!DOCTYPE html>
<html>
<body>
<h1>Terraphim Server</h1>
<p>Frontend assets not found. Run <code>scripts/build-frontend-for-server.sh</code> to build the web UI.</p>
</body>
</html>"#,
        )?;
    }

    Ok(())
}
