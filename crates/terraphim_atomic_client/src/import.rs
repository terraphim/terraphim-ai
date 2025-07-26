use std::collections::HashMap;
use crate::{Config, Store};
use serde_json::Value;

/// Import ontology from JSON file
/// 
/// # Arguments
/// * `json_file` - Path to JSON-AD file to import
/// * `parent_url` - Optional parent URL (defaults to Drive)
/// * `validate` - Enable validation of resources
#[cfg(feature = "native")]
pub async fn import_ontology_from_file(
    json_file: &str, 
    parent_url: Option<&str>, 
    validate: bool
) -> Result<(), Box<dyn std::error::Error>> {
    // Load configuration and initialize store
    let config = Config::from_env()?;
    let store = Store::new(config)?;
    
    // Read and parse JSON file
    let json_content = std::fs::read_to_string(json_file)?;
    let resources: Vec<serde_json::Map<String, Value>> = serde_json::from_str(&json_content)?;

    // Determine parent URL or use default
    let parent_url = parent_url.unwrap_or("https://atomicdata.dev/classes/Drive");
    
    println!("Importing {} resources from {} to parent {}", resources.len(), json_file, parent_url);
    
    // Sort resources by dependencies (ontology first, then classes, then properties)
    let sorted_resources = sort_resources_by_dependencies(resources, parent_url)?;

    let mut success_count = 0;
    let mut failed_resources = Vec::new();

    // Import resources in dependency order
    for (index, resource_obj) in sorted_resources.iter().enumerate() {
        match import_single_resource(&store, resource_obj, parent_url, validate).await {
            Ok(subject) => {
                println!("✓ Successfully imported resource {}/{}: {}", index + 1, sorted_resources.len(), subject);
                success_count += 1;
            }
            Err(e) => {
                println!("✗ Failed to import resource {}/{}: {}", index + 1, sorted_resources.len(), e);
                failed_resources.push((index + 1, e.to_string()));
            }
        }
    }

    // Print summary
    println!("\n=== Import Summary ===");
    println!("Successfully imported: {} resources", success_count);
    println!("Failed imports: {} resources", failed_resources.len());

    if !failed_resources.is_empty() {
        println!("\nErrors:");
        for (index, error) in failed_resources {
            println!("  ✗ Failed to import resource {}: {}", index, error);
        }
        return Err("Some resources failed to import".into());
    } else {
        println!("✓ All resources imported successfully!");
    }
    
    Ok(())
}

#[cfg(feature = "native")]
fn sort_resources_by_dependencies(
    resources: Vec<serde_json::Map<String, serde_json::Value>>,
    _parent_url: &str
) -> Result<Vec<serde_json::Map<String, serde_json::Value>>, Box<dyn std::error::Error>> {
    // Define dependency order: ontology -> classes -> properties
    let mut sorted = Vec::new();
    let mut remaining = resources;

    // First, find and add the ontology resource
    let ontology_index = remaining.iter().position(|r| {
        r.get("https://atomicdata.dev/properties/isA")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().any(|item| item.as_str() == Some("https://atomicdata.dev/class/ontology")))
            .unwrap_or(false)
    });

    if let Some(index) = ontology_index {
        sorted.push(remaining.remove(index));
    }

    // Then add all class resources
    let class_indices: Vec<usize> = remaining.iter()
        .enumerate()
        .filter(|(_, r)| {
            r.get("https://atomicdata.dev/properties/isA")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().any(|item| item.as_str() == Some("https://atomicdata.dev/classes/Class")))
                .unwrap_or(false)
        })
        .map(|(i, _)| i)
        .collect();

    // Add classes in reverse order to maintain original order
    for &index in class_indices.iter().rev() {
        sorted.push(remaining.remove(index));
    }

    // Finally add all property resources
    let property_indices: Vec<usize> = remaining.iter()
        .enumerate()
        .filter(|(_, r)| {
            r.get("https://atomicdata.dev/properties/isA")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().any(|item| item.as_str() == Some("https://atomicdata.dev/classes/Property")))
                .unwrap_or(false)
        })
        .map(|(i, _)| i)
        .collect();

    // Add properties in reverse order to maintain original order
    for &index in property_indices.iter().rev() {
        sorted.push(remaining.remove(index));
    }

    // Add any remaining resources
    sorted.extend(remaining);

    Ok(sorted)
}

