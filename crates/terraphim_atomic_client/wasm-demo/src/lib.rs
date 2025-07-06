use wasm_bindgen::prelude::*;
use atomic_server_client::{Config, Store};
use atomic_server_client::Agent;
use once_cell::sync::OnceCell;
use serde_json::json;
use js_sys::Date;
use serde::Serialize;

static STORE: OnceCell<Store> = OnceCell::new();
static RESOURCE_ID: OnceCell<String> = OnceCell::new();

/// Helper struct for returning test results to JS.
#[derive(Serialize)]
struct TestResult {
    name: &'static str,
    success: bool,
    message: String,
}

#[wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();
    wasm_logger::init(wasm_logger::Config::default());
}

fn store() -> &'static Store {
    STORE.get().expect("init_client not called")
}

fn set_id(id: &str) { RESOURCE_ID.set(id.to_string()).ok(); }
fn id() -> String { RESOURCE_ID.get().expect("no id yet").clone() }

fn err<E: core::fmt::Display>(e: E) -> JsValue { JsValue::from_str(&e.to_string()) }

#[wasm_bindgen]
pub async fn init_client(server_url: String, secret_b64: String) -> Result<(), JsValue> {
    let agent_opt = if secret_b64.trim().is_empty() {
        None
    } else {
        Some(Agent::from_base64(&secret_b64).map_err(err)?)
    };

    let cfg = Config {
        server_url: server_url.trim_end_matches('/').to_string(),
        agent: agent_opt,
    };
    let st = Store::new(cfg).map_err(err)?;
    STORE.set(st).map_err(|_| JsValue::from_str("already initialised"))
}

#[wasm_bindgen]
pub async fn create() -> Result<JsValue, JsValue> {
    let unique = Date::now() as u64;
    let subject = format!("{}/wasm-demo-{}", store().config.server_url, unique);
    set_id(&subject);

    let mut props = std::collections::HashMap::new();
    props.insert("https://atomicdata.dev/properties/shortname".into(), json!(format!("wasm-demo-{}", unique)));
    props.insert("https://atomicdata.dev/properties/name".into(), json!(format!("WASM demo {}", unique)));
    props.insert("https://atomicdata.dev/properties/description".into(), json!("created from browser WASM"));
    props.insert("https://atomicdata.dev/properties/parent".into(), json!(store().config.server_url.trim_end_matches('/')));
    props.insert("https://atomicdata.dev/properties/isA".into(), json!(["https://atomicdata.dev/classes/Article"]));

    store().create_with_commit(&subject, props).await.map_err(err)?;
    Ok(JsValue::from_str(&subject))
}

#[wasm_bindgen]
pub async fn read() -> Result<JsValue, JsValue> {
    let res = store().get_resource(&id()).await.map_err(err)?;
    Ok(JsValue::from_str(&serde_json::to_string_pretty(&res).unwrap()))
}

#[wasm_bindgen]
pub async fn update() -> Result<JsValue, JsValue> {
    let mut props = std::collections::HashMap::new();
    props.insert("https://atomicdata.dev/properties/description".into(), json!("updated via WASM"));
    store().update_with_commit(&id(), props).await.map_err(err)?;
    Ok(JsValue::from_str("updated"))
}

#[wasm_bindgen]
pub async fn delete_res() -> Result<JsValue, JsValue> {
    store().delete_with_commit(&id()).await.map_err(err)?;
    Ok(JsValue::from_str("deleted"))
}

#[wasm_bindgen]
pub async fn search(query: String) -> Result<JsValue, JsValue> {
    let results = store().search(&query).await.map_err(err)?;
    Ok(JsValue::from_str(&serde_json::to_string_pretty(&results).unwrap()))
}

async fn test_commit_create() -> Result<(), String> {
    let ts = Date::now() as u64;
    let subject = format!("{}/wasm-commit-{}", store().config.server_url, ts);

    let mut props = std::collections::HashMap::new();
    props.insert("https://atomicdata.dev/properties/shortname".into(), json!(format!("wasm-commit-{}", ts)));
    props.insert("https://atomicdata.dev/properties/name".into(), json!(format!("WASM Commit {}", ts)));
    props.insert("https://atomicdata.dev/properties/description".into(), json!("created via wasm commit"));
    props.insert("https://atomicdata.dev/properties/parent".into(), json!(store().config.server_url.trim_end_matches('/')));
    props.insert("https://atomicdata.dev/properties/isA".into(), json!(["https://atomicdata.dev/classes/Article"]));

    store().create_with_commit(&subject, props.clone()).await.map_err(|e| e.to_string())?;

    // read back
    let _ = store().get_resource(&subject).await.map_err(|e| e.to_string())?;

    // update
    let mut up = std::collections::HashMap::new();
    up.insert("https://atomicdata.dev/properties/description".into(), json!("wasm updated"));
    store().update_with_commit(&subject, up).await.map_err(|e| e.to_string())?;

    // delete
    store().delete_with_commit(&subject).await.map_err(|e| e.to_string())?;
    Ok(())
}

