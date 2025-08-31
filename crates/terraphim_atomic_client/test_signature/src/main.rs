use base64::{engine::general_purpose::STANDARD, Engine};
use ed25519_dalek::{Keypair, Signer};
use serde_json::{json, Value};
use std::env;
use std::time::{SystemTime, UNIX_EPOCH};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse command-line arguments
    let args: Vec<String> = env::args().collect();

    // Check operation type
    let operation = if args.len() > 1 { &args[1] } else { "create" };

    match operation {
        "create" => create_resource(&args)?,
        "update" => update_resource(&args)?,
        "delete" => delete_resource(&args)?,
        _ => {
            if !atty::is(atty::Stream::Stdout) {
                // If output is redirected, don't print error message
                return Ok(());
            }
            println!("Unknown operation: {}", operation);
            println!("Usage:");
            println!("  create <shortname> <description> <class> [name]");
            println!("  update <resource_url> <property> <value>");
            println!("  delete <resource_url>");
        }
    }

    Ok(())
}

fn create_resource(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    // Default values
    let shortname = if args.len() > 2 { &args[2] } else {
        return Err("Shortname is required".into());
    };

    let description = if args.len() > 3 { &args[3] } else {
        return Err("Description is required".into());
    };

    let class = if args.len() > 4 { &args[4] } else {
        return Err("Class is required".into());
    };

    let name = if args.len() > 5 { &args[5] } else { shortname };

    // Load agent data from environment
    let secret_base64 = std::env::var("ATOMIC_SERVER_SECRET").expect("ATOMIC_SERVER_SECRET not set");
    let server_url = std::env::var("ATOMIC_SERVER_URL").unwrap_or_else(|_| "http://localhost:9883".to_string());

    // Check if output is redirected
    let is_redirected = !atty::is(atty::Stream::Stdout);

    // Parse the agent secret
    let secret_bytes = STANDARD.decode(&secret_base64)?;
    let secret_json = String::from_utf8(secret_bytes)?;
    let secret: Value = serde_json::from_str(&secret_json)?;

    // Extract agent data
    let private_key_b64 = secret["privateKey"].as_str().expect("Missing privateKey in secret");
    let public_key_b64 = secret["publicKey"].as_str().expect("Missing publicKey in secret");
    let agent_subject = secret["subject"].as_str().expect("Missing subject in secret");

    if !is_redirected {
        println!("Agent subject: {}", agent_subject);
        println!("Public key: {}", public_key_b64);
    }

    // Decode the keys
    let private_key_bytes = STANDARD.decode(private_key_b64)?;
    let public_key_bytes = STANDARD.decode(public_key_b64)?;

    // Create a keypair
    let mut keypair_bytes = [0u8; 64];
    keypair_bytes[..32].copy_from_slice(&private_key_bytes);
    keypair_bytes[32..].copy_from_slice(&public_key_bytes);
    let keypair = Keypair::from_bytes(&keypair_bytes)?;

    // Create a timestamp for a unique resource ID
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)?
        .as_secs();
    let subject = format!("{}/{}", server_url, shortname);

    if !is_redirected {
        println!("Resource subject: {}", subject);
        println!("Resource class: {}", class);
        println!("Resource description: {}", description);
        println!("Resource name: {}", name);
    }

    // Create a commit object without signature
    let commit = json!({
        "https://atomicdata.dev/properties/subject": subject,
        "https://atomicdata.dev/properties/createdAt": timestamp,
        "https://atomicdata.dev/properties/signer": agent_subject,
        "https://atomicdata.dev/properties/isA": ["https://atomicdata.dev/classes/Commit"],
        "https://atomicdata.dev/properties/set": {
            "https://atomicdata.dev/properties/shortname": shortname,
            "https://atomicdata.dev/properties/description": description,
            "https://atomicdata.dev/properties/parent": server_url,
            "https://atomicdata.dev/properties/name": name,
            "https://atomicdata.dev/properties/isA": [format!("https://atomicdata.dev/classes/{}", class)]
        }
    });

    // Serialize the commit to a deterministic JSON string
    let commit_json = serde_jcs::to_string(&commit)?;

    if !is_redirected {
        println!("String to sign: {}", commit_json);
    }

    // Sign the commit
    let signature = keypair.sign(commit_json.as_bytes());
    let signature_b64 = STANDARD.encode(signature.to_bytes());

    if !is_redirected {
        println!("Signature: {}", signature_b64);
    }

    // Create the final commit JSON with signature
    let mut commit_with_sig = commit.clone();
    commit_with_sig["https://atomicdata.dev/properties/signature"] = json!(signature_b64);

    // Print the final commit JSON
    if is_redirected {
        // Just output the JSON when redirected
        print!("{}", serde_json::to_string(&commit_with_sig)?);
    } else {
        println!("\nFinal commit JSON:");
        println!("{}", serde_json::to_string_pretty(&commit_with_sig)?);

        // Print curl command to send the commit
        println!("\nTo send this commit to the server, run:");
        println!("curl -X POST -H \"Content-Type: application/json\" -d '{}' {}/commit",
            serde_json::to_string(&commit_with_sig)?, server_url);
    }

    Ok(())
}

