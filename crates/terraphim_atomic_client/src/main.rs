use serde_json::json;
use std::collections::HashMap;
use std::env;
use terraphim_atomic_client::{Config, Store};

#[cfg(feature = "native")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse command-line arguments
    let args: Vec<String> = env::args().collect();

    // Check operation type
    let operation = if args.len() > 1 { &args[1] } else { "help" };

    match operation {
        "create" => create_resource(&args).await?,
        "update" => update_resource(&args).await?,
        "delete" => delete_resource(&args).await?,
        "search" => search_resources(&args).await?,
        "get" => get_resource(&args).await?,
        "export" => export_resources(&args).await?,
        "export-ontology" => export_ontology(&args).await?,
        "collection" => collection_query(&args).await?,
        "export-to-local" => export_to_local(&args).await?,
        "import-ontology" => import_ontology(&args).await?,
        _ => {
            println!("Usage:");
            println!("  terraphim_atomic_client create <shortname> <name> <description> <class>");
            println!("  terraphim_atomic_client update <resource_url> <property> <value>");
            println!("  terraphim_atomic_client delete <resource_url>");
            println!("  terraphim_atomic_client search <query>");
            println!("  terraphim_atomic_client get <resource_url>");
            println!(
                "  terraphim_atomic_client export <subject_url> [output_file] [format] [--validate]"
            );
            println!(
                "  terraphim_atomic_client export-ontology <ontology_subject> [output_file] [format] [--validate]"
            );
            println!(
                "  terraphim_atomic_client collection <class_url> <sort_property_url> [--desc] [--limit N]"
            );
            println!(
                "  terraphim_atomic_client export-to-local <root_subject> [output_file] [format] [--validate]"
            );
            println!(
                "  terraphim_atomic_client import-ontology <json_file> [parent_url] [--validate]"
            );
        }
    }

    Ok(())
}

#[cfg(not(feature = "native"))]
fn main() {
    println!("This binary requires the 'native' feature to be enabled.");
    std::process::exit(1);
}

#[cfg(feature = "native")]
async fn get_resource(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    // Check if we have enough arguments
    if args.len() < 3 {
        println!("Usage: terraphim_atomic_client get <resource_url>");
        return Ok(());
    }

    let resource_url = &args[2];

    // Load configuration from environment
    let config = Config::from_env()?;
    let store = Store::new(config)?;

    // Get the resource
    let resource = store.get_resource(resource_url).await?;
    println!("Resource: {:#?}", resource);

    Ok(())
}

#[cfg(feature = "native")]
async fn create_resource(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    // Check if we have enough arguments
    if args.len() < 6 {
        println!("Usage: terraphim_atomic_client create <shortname> <name> <description> <class>");
        return Ok(());
    }

    let shortname = &args[2];
    let name = &args[3];
    let description = &args[4];
    let class = &args[5];

    // Load configuration from environment
    let config = Config::from_env()?;
    let store = Store::new(config)?;

    // Create a unique resource ID with proper URL formatting
    let server_url = store.config.server_url.trim_end_matches('/');
    let subject = format!("{}/{}", server_url, shortname);
    println!("Creating resource with ID: {}", subject);

    // Create properties for the resource
    let mut properties = HashMap::new();
    properties.insert(
        "https://atomicdata.dev/properties/shortname".to_string(),
        json!(shortname),
    );
    properties.insert(
        "https://atomicdata.dev/properties/name".to_string(),
        json!(name),
    );
    properties.insert(
        "https://atomicdata.dev/properties/description".to_string(),
        json!(description),
    );
    properties.insert(
        "https://atomicdata.dev/properties/parent".to_string(),
        json!(server_url),
    );
    properties.insert(
        "https://atomicdata.dev/properties/isA".to_string(),
        json!([format!("https://atomicdata.dev/classes/{}", class)]),
    );

    // Create the resource using a commit
    let result = store.create_with_commit(&subject, properties).await?;
    println!("Resource created successfully: {:#?}", result);

    Ok(())
}

#[cfg(feature = "native")]
async fn update_resource(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    // Check if we have enough arguments
    if args.len() < 5 {
        println!("Usage: terraphim_atomic_client update <resource_url> <property> <value>");
        return Ok(());
    }

    let resource_url = &args[2];
    let property = &args[3];
    let value = &args[4];

    // Load configuration from environment
    let config = Config::from_env()?;
    let store = Store::new(config)?;

    // Create properties for the update
    let mut properties = HashMap::new();
    properties.insert(property.to_string(), json!(value));

    // Update the resource using a commit
    let result = store.update_with_commit(resource_url, properties).await?;
    println!("Resource updated successfully: {:#?}", result);

    Ok(())
}

#[cfg(feature = "native")]
async fn delete_resource(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    // Check if we have enough arguments
    if args.len() < 3 {
        println!("Usage: terraphim_atomic_client delete <resource_url>");
        return Ok(());
    }

    let resource_url = &args[2];

    // Load configuration from environment
    let config = Config::from_env()?;
    let store = Store::new(config)?;

    // Delete the resource using a commit
    let result = store.delete_with_commit(resource_url).await?;
    println!("Resource deleted successfully: {:#?}", result);

    Ok(())
}

#[cfg(feature = "native")]
async fn search_resources(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    // Check if we have enough arguments
    if args.len() < 3 {
        println!("Usage: terraphim_atomic_client search <query>");
        return Ok(());
    }

    let query = &args[2];

    // Load configuration from environment
    let config = Config::from_env()?;
    let store = Store::new(config)?;

    // Search for resources
    let results = store.search(query).await?;
    println!("Search results: {:#?}", results);

    Ok(())
}