async fn test_search_basic() -> Result<(), String> {
    let _ = store().search("test").await.map_err(|e| e.to_string())?;
    Ok(())
}

async fn test_query_basic() -> Result<(), String> {
    let url = format!("{}/collections?current_page=0&page_size=5&include_nested=true", store().config.server_url.trim_end_matches('/'));
    let _ = store().get_resource(&url).await.map_err(|e| e.to_string())?;
    Ok(())
}

async fn test_create_and_search() -> Result<(), String> {
    let ts = Date::now() as u64;
    let slug = format!("wasm-search-{}", ts);
    let subject = format!("{}/{}", store().config.server_url.trim_end_matches('/'), slug);

    let mut props = std::collections::HashMap::new();
    props.insert("https://atomicdata.dev/properties/shortname".into(), json!(slug.clone()));
    props.insert("https://atomicdata.dev/properties/name".into(), json!(format!("Search {}", slug)));
    props.insert("https://atomicdata.dev/properties/description".into(), json!("searchable"));
    props.insert("https://atomicdata.dev/properties/parent".into(), json!(store().config.server_url.trim_end_matches('/')));
    props.insert("https://atomicdata.dev/properties/isA".into(), json!(["https://atomicdata.dev/classes/Article"]));
    store().create_with_commit(&subject, props).await.map_err(|e| e.to_string())?;

    // small delay not possible synchronously; hope immediate.
    let _ = store().search(&slug).await.map_err(|e| e.to_string())?;
    // cleanup
    store().delete_with_commit(&subject).await.map_err(|e| e.to_string())?;
    Ok(())
}

async fn test_create_and_query() -> Result<(), String> {
    let ts = Date::now() as u64;
    let slug = format!("wasm-query-{}", ts);
    let subject = format!("{}/{}", store().config.server_url.trim_end_matches('/'), slug);

    let mut props = std::collections::HashMap::new();
    props.insert("https://atomicdata.dev/properties/shortname".into(), json!(slug.clone()));
    props.insert("https://atomicdata.dev/properties/name".into(), json!(format!("Query {}", slug)));
    props.insert("https://atomicdata.dev/properties/description".into(), json!("queryable"));
    props.insert("https://atomicdata.dev/properties/parent".into(), json!(store().config.server_url.trim_end_matches('/')));
    props.insert("https://atomicdata.dev/properties/isA".into(), json!(["https://atomicdata.dev/classes/Article"]));
    store().create_with_commit(&subject, props).await.map_err(|e| e.to_string())?;

    let query_url = format!("{}/query?property=https://atomicdata.dev/properties/isA&value=https://atomicdata.dev/classes/Article", store().config.server_url.trim_end_matches('/'));
    let _ = store().get_resource(&query_url).await.map_err(|e| e.to_string())?;

    store().delete_with_commit(&subject).await.map_err(|e| e.to_string())?;
    Ok(())
}

/// Extra props mapping copied from generic test for validation.
fn extra_props(class_url: &str, slug: &str) -> std::collections::HashMap<String, serde_json::Value> {
    let mut m = std::collections::HashMap::new();
    match class_url {
        "https://atomicdata.dev/classes/Atom" => {
            m.insert("https://atomicdata.dev/properties/atom/subject".into(), json!(format!("http://example.com/{}_subject", slug)));
            m.insert("https://atomicdata.dev/properties/atom/property".into(), json!("https://atomicdata.dev/properties/description"));
            m.insert("https://atomicdata.dev/properties/atom/value".into(), json!("dummy"));
        },
        "https://atomicdata.dev/class/Bookmark" => {
            m.insert("https://atomicdata.dev/property/url".into(), json!("http://example.com"));
        },
        "https://atomicdata.dev/classes/File" => {
            m.insert("https://atomicdata.dev/properties/downloadURL".into(), json!("http://example.com/file.bin"));
            m.insert("https://atomicdata.dev/properties/mimetype".into(), json!("application/octet-stream"));
        },
        "https://atomicdata.dev/classes/Endpoint" => {
            m.insert("https://atomicdata.dev/properties/endpoint/parameters".into(), json!([]));
            m.insert("https://atomicdata.dev/properties/endpoint/results".into(), json!([]));
        },
        "https://atomicdata.dev/classes/Property" => {
            m.insert("https://atomicdata.dev/properties/datatype".into(), json!("https://atomicdata.dev/datatypes/string"));
        },
        "https://atomicdata.dev/classes/Redirect" => {
            m.insert("https://atomicdata.dev/properties/destination".into(), json!("http://example.com/dest"));
        },
        "https://atomicdata.dev/classes/SelectProperty" => {
            m.insert("https://atomicdata.dev/properties/allowsOnly".into(), json!(["http://example.com/value1"]));
        },
        "https://atomicdata.dev/classes/Table" => {
            m.insert("https://atomicdata.dev/properties/classtype".into(), json!("https://atomicdata.dev/classes/Article"));
        },
        _ => {}
    }
    m
}