fn update_resource(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    // Parse arguments
    let resource_url = if args.len() > 2 { &args[2] } else {
        return Err("Resource URL is required".into());
    };

    let property = if args.len() > 3 { &args[3] } else {
        return Err("Property is required".into());
    };

    let value = if args.len() > 4 { &args[4] } else {
        return Err("Value is required".into());
    };

    // Load agent data from environment
    let secret_base64 = std::env::var("ATOMIC_SERVER_SECRET").expect("ATOMIC_SERVER_SECRET not set");
    let server_url = std::env::var("ATOMIC_SERVER_URL").unwrap_or_else(|_| "http://localhost:9883".to_string());

    // Check if output is redirected
    let is_redirected = !atty::is(atty::Stream::Stdout);

    // Parse the agent secret
    let secret_bytes = STANDARD.decode(&secret_base64)?;
    let secret_json = String::from_utf8(secret_bytes)?;
    let secret: Value = serde_json::from_str(&secret_json)?;

    // Extract agent data
    let private_key_b64 = secret["privateKey"].as_str().expect("Missing privateKey in secret");
    let public_key_b64 = secret["publicKey"].as_str().expect("Missing publicKey in secret");
    let agent_subject = secret["subject"].as_str().expect("Missing subject in secret");

    if !is_redirected {
        println!("Agent subject: {}", agent_subject);
        println!("Public key: {}", public_key_b64);
        println!("Updating resource: {}", resource_url);
        println!("Setting {} to {}", property, value);
    }

    // Decode the keys
    let private_key_bytes = STANDARD.decode(private_key_b64)?;
    let public_key_bytes = STANDARD.decode(public_key_b64)?;

    // Create a keypair
    let mut keypair_bytes = [0u8; 64];
    keypair_bytes[..32].copy_from_slice(&private_key_bytes);
    keypair_bytes[32..].copy_from_slice(&public_key_bytes);
    let keypair = Keypair::from_bytes(&keypair_bytes)?;

    // Create a timestamp
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)?
        .as_secs();

    // Create a commit object without signature
    let mut set_properties = serde_json::Map::new();
    set_properties.insert(property.to_string(), json!(value));

    let commit = json!({
        "https://atomicdata.dev/properties/subject": resource_url,
        "https://atomicdata.dev/properties/createdAt": timestamp,
        "https://atomicdata.dev/properties/signer": agent_subject,
        "https://atomicdata.dev/properties/isA": ["https://atomicdata.dev/classes/Commit"],
        "https://atomicdata.dev/properties/set": set_properties
    });

    // Serialize the commit to a deterministic JSON string
    let commit_json = serde_jcs::to_string(&commit)?;

    if !is_redirected {
        println!("String to sign: {}", commit_json);
    }

    // Sign the commit
    let signature = keypair.sign(commit_json.as_bytes());
    let signature_b64 = STANDARD.encode(signature.to_bytes());

    if !is_redirected {
        println!("Signature: {}", signature_b64);
    }

    // Create the final commit JSON with signature
    let mut commit_with_sig = commit.clone();
    commit_with_sig["https://atomicdata.dev/properties/signature"] = json!(signature_b64);

    // Print the final commit JSON
    if is_redirected {
        // Just output the JSON when redirected
        print!("{}", serde_json::to_string(&commit_with_sig)?);
    } else {
        println!("\nFinal commit JSON:");
        println!("{}", serde_json::to_string_pretty(&commit_with_sig)?);

        // Print curl command to send the commit
        println!("\nTo send this commit to the server, run:");
        println!("curl -X POST -H \"Content-Type: application/json\" -d '{}' {}/commit",
            serde_json::to_string(&commit_with_sig)?, server_url);
    }

    Ok(())
}