#[cfg(feature = "native")]
async fn import_single_resource(
    store: &Store,
    resource_obj: &serde_json::Map<String, serde_json::Value>,
    parent_url: &str,
    validate: bool,
) -> Result<String, Box<dyn std::error::Error>> {
    // Extract or generate subject
    let subject = if let Some(id_value) = resource_obj.get("@id") {
        id_value.as_str()
            .ok_or("@id field must be a string")?
            .to_string()
    } else if let Some(local_id) = resource_obj.get("https://atomicdata.dev/properties/localId") {
        // Convert localId to full URL using parent context
        let local_id_str = local_id.as_str()
            .ok_or("localId field must be a string")?;
        format!("{}/{}", parent_url.trim_end_matches('/'), local_id_str)
    } else {
        // Generate a new subject URL based on parent and shortname
        let shortname = resource_obj.get("https://atomicdata.dev/properties/shortname")
            .and_then(|v| v.as_str())
            .ok_or("Resource must have either @id, localId, or shortname property")?;
        
        format!("{}/{}", parent_url.trim_end_matches('/'), shortname)
    };

    // Convert JSON-AD object to properties HashMap
    let mut properties = HashMap::new();

    // Add parent relationship if not already specified
    if !resource_obj.contains_key("https://atomicdata.dev/properties/parent") {
        properties.insert(
            "https://atomicdata.dev/properties/parent".to_string(),
            serde_json::json!(parent_url),
        );
    }

    // Process all properties from the JSON-AD object
    for (key, value) in resource_obj {
        if key == "@id" {
            // Skip @id as it's handled separately
            continue;
        }
        
        // Handle localId conversion
        if key == "https://atomicdata.dev/properties/localId" {
            // Convert localId to full URL and add as @id
            if let Some(local_id_str) = value.as_str() {
                let full_url = format!("{}/{}", parent_url.trim_end_matches('/'), local_id_str);
                properties.insert("@id".to_string(), serde_json::json!(full_url));
            }
            continue;
        }
        
        // Convert relative URLs in arrays to absolute URLs
        let processed_value = if key == "https://atomicdata.dev/properties/classes" || 
                                  key == "https://atomicdata.dev/properties/properties" {
            convert_relative_urls_in_array(value, parent_url)?
        } else if key == "https://atomicdata.dev/properties/parent" {
            // Parent should be a localId reference, not a URL
            // Keep it as is - it will be resolved by the atomic server
            value.clone()
        } else {
            value.clone()
        };
        
        properties.insert(key.clone(), processed_value);
    }

    // Validate resource if requested
    if validate {
        validate_resource(&properties)?;
    }

    // Create the resource using commit
    store.create_with_commit(&subject, properties).await
        .map_err(|e| format!("Failed to create resource {}: {}", subject, e))?;

    Ok(subject)
}

#[cfg(feature = "native")]
fn convert_relative_urls_in_array(
    value: &serde_json::Value, 
    parent_url: &str
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    match value {
        serde_json::Value::Array(arr) => {
            let mut converted = Vec::new();
            for item in arr {
                if let Some(url_str) = item.as_str() {
                    if url_str.starts_with("http://") || url_str.starts_with("https://") {
                        // Already absolute URL
                        converted.push(item.clone());
                    } else {
                        // Convert relative URL to absolute
                        let absolute_url = format!("{}/{}", parent_url.trim_end_matches('/'), url_str);
                        converted.push(serde_json::json!(absolute_url));
                    }
                } else {
                    converted.push(item.clone());
                }
            }
            Ok(serde_json::Value::Array(converted))
        }
        _ => Ok(value.clone())
    }
}

#[cfg(feature = "native")]
fn validate_resource(properties: &HashMap<String, serde_json::Value>) -> Result<(), Box<dyn std::error::Error>> {
    // Basic validation of atomic data properties
    
    // Check for required name, shortname, or localId
    let has_name = properties.contains_key("https://atomicdata.dev/properties/name");
    let has_shortname = properties.contains_key("https://atomicdata.dev/properties/shortname");
    let has_local_id = properties.contains_key("https://atomicdata.dev/properties/localId");
    
    if !has_name && !has_shortname && !has_local_id {
        return Err("Resource must have either 'name', 'shortname', or 'localId' property".into());
    }

    // Validate isA property
    if let Some(is_a_value) = properties.get("https://atomicdata.dev/properties/isA") {
        match is_a_value {
            serde_json::Value::Array(classes) => {
                for class in classes {
                    if let Some(class_str) = class.as_str() {
                        if !class_str.starts_with("https://") && !class_str.starts_with("http://") {
                            return Err(format!("Invalid class URL in isA: {}", class_str).into());
                        }
                    } else {
                        return Err("All items in isA array must be strings".into());
                    }
                }
            }
            serde_json::Value::String(class_str) => {
                if !class_str.starts_with("https://") && !class_str.starts_with("http://") {
                    return Err(format!("Invalid class URL in isA: {}", class_str).into());
                }
            }
            _ => {
                return Err("isA property must be a string or array of strings".into());
            }
        }
    }

    Ok(())
}
