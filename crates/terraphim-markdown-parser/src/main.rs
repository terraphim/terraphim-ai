use std::io::Read;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut markdown = String::new();
    if let Some(path) = std::env::args().nth(1) {
        markdown = std::fs::read_to_string(path)?;
    } else {
        std::io::stdin().read_to_string(&mut markdown)?;
    }

    let normalized = terraphim_markdown_parser::ensure_terraphim_block_ids(&markdown)?;
    print!("{normalized}");
    Ok(())
}