#[cfg(feature = "native")]
async fn export_resources(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    // Usage: terraphim_atomic_client export <subject_url> [output_file] [format]
    if args.len() < 3 {
        println!(
            "Usage: terraphim_atomic_client export <subject_url> [output_file] [format] [--validate]"
        );
        return Ok(());
    }

    let subject = &args[2];
    let output_file = if args.len() > 3 {
        &args[3]
    } else {
        "export.json"
    };
    let format = if args.len() > 4 {
        args[4].to_lowercase()
    } else {
        "json-ad".to_string()
    };

    // optional validation flag
    let validate = args.iter().any(|a| a == "--validate");

    // Load configuration from environment
    let config = Config::from_env()?;
    let store = Store::new(config.clone())?;

    // Fetch all resources recursively
    let resources = fetch_all_subresources(&store, subject).await?;

    use std::fs::File;
    use std::io::Write;

    let mut file = File::create(output_file)?;

    match format.as_str() {
        "json" | "json-ad" => {
            let serialized = serde_json::to_vec_pretty(&resources)?;
            file.write_all(&serialized)?;
        }
        "turtle" => {
            // Serialize each resource individually using Turtle representation from server
            let client = reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .user_agent("Terraphim-Atomic-Client/1.0")
                .build()
                .unwrap_or_else(|_| reqwest::Client::new());
            use reqwest::header::ACCEPT;
            for res in &resources {
                let resp = client
                    .get(&res.subject)
                    .header(ACCEPT, "text/turtle")
                    .send()
                    .await?;
                if !resp.status().is_success() {
                    return Err(format!(
                        "Failed to fetch Turtle serialization for {}: {}",
                        res.subject,
                        resp.status()
                    )
                    .into());
                }
                let body = resp.text().await?;
                writeln!(file, "{}", body)?;
            }
        }
        other => {
            println!(
                "Unsupported format '{}'. Supported: json, json-ad, turtle",
                other
            );
            return Ok(());
        }
    }

    println!(
        "Saved {} resources to {} in {} format",
        resources.len(),
        output_file,
        format
    );

    if validate && format == "json-ad" {
        let server_prefix = store.config.server_url.trim_end_matches('/');
        let encoded_parent = urlencoding::encode(subject);
        let mut import_url = format!(
            "{}/import?parent={}&overwrite_outside=true",
            server_prefix, encoded_parent
        );
        if let Some(agent) = &store.config.agent {
            let encoded_agent = urlencoding::encode(&agent.subject);
            import_url.push_str("&agent=");
            import_url.push_str(&encoded_agent);
        }

        // Always send JSON-AD regardless of chosen output format.
        let body = serde_json::to_vec(&resources)?;

        use reqwest::header::{ACCEPT, CONTENT_TYPE, HeaderValue};
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .user_agent("Terraphim-Atomic-Client/1.0")
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        let resp = if let Some(agent) = &store.config.agent {
            let mut headers =
                terraphim_atomic_client::get_authentication_headers(agent, &import_url, "POST")?;
            headers.insert(
                CONTENT_TYPE,
                HeaderValue::from_static("application/ad+json"),
            );
            headers.insert(ACCEPT, HeaderValue::from_static("application/ad+json"));
            client
                .post(&import_url)
                .headers(headers)
                .body(body)
                .send()
                .await?
        } else {
            client
                .post(&import_url)
                .body(body)
                .header(CONTENT_TYPE, "application/ad+json")
                .header(ACCEPT, "application/ad+json")
                .send()
                .await?
        };
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        println!(
            "Warning: server responded with status {} during import validation. Body: {}",
            status, text
        );
    } else if validate {
        println!("Validation requires json-ad format – skipped.");
    } else {
        println!("Import validation skipped (pass --validate to enable)");
    }

    Ok(())
}

#[cfg(feature = "native")]
async fn fetch_all_subresources(
    store: &Store,
    root: &str,
) -> terraphim_atomic_client::Result<Vec<terraphim_atomic_client::Resource>> {
    use std::collections::{HashSet, VecDeque};

    let mut visited: HashSet<String> = HashSet::new();
    let mut queue: VecDeque<String> = VecDeque::new();
    let mut collected: Vec<terraphim_atomic_client::Resource> = Vec::new();

    println!("Starting fetch_all_subresources with root: {}", root);
    println!("Server URL: {}", store.config.server_url);

    // First, process the root resource to get its subresources
    println!("Processing root resource first: {}", root);
    match store.get_resource(root).await {
        Ok(res) => {
            println!("Successfully fetched root resource: {}", res.subject);
            collected.push(res.clone());
            visited.insert(root.to_string());

            // Add subresources from root to queue
            if let Some(subresources) = res
                .properties
                .get("https://atomicdata.dev/properties/subresources")
            {
                println!("Found subresources property: {:?}", subresources);
                if let Some(subresources_array) = subresources.as_array() {
                    for subresource in subresources_array {
                        if let Some(subresource_str) = subresource.as_str() {
                            println!("Found subresource: {}", subresource_str);
                            queue.push_back(subresource_str.to_string());
                        }
                    }
                }
            } else {
                println!("No subresources property found in root resource");
            }

            // Also collect links from root resource properties
            let server_prefix = store.config.server_url.trim_end_matches('/');
            for value in res.properties.values() {
                collect_links(&mut queue, value, server_prefix);
            }
        }
        Err(e) => {
            eprintln!("Warning: failed to fetch root resource {}: {}", root, e);
            return Ok(collected);
        }
    }

    // Now process the queue
    while let Some(subject) = queue.pop_front() {
        println!("Processing subject: {}", subject);

        // Basic guards: only follow HTTP/HTTPS URLs belonging to same server.
        // Normalize URLs by removing trailing slashes for comparison
        let normalized_subject = subject.trim_end_matches('/');
        let normalized_server_url = store.config.server_url.trim_end_matches('/');

        println!("Normalized subject: {}", normalized_subject);
        println!("Normalized server URL: {}", normalized_server_url);

        if !normalized_subject.starts_with(normalized_server_url) {
            println!("Skipping {} - not on same server", subject);
            continue;
        }

        if visited.contains(&subject) {
            println!("Already visited: {}", subject);
            continue;
        }
        visited.insert(subject.clone());

        match store.get_resource(&subject).await {
            Ok(res) => {
                println!("Successfully fetched resource: {}", res.subject);
                collected.push(res.clone());
                let server_prefix = store.config.server_url.trim_end_matches('/');

                // Collect links from all properties (for nested resources)
                for value in res.properties.values() {
                    collect_links(&mut queue, value, server_prefix);
                }
            }
            Err(e) => {
                eprintln!("Warning: skipping {}: {}", subject, e);
            }
        }
    }

    println!("Total resources collected: {}", collected.len());
    Ok(collected)
}

