use dotenvy::dotenv;
use serde_json::json;
use std::collections::{HashMap, HashSet};
use terraphim_atomic_client::time_utils::unix_timestamp_secs;
use terraphim_atomic_client::{Config, Store};

/// Returns additional properties per class so that creation passes validation.
fn extra_props(class_url: &str, slug: &str) -> HashMap<String, serde_json::Value> {
    let mut m = HashMap::new();
    match class_url {
        // Atom requirements
        "https://atomicdata.dev/classes/Atom" => {
            m.insert(
                "https://atomicdata.dev/properties/atom/subject".to_string(),
                json!(format!("http://example.com/{}_subject", slug)),
            );
            m.insert(
                "https://atomicdata.dev/properties/atom/property".to_string(),
                json!("https://atomicdata.dev/properties/description"),
            );
            m.insert(
                "https://atomicdata.dev/properties/atom/value".to_string(),
                json!("dummy"),
            );
        }
        // Bookmark
        "https://atomicdata.dev/class/Bookmark" => {
            m.insert(
                "https://atomicdata.dev/property/url".to_string(),
                json!("http://example.com"),
            );
        }
        // File
        "https://atomicdata.dev/classes/File" => {
            m.insert(
                "https://atomicdata.dev/properties/downloadURL".to_string(),
                json!("http://example.com/file.bin"),
            );
            m.insert(
                "https://atomicdata.dev/properties/mimetype".to_string(),
                json!("application/octet-stream"),
            );
        }
        // Endpoint
        "https://atomicdata.dev/classes/Endpoint" => {
            m.insert(
                "https://atomicdata.dev/properties/endpoint/parameters".to_string(),
                json!([]),
            );
            m.insert(
                "https://atomicdata.dev/properties/endpoint/results".to_string(),
                json!([]),
            );
        }
        // Property
        "https://atomicdata.dev/classes/Property" => {
            m.insert(
                "https://atomicdata.dev/properties/datatype".to_string(),
                json!("https://atomicdata.dev/datatypes/string"),
            );
        }
        // Redirect
        "https://atomicdata.dev/classes/Redirect" => {
            m.insert(
                "https://atomicdata.dev/properties/destination".to_string(),
                json!("http://example.com/dest"),
            );
        }
        // SelectProperty
        "https://atomicdata.dev/classes/SelectProperty" => {
            m.insert(
                "https://atomicdata.dev/properties/allowsOnly".to_string(),
                json!(["http://example.com/value1"]),
            );
        }
        // Table
        "https://atomicdata.dev/classes/Table" => {
            m.insert(
                "https://atomicdata.dev/properties/classtype".to_string(),
                json!("https://atomicdata.dev/classes/Article"),
            );
        }
        _ => {}
    }
    m
}

#[tokio::test]
async fn generic_classes_crud_search() {
    dotenv().ok();

    // Skip in CI or when environment variables aren't set
    let config = match Config::from_env() {
        Ok(c) => c,
        Err(_) => {
            eprintln!(
                "Skipping test: ATOMIC_SERVER_URL & ATOMIC_SERVER_SECRET not set (integration test requires live server)"
            );
            return;
        }
    };

    if config.agent.is_none() {
        eprintln!("Skipping test: Need authenticated agent");
        return;
    }
    let store = Store::new(config).expect("Create store");

    let skip: HashSet<&str> = [
        // rights or immutable
        "https://atomicdata.dev/classes/Agent",
        "https://atomicdata.dev/classes/Drive",
        "https://atomicdata.dev/classes/Commit",
        // classes that require strict URL fields we don't populate in this generic test
        "https://atomicdata.dev/classes/Article",
        "https://atomicdata.dev/classes/Atom",
        "https://atomicdata.dev/classes/ChatRoom",
        "https://atomicdata.dev/classes/Class",
        "https://atomicdata.dev/classes/Collection",
        "https://atomicdata.dev/classes/Datatype",
        "https://atomicdata.dev/classes/DateFormat",
        "https://atomicdata.dev/classes/Document",
        "https://atomicdata.dev/classes/Endpoint",
        "https://atomicdata.dev/classes/File",
        "https://atomicdata.dev/classes/FloatRangeProperty",
        "https://atomicdata.dev/classes/Importer",
        // special handled in other tests
        "https://atomicdata.dev/classes/Folder",
        "https://atomicdata.dev/classes/FormattedDate",
        "https://atomicdata.dev/classes/FormattedNumber",
        "https://atomicdata.dev/classes/Invite",
    ]
    .into_iter()
    .collect();

    // fetch collections
    let collections_url = format!(
        "{}/collections",
        store.config.server_url.trim_end_matches('/')
    );
    let collections_res = store
        .get_resource(&collections_url)
        .await
        .expect("fetch collections");
    let members = collections_res.properties
        ["https://atomicdata.dev/properties/collection/members"]
        .as_array()
        .expect("members array")
        .clone();

    let mut errors = Vec::new();

    for member in members {
        let class_url = match member
            .get("https://atomicdata.dev/properties/collection/value")
            .and_then(|v| v.as_str())
        {
            Some(u) => u,
            None => continue,
        };

        // Skip custom application classes - only test standard Atomic Data classes
        if !class_url.starts_with("https://atomicdata.dev/classes/") {
            continue;
        }

        if skip.contains(class_url) {
            continue;
        }

        let name_prop = member
            .get("https://atomicdata.dev/properties/name")
            .and_then(|v| v.as_str())
            .unwrap_or("resource");
        let ts = unix_timestamp_secs();
        let base: String = name_prop
            .chars()
            .filter(char::is_ascii_alphanumeric)
            .collect();
        let slug = format!("{}test{}", base, ts);
        let subject = format!("{}/{}", store.config.server_url.trim_end_matches('/'), slug);

        let mut props = HashMap::new();
        props.insert(
            "https://atomicdata.dev/properties/shortname".to_string(),
            json!(slug.clone()),
        );
        props.insert(
            "https://atomicdata.dev/properties/name".to_string(),
            json!(format!("Test {}", name_prop)),
        );
        props.insert(
            "https://atomicdata.dev/properties/description".to_string(),
            json!("Generated by tests"),
        );
        props.insert(
            "https://atomicdata.dev/properties/parent".to_string(),
            json!(store.config.server_url.trim_end_matches('/')),
        );
        props.insert(
            "https://atomicdata.dev/properties/isA".to_string(),
            json!([class_url]),
        );
        props.extend(extra_props(class_url, &slug));

        if let Err(e) = store.create_with_commit(&subject, props).await {
            errors.push(format!("Create failed for {}: {}", class_url, e));
            continue;
        }
        if let Err(e) = store.get_resource(&subject).await {
            errors.push(format!("Read failed {}", e));
        }
        let mut up_props = HashMap::new();
        up_props.insert(
            "https://atomicdata.dev/properties/description".to_string(),
            json!("updated"),
        );
        let _ = store
            .update_with_commit(&subject, up_props)
            .await
            .map_err(|e| errors.push(format!("Update failed {}", e)));
        let _ = store
            .search(&slug)
            .await
            .map_err(|e| errors.push(format!("Search failed {}", e)));
        let _ = store
            .delete_with_commit(&subject)
            .await
            .map_err(|e| errors.push(format!("Delete failed {}", e)));
    }

    if !errors.is_empty() {
        panic!("{} errors:\n{}", errors.len(), errors.join("\n"));
    }
}
