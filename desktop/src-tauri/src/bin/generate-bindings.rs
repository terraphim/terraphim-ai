use terraphim_ai_desktop::generate_typescript_bindings;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔧 Generating TypeScript bindings from Rust types...");
    generate_typescript_bindings()?;
    println!("✅ Done! Check desktop/src/lib/generated/types.ts");
    Ok(())
} 