#[cfg(feature = "native")]
fn collect_links(
    queue: &mut std::collections::VecDeque<String>,
    value: &serde_json::Value,
    server_prefix: &str,
) {
    if let Some(arr) = value.as_array() {
        for val in arr {
            collect_links(queue, val, server_prefix);
        }
    } else if let Some(obj) = value.as_object() {
        // If object has an @id field, treat it as a link.
        if let Some(id_val) = obj.get("@id") {
            if let Some(id_str) = id_val.as_str() {
                if id_str.starts_with(server_prefix) {
                    println!("Found object link: {}", id_str);
                    queue.push_back(id_str.to_string());
                }
            }
        }

        // Also iterate over object values recursively (handles TranslationBoxes etc.)
        for (_k, v) in obj {
            collect_links(queue, v, server_prefix);
        }
    } else if let Some(str_val) = value.as_str() {
        if str_val.starts_with(server_prefix) {
            println!("Found string link: {}", str_val);
            queue.push_back(str_val.to_string());
        }
    }
}

#[cfg(feature = "native")]
async fn export_ontology(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    // Usage: terraphim_atomic_client export-ontology <ontology_subject> [output_file] [format] [--validate]

    if args.len() < 3 {
        println!(
            "Usage: terraphim_atomic_client export-ontology <ontology_subject> [output_file] [format] [--validate]"
        );
        return Ok(());
    }

    let ontology_subject = &args[2];
    let output_file = if args.len() > 3 {
        &args[3]
    } else {
        "ontology.json"
    };
    let format = if args.len() > 4 {
        args[4].to_lowercase()
    } else {
        "json-ad".to_string()
    };
    let validate = args.iter().any(|a| a == "--validate");

    // Load configuration from environment
    let config = Config::from_env()?;
    let store = Store::new(config.clone())?;

    // Gather resources using new helper
    let resources = store.gather_ontology_resources(ontology_subject).await?;

    // Build mapping of subject -> localId (path relative to ontology root)
    let ontology_path = {
        let url = ontology_subject.trim_end_matches('/');
        let after = url.split('/').next_back().unwrap_or("");
        if after.is_empty() { url } else { after }
    };

    let mut mapping: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    let scope_prefix = format!("{}/", ontology_subject.trim_end_matches('/'));
    for res in &resources {
        let local_id = if res.subject == *ontology_subject {
            ontology_path.to_string()
        } else {
            let rest = res.subject.trim_start_matches(&scope_prefix);
            format!("{}/{}", ontology_path, rest)
        };
        mapping.insert(res.subject.clone(), local_id);
    }

    // Helper to convert values using mapping (similar to export_ontology)
    fn map_value(
        val: &serde_json::Value,
        mapping: &std::collections::HashMap<String, String>,
        root_subject: &str,
    ) -> serde_json::Value {
        match val {
            serde_json::Value::String(s) => {
                if let Some(m) = mapping.get(s) {
                    serde_json::Value::String(m.clone())
                } else if s.starts_with("http://") || s.starts_with("https://") {
                    // Check if this is a reference to the root resource
                    if s == root_subject {
                        // Convert root resource reference to empty string or relative path
                        serde_json::Value::String("".to_string())
                    } else {
                        // Keep other full URLs as-is
                        serde_json::Value::String(s.clone())
                    }
                } else {
                    // For relative paths or other strings, keep them as-is
                    // These will be treated as localId references or literal values
                    serde_json::Value::String(s.clone())
                }
            }
            serde_json::Value::Array(arr) => serde_json::Value::Array(
                arr.iter()
                    .map(|v| map_value(v, mapping, root_subject))
                    .collect(),
            ),
            serde_json::Value::Object(obj) => {
                let mut new_obj = serde_json::Map::new();
                for (k, v) in obj {
                    if k == "@id" {
                        if let Some(id_str) = v.as_str() {
                            if id_str.starts_with("http://") || id_str.starts_with("https://") {
                                // Check if this is a reference to the root resource
                                if id_str == root_subject {
                                    // Skip root resource references
                                    continue;
                                } else {
                                    // Keep other full URLs as-is for @id properties
                                    new_obj.insert(
                                        k.clone(),
                                        serde_json::Value::String(id_str.to_string()),
                                    );
                                    continue;
                                }
                            } else if let Some(mapped) = mapping.get(id_str) {
                                // If it's in the mapping, use the mapped value
                                new_obj
                                    .insert(k.clone(), serde_json::Value::String(mapped.clone()));
                                continue;
                            } else {
                                // If @id is not a full URL and not in mapping, skip it
                                continue;
                            }
                        }
                    }
                    new_obj.insert(k.clone(), map_value(v, mapping, root_subject));
                }
                serde_json::Value::Object(new_obj)
            }
            _ => val.clone(),
        }
    }

    // Helper to filter out objects with invalid @id values
    fn filter_invalid_objects(val: &serde_json::Value) -> serde_json::Value {
        match val {
            serde_json::Value::Array(arr) => {
                let filtered: Vec<serde_json::Value> = arr
                    .iter()
                    .filter_map(|v| {
                        if let serde_json::Value::Object(obj) = v {
                            // Check if object has an @id that's not a valid URL
                            if let Some(id_val) = obj.get("@id") {
                                if let Some(id_str) = id_val.as_str() {
                                    if !id_str.starts_with("http://")
                                        && !id_str.starts_with("https://")
                                    {
                                        // Skip this object entirely
                                        return None;
                                    }
                                    // Also filter out any objects with @id properties that are inside collection members
                                    // These are metadata objects or existing resources that shouldn't be imported
                                    if obj.contains_key(
                                        "https://atomicdata.dev/properties/collection/currentPage",
                                    ) || obj.contains_key(
                                        "https://atomicdata.dev/properties/collection/pageSize",
                                    ) || obj
                                        .contains_key("https://atomicdata.dev/properties/createdAt")
                                        || obj.contains_key(
                                            "https://atomicdata.dev/properties/publicKey",
                                        )
                                    {
                                        return None;
                                    }
                                }
                            }
                        }
                        Some(filter_invalid_objects(v))
                    })
                    .collect();
                serde_json::Value::Array(filtered)
            }
            serde_json::Value::Object(obj) => {
                let mut new_obj = serde_json::Map::new();
                for (k, v) in obj {
                    new_obj.insert(k.clone(), filter_invalid_objects(v));
                }
                serde_json::Value::Object(new_obj)
            }
            _ => val.clone(),
        }
    }

    let ontology_subject_owned: String = ontology_subject.clone();

    // Transform each resource into object with localId-mapped properties
    let transformed: Vec<serde_json::Value> = resources
        .iter()
        .map(|res| {
            let mut obj = serde_json::Map::new();
            // Constant for lastCommit prop
            const LAST_COMMIT_PROP: &str = "https://atomicdata.dev/properties/lastCommit";

            for (prop, val) in &res.properties {
                if prop == LAST_COMMIT_PROP {
                    continue;
                }
                // Skip parent on root resource (if any)
                if res.subject == ontology_subject_owned
                    && prop == "https://atomicdata.dev/properties/parent"
                {
                    continue;
                }
                // Skip read/write properties that contain agent references
                if prop == "https://atomicdata.dev/properties/read"
                    || prop == "https://atomicdata.dev/properties/write"
                {
                    if let serde_json::Value::Array(arr) = val {
                        let filtered: Vec<serde_json::Value> = arr
                            .iter()
                            .filter(|v| {
                                if let serde_json::Value::String(s) = v {
                                    !s.contains("/agents/")
                                } else {
                                    true
                                }
                            })
                            .cloned()
                            .collect();
                        if !filtered.is_empty() {
                            let prop_key = mapping.get(prop).unwrap_or(prop).clone();
                            let filtered_val =
                                filter_invalid_objects(&serde_json::Value::Array(filtered));
                            obj.insert(
                                prop_key,
                                map_value(&filtered_val, &mapping, &ontology_subject_owned),
                            );
                        }
                    }
                    continue;
                }
                let prop_key = mapping.get(prop).unwrap_or(prop).clone();
                // First filter out invalid objects, then map values
                let filtered_val = filter_invalid_objects(val);
                obj.insert(
                    prop_key,
                    map_value(&filtered_val, &mapping, &ontology_subject_owned),
                );
            }
            // Add localId property
            obj.insert(
                "https://atomicdata.dev/properties/localId".to_string(),
                serde_json::Value::String(mapping.get(&res.subject).unwrap().clone()),
            );
            serde_json::Value::Object(obj)
        })
        .collect();

    // Write to disk
    use std::fs::File;
    use std::io::Write;
    let mut file = File::create(output_file)?;

    match format.as_str() {
        "json" | "json-ad" => {
            let serialized = serde_json::to_vec_pretty(&transformed)?;
            file.write_all(&serialized)?;
        }
        "turtle" => {
            let client = reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .user_agent("Terraphim-Atomic-Client/1.0")
                .build()
                .unwrap_or_else(|_| reqwest::Client::new());
            use reqwest::header::ACCEPT;
            for res in &resources {
                let resp = client
                    .get(&res.subject)
                    .header(ACCEPT, "text/turtle")
                    .send()
                    .await?;
                if resp.status().is_success() {
                    writeln!(file, "{}", resp.text().await?)?;
                } else {
                    eprintln!(
                        "Failed Turtle serialization for {}: {}",
                        res.subject,
                        resp.status()
                    );
                }
            }
        }
        other => {
            println!(
                "Unsupported format '{}'. Supported: json, json-ad, turtle",
                other
            );
            return Ok(());
        }
    }

    println!(
        "Exported ontology with {} resources to {} in {} format",
        transformed.len(),
        output_file,
        format
    );

    if validate && format == "json-ad" {
        let server_prefix = store.config.server_url.trim_end_matches('/');
        let encoded_parent = urlencoding::encode(ontology_subject);
        let mut import_url = format!(
            "{}/import?parent={}&overwrite_outside=true",
            server_prefix, encoded_parent
        );
        if let Some(agent) = &store.config.agent {
            let encoded_agent = urlencoding::encode(&agent.subject);
            import_url.push_str("&agent=");
            import_url.push_str(&encoded_agent);
        }

        // Always send JSON-AD transformed payload for validation
        let body = serde_json::to_vec(&transformed)?;

        use reqwest::header::{ACCEPT, CONTENT_TYPE, HeaderValue};
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .user_agent("Terraphim-Atomic-Client/1.0")
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        let resp = if let Some(agent) = &store.config.agent {
            let mut headers =
                terraphim_atomic_client::get_authentication_headers(agent, &import_url, "POST")?;
            headers.insert(
                CONTENT_TYPE,
                HeaderValue::from_static("application/ad+json"),
            );
            headers.insert(ACCEPT, HeaderValue::from_static("application/ad+json"));
            client
                .post(&import_url)
                .headers(headers)
                .body(body)
                .send()
                .await?
        } else {
            client
                .post(&import_url)
                .body(body)
                .header(CONTENT_TYPE, "application/ad+json")
                .header(ACCEPT, "application/ad+json")
                .send()
                .await?
        };
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        println!(
            "Warning: server responded with status {} during ontology import validation. Body: {}",
            status, text
        );
    } else if validate {
        println!("Validation requires json-ad format – skipped.");
    } else {
        println!("Import validation skipped (pass --validate to enable)");
    }

    Ok(())
}

