use tempfile::NamedTempFile;

fn main() {
    let temp_file = NamedTempFile::new().unwrap();
    let url = format!("file://{}", temp_file.path().display());
    println!("URL: {}", url);

    // Try to understand what ureq does with this
    match ureq::get(&url).call() {
        Ok(_) => println!("Success"),
        Err(e) => println!("Error: {:?}", e),
    }
}
