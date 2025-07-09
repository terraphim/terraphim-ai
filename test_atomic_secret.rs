use base64::{engine::general_purpose::STANDARD, Engine};
use serde_json::Value;

fn main() {
    let secret = std::env::var("ATOMIC_SERVER_SECRET").expect("ATOMIC_SERVER_SECRET not set");
    println!("Secret length: {}", secret.len());
    println!("Secret (first 50 chars): {}", &secret[..50.min(secret.len())]);
    
    // Test base64 decoding
    match STANDARD.decode(&secret) {
        Ok(bytes) => {
            println!("✅ Base64 decode successful, {} bytes", bytes.len());
            
            // Try to parse as JSON
            match String::from_utf8(bytes) {
                Ok(json_str) => {
                    println!("✅ UTF-8 decode successful");
                    match serde_json::from_str::<Value>(&json_str) {
                        Ok(json) => {
                            println!("✅ JSON parse successful");
                            println!("JSON keys: {:?}", json.as_object().map(|obj| obj.keys().collect::<Vec<_>>()));
                            
                            // Check for required fields
                            if let Some(private_key) = json.get("privateKey") {
                                println!("✅ Found privateKey");
                                if let Some(private_key_str) = private_key.as_str() {
                                    match STANDARD.decode(private_key_str) {
                                        Ok(_) => println!("✅ privateKey base64 decode successful"),
                                        Err(e) => println!("❌ privateKey base64 decode failed: {}", e),
                                    }
                                }
                            } else {
                                println!("❌ Missing privateKey");
                            }
                            
                            if let Some(subject) = json.get("subject") {
                                println!("✅ Found subject: {}", subject);
                            } else {
                                println!("❌ Missing subject");
                            }
                        },
                        Err(e) => println!("❌ JSON parse failed: {}", e),
                    }
                },
                Err(e) => println!("❌ UTF-8 decode failed: {}", e),
            }
        },
        Err(e) => {
            println!("❌ Base64 decode failed: {}", e);
            
            // Try with padding
            let padded_secret = if secret.len() % 4 != 0 {
                let padding_needed = 4 - (secret.len() % 4);
                format!("{}{}", secret, "=".repeat(padding_needed))
            } else {
                secret.clone()
            };
            
            println!("Trying with padding: {}", padded_secret.len());
            match STANDARD.decode(&padded_secret) {
                Ok(_) => println!("✅ Base64 decode successful with padding"),
                Err(e2) => println!("❌ Base64 decode still failed with padding: {}", e2),
            }
        }
    }
} 