#[cfg(feature = "native")]
async fn collection_query(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    // Usage: terraphim_atomic_client collection <class_url> <sort_property_url> [--desc] [--limit N]
    if args.len() < 4 {
        println!(
            "Usage: terraphim_atomic_client collection <class_url> <sort_property_url> [--desc] [--limit N]"
        );
        return Ok(());
    }

    let class_url = &args[2];
    let sort_property_url = &args[3];
    let desc = args.iter().any(|a| a == "--desc");
    let limit_opt = args
        .iter()
        .position(|a| a == "--limit")
        .and_then(|idx| args.get(idx + 1))
        .and_then(|s| s.parse::<u32>().ok());

    // Load configuration from environment
    let config = Config::from_env()?;
    let store = Store::new(config)?;

    let res: serde_json::Value = store
        .collection_by_class(class_url, sort_property_url, desc, limit_opt)
        .await?;
    println!("{}", serde_json::to_string_pretty(&res)?);

    Ok(())
}

#[cfg(feature = "native")]
async fn export_to_local(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    // Usage: terraphim_atomic_client export-to-local <root_subject> [output_file] [format] [--validate]
    if args.len() < 3 {
        println!(
            "Usage: terraphim_atomic_client export-to-local <root_subject> [output_file] [format] [--validate]"
        );
        return Ok(());
    }

    let mut root_subject = args[2].clone();
    // Load configuration from environment
    let config = Config::from_env()?;
    let store = Store::new(config.clone())?;

    // If user passed bare server URL, try /drive, but fall back to server root if /drive not found
    let server_prefix_cfg = config.server_url.trim_end_matches('/');
    if root_subject.trim_end_matches('/') == server_prefix_cfg {
        let candidate = format!("{}/drive", server_prefix_cfg);
        match store.get_resource(&candidate).await {
            Ok(_r) => {
                root_subject = candidate;
            }
            Err(_e) => {
                // Keep original server root
                root_subject = server_prefix_cfg.to_string();
            }
        }
    }

    let output_file = if args.len() > 3 {
        &args[3]
    } else {
        "export.json"
    };
    let format = if args.len() > 4 {
        args[4].to_lowercase()
    } else {
        "json-ad".to_string()
    };
    let validate = args.iter().any(|a| a == "--validate");

    // Fetch all in-server resources recursively, starting at resolved root_subject.
    let resources = fetch_all_subresources(&store, &root_subject).await?;

    // Filter out Agent and Commit resources – keep them external to avoid invalid localIds
    let filtered_resources: Vec<_> = resources
        .into_iter()
        .filter(|res| {
            !res.subject.contains("/agents/") &&
            !res.subject.contains("/commits/") &&
            // Only include resources that are part of the borrower-portal domain
            (res.subject.contains("/borrower-portal/") || res.subject == root_subject)
        })
        .collect();

    let server_prefix = store.config.server_url.trim_end_matches('/');
    let root_subject_owned = root_subject.clone();

    let mut mapping: HashMap<String, String> = HashMap::new();
    let scope_prefix = format!("{}/", server_prefix);
    for res in &filtered_resources {
        let local_id = res.subject.trim_start_matches(&scope_prefix).to_string();
        mapping.insert(res.subject.clone(), local_id);
    }

    // Helper to convert values using mapping (similar to export_ontology)
    fn map_value(
        val: &serde_json::Value,
        mapping: &HashMap<String, String>,
        root_subject: &str,
    ) -> serde_json::Value {
        match val {
            serde_json::Value::String(s) => {
                if let Some(m) = mapping.get(s) {
                    serde_json::Value::String(m.clone())
                } else if s.starts_with("http://") || s.starts_with("https://") {
                    // Check if this is a reference to the root resource
                    if s == root_subject {
                        // Convert root resource reference to empty string or relative path
                        serde_json::Value::String("".to_string())
                    } else {
                        // Keep other full URLs as-is
                        serde_json::Value::String(s.clone())
                    }
                } else {
                    // For relative paths or other strings, keep them as-is
                    // These will be treated as localId references or literal values
                    serde_json::Value::String(s.clone())
                }
            }
            serde_json::Value::Array(arr) => serde_json::Value::Array(
                arr.iter()
                    .map(|v| map_value(v, mapping, root_subject))
                    .collect(),
            ),
            serde_json::Value::Object(obj) => {
                let mut new_obj = serde_json::Map::new();
                for (k, v) in obj {
                    if k == "@id" {
                        if let Some(id_str) = v.as_str() {
                            if id_str.starts_with("http://") || id_str.starts_with("https://") {
                                // Check if this is a reference to the root resource
                                if id_str == root_subject {
                                    // Skip root resource references
                                    continue;
                                } else {
                                    // Keep other full URLs as-is for @id properties
                                    new_obj.insert(
                                        k.clone(),
                                        serde_json::Value::String(id_str.to_string()),
                                    );
                                    continue;
                                }
                            } else if let Some(mapped) = mapping.get(id_str) {
                                // If it's in the mapping, use the mapped value
                                new_obj
                                    .insert(k.clone(), serde_json::Value::String(mapped.clone()));
                                continue;
                            } else {
                                // If @id is not a full URL and not in mapping, skip it
                                continue;
                            }
                        }
                    }
                    new_obj.insert(k.clone(), map_value(v, mapping, root_subject));
                }
                serde_json::Value::Object(new_obj)
            }
            _ => val.clone(),
        }
    }

    // Helper to filter out objects with invalid @id values
    fn filter_invalid_objects(val: &serde_json::Value) -> serde_json::Value {
        match val {
            serde_json::Value::Array(arr) => {
                let filtered: Vec<serde_json::Value> = arr
                    .iter()
                    .filter_map(|v| {
                        if let serde_json::Value::Object(obj) = v {
                            // Check if object has an @id that's not a valid URL
                            if let Some(id_val) = obj.get("@id") {
                                if let Some(id_str) = id_val.as_str() {
                                    if !id_str.starts_with("http://")
                                        && !id_str.starts_with("https://")
                                    {
                                        // Skip this object entirely
                                        return None;
                                    }
                                    // Also filter out any objects with @id properties that are inside collection members
                                    // These are metadata objects or existing resources that shouldn't be imported
                                    if obj.contains_key(
                                        "https://atomicdata.dev/properties/collection/currentPage",
                                    ) || obj.contains_key(
                                        "https://atomicdata.dev/properties/collection/pageSize",
                                    ) || obj
                                        .contains_key("https://atomicdata.dev/properties/createdAt")
                                        || obj.contains_key(
                                            "https://atomicdata.dev/properties/publicKey",
                                        )
                                    {
                                        return None;
                                    }
                                }
                            }
                        }
                        Some(filter_invalid_objects(v))
                    })
                    .collect();
                serde_json::Value::Array(filtered)
            }
            serde_json::Value::Object(obj) => {
                let mut new_obj = serde_json::Map::new();
                for (k, v) in obj {
                    new_obj.insert(k.clone(), filter_invalid_objects(v));
                }
                serde_json::Value::Object(new_obj)
            }
            _ => val.clone(),
        }
    }

    // Transform resources
    const LAST_COMMIT_PROP: &str = "https://atomicdata.dev/properties/lastCommit";
    let transformed: Vec<serde_json::Value> = filtered_resources
        .iter()
        .filter_map(|res| {
            // Skip the root resource itself since it already exists
            if res.subject == root_subject_owned {
                return None;
            }

            let mut obj = serde_json::Map::new();
            for (prop, val) in &res.properties {
                if prop == LAST_COMMIT_PROP {
                    continue;
                }
                // Skip parent on root resource (if any)
                if res.subject == root_subject_owned
                    && prop == "https://atomicdata.dev/properties/parent"
                {
                    continue;
                }
                // Skip read/write properties that contain agent references
                if prop == "https://atomicdata.dev/properties/read"
                    || prop == "https://atomicdata.dev/properties/write"
                {
                    if let serde_json::Value::Array(arr) = val {
                        let filtered: Vec<serde_json::Value> = arr
                            .iter()
                            .filter(|v| {
                                if let serde_json::Value::String(s) = v {
                                    !s.contains("/agents/")
                                } else {
                                    true
                                }
                            })
                            .cloned()
                            .collect();
                        if !filtered.is_empty() {
                            let prop_key = mapping.get(prop).unwrap_or(prop).clone();
                            let filtered_val =
                                filter_invalid_objects(&serde_json::Value::Array(filtered));
                            obj.insert(
                                prop_key,
                                map_value(&filtered_val, &mapping, &root_subject_owned),
                            );
                        }
                    }
                    continue;
                }
                let prop_key = mapping.get(prop).unwrap_or(prop).clone();
                // First filter out invalid objects, then map values
                let filtered_val = filter_invalid_objects(val);
                obj.insert(
                    prop_key,
                    map_value(&filtered_val, &mapping, &root_subject_owned),
                );
            }
            // localId
            obj.insert(
                "https://atomicdata.dev/properties/localId".to_string(),
                serde_json::Value::String(mapping.get(&res.subject).unwrap().clone()),
            );
            Some(serde_json::Value::Object(obj))
        })
        .collect();

    // Write to disk
    use std::fs::File;
    use std::io::Write;
    let mut file = File::create(output_file)?;
    match format.as_str() {
        "json" | "json-ad" => {
            let serialized = serde_json::to_vec_pretty(&transformed)?;
            file.write_all(&serialized)?;
        }
        "turtle" => {
            let client = reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .user_agent("Terraphim-Atomic-Client/1.0")
                .build()
                .unwrap_or_else(|_| reqwest::Client::new());
            use reqwest::header::ACCEPT;
            for res in &filtered_resources {
                let resp = client
                    .get(&res.subject)
                    .header(ACCEPT, "text/turtle")
                    .send()
                    .await?;
                if resp.status().is_success() {
                    writeln!(file, "{}", resp.text().await?)?;
                } else {
                    eprintln!(
                        "Failed Turtle serialization for {}: {}",
                        res.subject,
                        resp.status()
                    );
                }
            }
        }
        other => {
            println!(
                "Unsupported format '{}'. Supported: json, json-ad, turtle",
                other
            );
            return Ok(());
        }
    }

    println!(
        "Exported {} resources to {} in {} format",
        transformed.len(),
        output_file,
        format
    );

    if validate && format == "json-ad" {
        let server_prefix = store.config.server_url.trim_end_matches('/');
        let encoded_parent = urlencoding::encode(&root_subject);
        let mut import_url = format!(
            "{}/import?parent={}&overwrite_outside=true",
            server_prefix, encoded_parent
        );
        if let Some(agent) = &store.config.agent {
            let encoded_agent = urlencoding::encode(&agent.subject);
            import_url.push_str("&agent=");
            import_url.push_str(&encoded_agent);
        }
        let body = serde_json::to_vec(&transformed)?;
        use reqwest::header::{ACCEPT, CONTENT_TYPE, HeaderValue};
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .user_agent("Terraphim-Atomic-Client/1.0")
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        let resp = if let Some(agent) = &store.config.agent {
            let mut headers =
                terraphim_atomic_client::get_authentication_headers(agent, &import_url, "POST")?;
            headers.insert(
                CONTENT_TYPE,
                HeaderValue::from_static("application/ad+json"),
            );
            headers.insert(ACCEPT, HeaderValue::from_static("application/ad+json"));
            client
                .post(&import_url)
                .headers(headers)
                .body(body)
                .send()
                .await?
        } else {
            client
                .post(&import_url)
                .body(body)
                .header(CONTENT_TYPE, "application/ad+json")
                .header(ACCEPT, "application/ad+json")
                .send()
                .await?
        };
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        println!(
            "Warning: server responded with status {} during import validation. Body: {}",
            status, text
        );
    } else if validate {
        println!("Validation requires json-ad format – skipped.");
    } else {
        println!("Import validation skipped (pass --validate to enable)");
    }

    Ok(())
}

