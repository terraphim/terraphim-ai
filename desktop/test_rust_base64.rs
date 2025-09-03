use base64::{engine::general_purpose::STANDARD, Engine};

fn main() {
    let private_key = "RygvlKGUDG9/loCY5KCHrQDrnJEG7P7P9HKb+BE8NS0=";
    println!("Testing private key: {}", private_key);
    println!("Length: {}", private_key.len());
    println!("Length % 4: {}", private_key.len() % 4);

    match STANDARD.decode(private_key) {
        Ok(bytes) => {
            println!("✅ Base64 decode successful, {} bytes", bytes.len());
        },
        Err(e) => {
            println!("❌ Base64 decode failed: {}", e);

            // Try with padding
            let padded_key = if private_key.len() % 4 != 0 {
                let padding_needed = 4 - (private_key.len() % 4);
                format!("{}{}", private_key, "=".repeat(padding_needed))
            } else {
                private_key.to_string()
            };

            println!("Trying with padding: {}", padded_key);
            match STANDARD.decode(&padded_key) {
                Ok(bytes) => println!("✅ Base64 decode successful with padding, {} bytes", bytes.len()),
                Err(e2) => println!("❌ Base64 decode still failed with padding: {}", e2),
            }
        }
    }
}