async fn test_generic_classes_crud() -> Result<(), String> {
    let collections_url = format!("{}/collections", store().config.server_url.trim_end_matches('/'));
    let collections_res = store().get_resource(&collections_url).await.map_err(|e| e.to_string())?;
    let members = collections_res.properties["https://atomicdata.dev/properties/collection/members"].as_array().unwrap_or(&vec![]).clone();

    let skip: std::collections::HashSet<&str> = [
        "https://atomicdata.dev/classes/Agent",
        "https://atomicdata.dev/classes/Drive",
        "https://atomicdata.dev/classes/Commit",
        "https://atomicdata.dev/classes/Folder",
        "https://atomicdata.dev/classes/FormattedDate",
        "https://atomicdata.dev/classes/FormattedNumber",
        "https://atomicdata.dev/classes/Invite",
    ].into_iter().collect();

    let mut errors = vec![];

    for member in members {
        let class_url_opt = member.get("https://atomicdata.dev/properties/collection/value").and_then(|v| v.as_str());
        let class_url = match class_url_opt { Some(u) => u, None => continue };
        if skip.contains(class_url) { continue; }

        let name_prop = member.get("https://atomicdata.dev/properties/name").and_then(|v| v.as_str()).unwrap_or("resource");
        let ts = Date::now() as u64;
        let base: String = name_prop.chars().filter(|c| c.is_ascii_alphanumeric()).collect();
        let slug = format!("{}wasm{}", base, ts);
        let subject = format!("{}/{}", store().config.server_url.trim_end_matches('/'), slug);

        let mut props = std::collections::HashMap::new();
        props.insert("https://atomicdata.dev/properties/shortname".into(), json!(slug.clone()));
        props.insert("https://atomicdata.dev/properties/name".into(), json!(format!("WASM {}", name_prop)));
        props.insert("https://atomicdata.dev/properties/description".into(), json!("generated wasm"));
        props.insert("https://atomicdata.dev/properties/parent".into(), json!(store().config.server_url.trim_end_matches('/')));
        props.insert("https://atomicdata.dev/properties/isA".into(), json!([class_url]));
        props.extend(extra_props(class_url, &slug));

        if let Err(e) = store().create_with_commit(&subject, props).await { errors.push(format!("Create {}", e)); continue; }
        if let Err(e) = store().get_resource(&subject).await { errors.push(format!("Read {}", e)); }
        let mut up = std::collections::HashMap::new();
        up.insert("https://atomicdata.dev/properties/description".into(), json!("wasm updated"));
        let _ = store().update_with_commit(&subject, up).await.map_err(|e| errors.push(e.to_string()));
        let _ = store().search(&slug).await.map_err(|e| errors.push(e.to_string()));
        let _ = store().delete_with_commit(&subject).await.map_err(|e| errors.push(e.to_string()));
    }

    if errors.is_empty() { Ok(()) } else { Err(format!("{} errors: {}", errors.len(), errors.join("; "))) }
}

#[wasm_bindgen]
pub async fn run_tests() -> Result<JsValue, JsValue> {
    let mut results: Vec<TestResult> = Vec::new();

    macro_rules! push_res {
        ($name:expr, $future:expr) => {
            match $future.await {
                Ok(_) => results.push(TestResult{ name: $name, success: true, message: "ok".to_string() }),
                Err(e) => results.push(TestResult{ name: $name, success: false, message: e }),
            }
        };
    }

    push_res!("commit_create", test_commit_create());
    push_res!("search_basic", test_search_basic());
    push_res!("query_basic", test_query_basic());
    push_res!("create_and_search", test_create_and_search());
    push_res!("create_and_query", test_create_and_query());
    push_res!("generic_classes_crud", test_generic_classes_crud());

    Ok(JsValue::from_str(&serde_json::to_string(&results).unwrap()))
} 