#[cfg(feature = "native")]
async fn import_ontology(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    // Skip program name and command name, start from actual arguments
    let import_args = &args[2..];

    if import_args.is_empty() {
        println!(
            "Usage: terraphim_atomic_client import-ontology <json_file> [parent_url] [--validate]"
        );
        println!("  json_file: Path to JSON-AD file to import");
        println!(
            "  parent_url: Optional parent URL (defaults to https://atomicdata.dev/classes/Drive)"
        );
        println!("  --validate: Enable validation of resources");
        return Ok(());
    }

    let json_file = &import_args[0];
    let default_parent = "https://atomicdata.dev/classes/Drive".to_string();
    let parent_url = import_args
        .get(1)
        .filter(|&url| !url.starts_with("--"))
        .unwrap_or(&default_parent);
    let validate = import_args.iter().any(|arg| arg == "--validate");

    println!("Importing ontology from: {}", json_file);
    println!("Using parent URL: {}", parent_url);
    println!("Validation enabled: {}", validate);

    // Read and parse JSON file
    let json_content = std::fs::read_to_string(json_file)?;
    let resources: Vec<serde_json::Map<String, serde_json::Value>> =
        serde_json::from_str(&json_content)?;

    println!("Found {} resources to import", resources.len());

    // Sort resources by dependencies (ontology first, then classes, then properties)
    let sorted_resources = sort_resources_by_dependencies(resources, parent_url)?;

    // Initialize store
    let store = initialize_store().await?;

    let mut success_count = 0;
    let mut failed_resources = Vec::new();

    // Import resources in dependency order
    for (index, resource_obj) in sorted_resources.iter().enumerate() {
        match import_single_resource(&store, resource_obj, parent_url, validate).await {
            Ok(subject) => {
                println!(
                    "✓ Successfully imported resource {}/{}: {}",
                    index + 1,
                    sorted_resources.len(),
                    subject
                );
                success_count += 1;
            }
            Err(e) => {
                println!(
                    "✗ Failed to import resource {}/{}: {}",
                    index + 1,
                    sorted_resources.len(),
                    e
                );
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
            println!(
                "  ✗ Failed to import resource {}/{}: {}",
                index,
                sorted_resources.len(),
                error
            );
        }
        println!("\n✗ No resources were imported");
    } else {
        println!("✓ All resources imported successfully!");
    }

    Ok(())
}

#[cfg(feature = "native")]
fn sort_resources_by_dependencies(
    resources: Vec<serde_json::Map<String, serde_json::Value>>,
    _parent_url: &str,
) -> Result<Vec<serde_json::Map<String, serde_json::Value>>, Box<dyn std::error::Error>> {
    // Define dependency order: ontology -> classes -> properties
    let mut sorted = Vec::new();
    let mut remaining = resources;

    // First, find and add the ontology resource
    let ontology_index = remaining.iter().position(|r| {
        r.get("https://atomicdata.dev/properties/isA")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .any(|item| item.as_str() == Some("https://atomicdata.dev/class/ontology"))
            })
            .unwrap_or(false)
    });

    if let Some(index) = ontology_index {
        sorted.push(remaining.remove(index));
    }

    // Then add all class resources
    let class_indices: Vec<usize> = remaining
        .iter()
        .enumerate()
        .filter(|(_, r)| {
            r.get("https://atomicdata.dev/properties/isA")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .any(|item| item.as_str() == Some("https://atomicdata.dev/classes/Class"))
                })
                .unwrap_or(false)
        })
        .map(|(i, _)| i)
        .collect();

    // Add classes in reverse order to maintain original order
    for &index in class_indices.iter().rev() {
        sorted.push(remaining.remove(index));
    }

    // Finally add all property resources
    let property_indices: Vec<usize> = remaining
        .iter()
        .enumerate()
        .filter(|(_, r)| {
            r.get("https://atomicdata.dev/properties/isA")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter().any(|item| {
                        item.as_str() == Some("https://atomicdata.dev/classes/Property")
                    })
                })
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
        id_value
            .as_str()
            .ok_or("@id field must be a string")?
            .to_string()
    } else if let Some(local_id) = resource_obj.get("https://atomicdata.dev/properties/localId") {
        // Convert localId to full URL using parent context
        let local_id_str = local_id.as_str().ok_or("localId field must be a string")?;
        format!("{}/{}", parent_url.trim_end_matches('/'), local_id_str)
    } else {
        // Generate a new subject URL based on parent and shortname
        let shortname = resource_obj
            .get("https://atomicdata.dev/properties/shortname")
            .and_then(|v| v.as_str())
            .ok_or("Resource must have either @id, localId, or shortname property")?;

        format!("{}/{}", parent_url.trim_end_matches('/'), shortname)
    };

    // Convert JSON-AD object to properties HashMap
    let mut properties = std::collections::HashMap::new();

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
        let processed_value = if key == "https://atomicdata.dev/properties/classes"
            || key == "https://atomicdata.dev/properties/properties"
        {
            convert_relative_urls_in_array(value, parent_url)?
        } else if key == "https://atomicdata.dev/properties/parent" {
            // Parent should be a localId reference, not a URL
            // Keep it as is - it will be resolved by the atomic server
            value.clone()
        } else {
            value.clone()
        };

        // Validate property URLs if validation is enabled
        if validate && !key.starts_with("https://") && !key.starts_with("http://") {
            return Err(format!("Invalid property key '{}': must be a valid URL", key).into());
        }

        properties.insert(key.clone(), processed_value);
    }

    // Ensure required atomic data properties
    if !properties.contains_key("https://atomicdata.dev/properties/isA") {
        // Default to Class if not specified
        properties.insert(
            "https://atomicdata.dev/properties/isA".to_string(),
            serde_json::json!(["https://atomicdata.dev/classes/Class"]),
        );
    }

    // Validate the resource if requested
    if validate {
        validate_resource(&properties)?;
    }

    // Create the resource using commit
    let _result = store.create_with_commit(&subject, properties).await?;

    Ok(subject)
}