fn delete_resource(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    // Parse arguments
    let resource_url = if args.len() > 2 { &args[2] } else {
        return Err("Resource URL is required".into());
    };

    // Load agent data from environment
    let secret_base64 = std::env::var("ATOMIC_SERVER_SECRET").expect("ATOMIC_SERVER_SECRET not set");
    let server_url = std::env::var("ATOMIC_SERVER_URL").unwrap_or_else(|_| "http://localhost:9883".to_string());

    // Check if output is redirected
    let is_redirected = !atty::is(atty::Stream::Stdout);

    // Parse the agent secret
    let secret_bytes = STANDARD.decode(&secret_base64)?;
    let secret_json = String::from_utf8(secret_bytes)?;
    let secret: Value = serde_json::from_str(&secret_json)?;

    // Extract agent data
    let private_key_b64 = secret["privateKey"].as_str().expect("Missing privateKey in secret");
    let public_key_b64 = secret["publicKey"].as_str().expect("Missing publicKey in secret");
    let agent_subject = secret["subject"].as_str().expect("Missing subject in secret");

    if !is_redirected {
        println!("Agent subject: {}", agent_subject);
        println!("Public key: {}", public_key_b64);
        println!("Deleting resource: {}", resource_url);
    }

    // Decode the keys
    let private_key_bytes = STANDARD.decode(private_key_b64)?;
    let public_key_bytes = STANDARD.decode(public_key_b64)?;

    // Create a keypair
    let mut keypair_bytes = [0u8; 64];
    keypair_bytes[..32].copy_from_slice(&private_key_bytes);
    keypair_bytes[32..].copy_from_slice(&public_key_bytes);
    let keypair = Keypair::from_bytes(&keypair_bytes)?;

    // Create a timestamp
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)?
        .as_secs();

    // Create a commit object without signature
    let commit = json!({
        "https://atomicdata.dev/properties/subject": resource_url,
        "https://atomicdata.dev/properties/createdAt": timestamp,
        "https://atomicdata.dev/properties/signer": agent_subject,
        "https://atomicdata.dev/properties/isA": ["https://atomicdata.dev/classes/Commit"],
        "https://atomicdata.dev/properties/destroy": true
    });

    // Serialize the commit to a deterministic JSON string
    let commit_json = serde_jcs::to_string(&commit)?;

    if !is_redirected {
        println!("String to sign: {}", commit_json);
    }

    // Sign the commit
    let signature = keypair.sign(commit_json.as_bytes());
    let signature_b64 = STANDARD.encode(signature.to_bytes());

    if !is_redirected {
        println!("Signature: {}", signature_b64);
    }

    // Create the final commit JSON with signature
    let mut commit_with_sig = commit.clone();
    commit_with_sig["https://atomicdata.dev/properties/signature"] = json!(signature_b64);

    // Print the final commit JSON
    if is_redirected {
        // Just output the JSON when redirected
        print!("{}", serde_json::to_string(&commit_with_sig)?);
    } else {
        println!("\nFinal commit JSON:");
        println!("{}", serde_json::to_string_pretty(&commit_with_sig)?);

        // Print curl command to send the commit
        println!("\nTo send this commit to the server, run:");
        println!("curl -X POST -H \"Content-Type: application/json\" -d '{}' {}/commit",
            serde_json::to_string(&commit_with_sig)?, server_url);
    }

    Ok(())
}
