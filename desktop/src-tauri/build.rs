use std::env;
use std::fs;
use std::path::Path;

fn main() {
    // Copy configuration files to the build directory
    copy_configs();

    tauri_build::build()
}

fn copy_configs() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();

    // Source and destination paths
    let source_default_dir = Path::new(&manifest_dir).join("../default");
    let dest_configs_dir = Path::new(&out_dir).join("configs");

    // Create destination directory
    if let Err(e) = fs::create_dir_all(&dest_configs_dir) {
        println!("cargo:warning=Failed to create configs directory: {}", e);
        return;
    }

    // Copy configuration files
    if source_default_dir.exists() {
        if let Err(e) = copy_dir_all(&source_default_dir, &dest_configs_dir) {
            println!("cargo:warning=Failed to copy configs: {}", e);
        } else {
            println!("cargo:rerun-if-changed=../default");
            println!("cargo:warning=Successfully copied config files to build");
        }
    } else {
        println!(
            "cargo:warning=Source config directory doesn't exist: {:?}",
            source_default_dir
        );
    }

    // Copy KG files
    let kg_source_dir = Path::new(&manifest_dir).join("../../../docs/src/kg");
    let kg_dest_dir = Path::new(&out_dir).join("kg");

    if kg_source_dir.exists() {
        if let Err(e) = fs::create_dir_all(&kg_dest_dir) {
            println!("cargo:warning=Failed to create KG directory: {}", e);
        } else if let Err(e) = copy_dir_all(&kg_source_dir, &kg_dest_dir) {
            println!("cargo:warning=Failed to copy KG files: {}", e);
        } else {
            println!("cargo:rerun-if-changed=../../../docs/src/kg");
            println!("cargo:warning=Successfully copied KG files to build");
        }
    }
}

fn copy_dir_all(src: &Path, dst: &Path) -> std::io::Result<()> {
    if !src.is_dir() {
        return Ok(());
    }

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if ty.is_dir() {
            fs::create_dir_all(&dst_path)?;
            copy_dir_all(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}