#[cfg(feature = "native")]
fn convert_relative_urls_in_array(
    value: &serde_json::Value,
    parent_url: &str,
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
                        let absolute_url =
                            format!("{}/{}", parent_url.trim_end_matches('/'), url_str);
                        converted.push(serde_json::json!(absolute_url));
                    }
                } else {
                    converted.push(item.clone());
                }
            }
            Ok(serde_json::Value::Array(converted))
        }
        _ => Ok(value.clone()),
    }
}

#[cfg(feature = "native")]
fn validate_resource(
    properties: &std::collections::HashMap<String, serde_json::Value>,
) -> Result<(), Box<dyn std::error::Error>> {
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

    // Validate classes and properties arrays if present
    for array_key in &[
        "https://atomicdata.dev/properties/classes",
        "https://atomicdata.dev/properties/properties",
    ] {
        if let Some(array_value) = properties.get(*array_key) {
            if let serde_json::Value::Array(arr) = array_value {
                for item in arr {
                    if let Some(item_str) = item.as_str() {
                        // Allow both absolute URLs and relative paths (will be converted later)
                        if !item_str.starts_with("https://")
                            && !item_str.starts_with("http://")
                            && !item_str.contains("/")
                        {
                            return Err(format!(
                                "Invalid URL in {} array: {}",
                                array_key, item_str
                            )
                            .into());
                        }
                    } else {
                        return Err(
                            format!("All items in {} array must be strings", array_key).into()
                        );
                    }
                }
            } else {
                return Err(format!("{} property must be an array", array_key).into());
            }
        }
    }

    Ok(())
}

#[cfg(feature = "native")]
async fn initialize_store() -> Result<Store, Box<dyn std::error::Error>> {
    // Load configuration from environment
    let config = Config::from_env()?;
    let store = Store::new(config)?;
    Ok(store)
}
