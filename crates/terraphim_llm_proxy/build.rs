//! Build script for terraphim-llm-proxy
//!
//! Fetches Groq and Cerebras models at build time and generates Rust modules with model constants.
//! Falls back to static lists if APIs are unreachable (offline builds, CI without API keys).
//! Kimi uses Anthropic-compatible API which doesn't have a models endpoint, so it uses fallback only.

use serde::Deserialize;
use std::env;
use std::fs;
use std::path::Path;

// ============================================================================
// Groq Types
// ============================================================================

/// Groq model from API response
#[derive(Debug, Deserialize)]
struct GroqApiModel {
    id: String,
    #[serde(default)]
    context_window: u32,
    #[serde(default)]
    owned_by: String,
}

/// Groq API models list response
#[derive(Debug, Deserialize)]
struct GroqModelsResponse {
    data: Vec<GroqApiModel>,
}

/// Groq model data for code generation
struct GroqModelData {
    id: String,
    context_window: u32,
    max_completion_tokens: Option<u32>,
    owned_by: String,
}

// ============================================================================
// Cerebras Types
// ============================================================================

/// Cerebras model from API response (OpenAI-compatible format)
#[derive(Debug, Deserialize)]
struct CerebrasApiModel {
    id: String,
    #[serde(default)]
    created: u64,
    #[serde(default)]
    owned_by: String,
}

/// Cerebras API models list response
#[derive(Debug, Deserialize)]
struct CerebrasModelsResponse {
    data: Vec<CerebrasApiModel>,
}

/// Cerebras model data for code generation
struct CerebrasModelData {
    id: String,
    created: u64,
    owned_by: String,
}

// ============================================================================
// Kimi Types
// ============================================================================

/// Kimi model from API response (OpenAI-compatible format)
#[derive(Debug, Deserialize)]
struct KimiApiModel {
    id: String,
    #[serde(default)]
    created: u64,
    #[serde(default)]
    object: String,
    #[serde(default)]
    display_name: String,
    #[serde(default)]
    context_length: u64,
    #[serde(default)]
    supports_reasoning: bool,
}

/// Kimi API models list response
#[derive(Debug, Deserialize)]
struct KimiModelsResponse {
    data: Vec<KimiApiModel>,
    object: String,
}

/// Kimi model data for code generation
struct KimiModelData {
    id: String,
    created: u64,
    display_name: String,
    context_length: u64,
    supports_reasoning: bool,
}

// ============================================================================
// Main
// ============================================================================

fn main() {
    // Rerun if fallback files change
    println!("cargo:rerun-if-changed=src/groq_models_fallback.rs");
    println!("cargo:rerun-if-changed=src/cerebras_models_fallback.rs");
    println!("cargo:rerun-if-changed=src/kimi_models_fallback.rs");
    // Rerun if API keys change
    println!("cargo:rerun-if-env-changed=GROQ_API_KEY");
    println!("cargo:rerun-if-env-changed=CEREBRAS_API_KEY");
    println!("cargo:rerun-if-env-changed=KIMI_API_KEY");

    let out_dir = env::var("OUT_DIR").expect("OUT_DIR not set");

    // Generate Groq models
    let groq_dest_path = Path::new(&out_dir).join("groq_models_generated.rs");
    let groq_models = match fetch_groq_models() {
        Ok(models) => {
            println!(
                "cargo:warning=Fetched {} models from Groq API",
                models.len()
            );
            models
        }
        Err(e) => {
            println!(
                "cargo:warning=Failed to fetch Groq models: {}. Using fallback list.",
                e
            );
            get_groq_fallback_models()
        }
    };
    let groq_code = generate_groq_rust_code(&groq_models);
    fs::write(&groq_dest_path, groq_code).expect("Failed to write Groq generated file");

    // Generate Cerebras models
    let cerebras_dest_path = Path::new(&out_dir).join("cerebras_models_generated.rs");
    let cerebras_models = match fetch_cerebras_models() {
        Ok(models) => {
            println!(
                "cargo:warning=Fetched {} models from Cerebras API",
                models.len()
            );
            models
        }
        Err(e) => {
            println!(
                "cargo:warning=Failed to fetch Cerebras models: {}. Using fallback list.",
                e
            );
            get_cerebras_fallback_models()
        }
    };
    let cerebras_code = generate_cerebras_rust_code(&cerebras_models);
    fs::write(&cerebras_dest_path, cerebras_code).expect("Failed to write Cerebras generated file");

    // Generate Kimi models
    let kimi_dest_path = Path::new(&out_dir).join("kimi_models_generated.rs");
    let kimi_models = match fetch_kimi_models() {
        Ok(models) => {
            println!(
                "cargo:warning=Fetched {} models from Kimi API",
                models.len()
            );
            models
        }
        Err(e) => {
            println!(
                "cargo:warning=Failed to fetch Kimi models: {}. Using fallback list.",
                e
            );
            get_kimi_fallback_models()
        }
    };
    let kimi_code = generate_kimi_rust_code(&kimi_models);
    fs::write(&kimi_dest_path, kimi_code).expect("Failed to write Kimi generated file");
}

// ============================================================================
// Groq Functions
// ============================================================================

/// Fetch models from Groq API
fn fetch_groq_models() -> Result<Vec<GroqModelData>, String> {
    let api_key = env::var("GROQ_API_KEY").ok();

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let mut request = client.get("https://api.groq.com/openai/v1/models");

    if let Some(key) = api_key {
        request = request.header("Authorization", format!("Bearer {}", key));
    }

    let response = request
        .send()
        .map_err(|e| format!("HTTP request failed: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("API returned status {}", response.status()));
    }

    let api_response: GroqModelsResponse = response
        .json()
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    let models = api_response
        .data
        .into_iter()
        .filter(|m| !m.id.is_empty())
        .map(|m| {
            let max_tokens = if m.context_window > 0 {
                Some(std::cmp::min(m.context_window / 4, 32768))
            } else {
                None
            };

            GroqModelData {
                id: m.id,
                context_window: m.context_window,
                max_completion_tokens: max_tokens,
                owned_by: if m.owned_by.is_empty() {
                    "Unknown".to_string()
                } else {
                    m.owned_by
                },
            }
        })
        .collect();

    Ok(models)
}

/// Get fallback Groq models from static list
fn get_groq_fallback_models() -> Vec<GroqModelData> {
    vec![
        GroqModelData {
            id: "llama-3.3-70b-versatile".to_string(),
            context_window: 131072,
            max_completion_tokens: Some(32768),
            owned_by: "Meta".to_string(),
        },
        GroqModelData {
            id: "llama-3.1-8b-instant".to_string(),
            context_window: 131072,
            max_completion_tokens: Some(8192),
            owned_by: "Meta".to_string(),
        },
        GroqModelData {
            id: "llama-guard-4-12b".to_string(),
            context_window: 131072,
            max_completion_tokens: Some(8192),
            owned_by: "Meta".to_string(),
        },
        GroqModelData {
            id: "gpt-oss-20b".to_string(),
            context_window: 131072,
            max_completion_tokens: Some(8192),
            owned_by: "OpenAI".to_string(),
        },
        GroqModelData {
            id: "whisper-large-v3".to_string(),
            context_window: 0,
            max_completion_tokens: None,
            owned_by: "OpenAI".to_string(),
        },
        GroqModelData {
            id: "whisper-large-v3-turbo".to_string(),
            context_window: 0,
            max_completion_tokens: None,
            owned_by: "OpenAI".to_string(),
        },
        GroqModelData {
            id: "compound".to_string(),
            context_window: 131072,
            max_completion_tokens: Some(8192),
            owned_by: "Groq".to_string(),
        },
        GroqModelData {
            id: "compound-mini".to_string(),
            context_window: 131072,
            max_completion_tokens: Some(8192),
            owned_by: "Groq".to_string(),
        },
        GroqModelData {
            id: "llama-3.1-70b-versatile".to_string(),
            context_window: 131072,
            max_completion_tokens: Some(32768),
            owned_by: "Meta".to_string(),
        },
        GroqModelData {
            id: "mixtral-8x7b-32768".to_string(),
            context_window: 32768,
            max_completion_tokens: Some(32768),
            owned_by: "Mistral".to_string(),
        },
        GroqModelData {
            id: "gemma2-9b-it".to_string(),
            context_window: 8192,
            max_completion_tokens: Some(8192),
            owned_by: "Google".to_string(),
        },
    ]
}

/// Generate Rust code for the Groq models module
fn generate_groq_rust_code(models: &[GroqModelData]) -> String {
    let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC");

    let mut code = format!(
        r#"// Auto-generated Groq model definitions
// Generated by build.rs - DO NOT EDIT
// Last updated: {}

/// Groq model information
#[derive(Debug, Clone, Copy)]
pub struct GroqModel {{
    pub id: &'static str,
    pub context_window: u32,
    pub max_completion_tokens: Option<u32>,
    pub owned_by: &'static str,
}}

/// All known Groq models
pub const GROQ_MODELS: &[GroqModel] = &[
"#,
        timestamp
    );

    for model in models {
        let max_tokens = match model.max_completion_tokens {
            Some(n) => format!("Some({})", n),
            None => "None".to_string(),
        };

        code.push_str(&format!(
            r#"    GroqModel {{
        id: "{}",
        context_window: {},
        max_completion_tokens: {},
        owned_by: "{}",
    }},
"#,
            model.id, model.context_window, max_tokens, model.owned_by
        ));
    }

    code.push_str(
        r#"];

/// Check if a model ID is a known Groq model
pub fn is_valid_groq_model(model_id: &str) -> bool {
    GROQ_MODELS.iter().any(|m| m.id == model_id)
}

/// Get model info by ID
pub fn get_groq_model(model_id: &str) -> Option<&'static GroqModel> {
    GROQ_MODELS.iter().find(|m| m.id == model_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_groq_models_not_empty() {
        assert!(!GROQ_MODELS.is_empty(), "GROQ_MODELS should not be empty");
    }

    #[test]
    fn test_is_valid_groq_model() {
        assert!(is_valid_groq_model("llama-3.3-70b-versatile"));
        assert!(!is_valid_groq_model("invalid-model-xyz"));
    }

    #[test]
    fn test_get_groq_model() {
        let model = get_groq_model("llama-3.3-70b-versatile");
        assert!(model.is_some());
        assert_eq!(model.unwrap().owned_by, "Meta");
    }
}
"#,
    );

    code
}

// ============================================================================
// Cerebras Functions
// ============================================================================

/// Fetch models from Cerebras API
fn fetch_cerebras_models() -> Result<Vec<CerebrasModelData>, String> {
    let api_key = match env::var("CEREBRAS_API_KEY") {
        Ok(key) if !key.is_empty() => key,
        _ => return Err("CEREBRAS_API_KEY not set, skipping API fetch".to_string()),
    };

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let request = client
        .get("https://api.cerebras.ai/v1/models")
        .header("Authorization", format!("Bearer {}", api_key));

    let response = request
        .send()
        .map_err(|e| format!("HTTP request failed: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("API returned status {}", response.status()));
    }

    let api_response: CerebrasModelsResponse = response
        .json()
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    let models = api_response
        .data
        .into_iter()
        .filter(|m| !m.id.is_empty())
        .map(|m| CerebrasModelData {
            id: m.id,
            created: m.created,
            owned_by: if m.owned_by.is_empty() {
                "Unknown".to_string()
            } else {
                m.owned_by
            },
        })
        .collect();

    Ok(models)
}

/// Get fallback Cerebras models from static list
fn get_cerebras_fallback_models() -> Vec<CerebrasModelData> {
    vec![
        CerebrasModelData {
            id: "llama3.1-8b".to_string(),
            created: 1721692800,
            owned_by: "Meta".to_string(),
        },
        CerebrasModelData {
            id: "llama3.1-70b".to_string(),
            created: 1721692800,
            owned_by: "Meta".to_string(),
        },
        CerebrasModelData {
            id: "llama-3.3-70b".to_string(),
            created: 1733443200,
            owned_by: "Meta".to_string(),
        },
        CerebrasModelData {
            id: "qwen-3-32b".to_string(),
            created: 1733443200,
            owned_by: "Alibaba".to_string(),
        },
    ]
}

/// Generate Rust code for the Cerebras models module
fn generate_cerebras_rust_code(models: &[CerebrasModelData]) -> String {
    let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC");

    let mut code = format!(
        r#"// Auto-generated Cerebras model definitions
// Generated by build.rs - DO NOT EDIT
// Last updated: {}

/// Cerebras model information
#[derive(Debug, Clone, Copy)]
pub struct CerebrasModel {{
    pub id: &'static str,
    pub created: u64,
    pub owned_by: &'static str,
}}

/// All known Cerebras models
pub const CEREBRAS_MODELS: &[CerebrasModel] = &[
"#,
        timestamp
    );

    for model in models {
        code.push_str(&format!(
            r#"    CerebrasModel {{
        id: "{}",
        created: {},
        owned_by: "{}",
    }},
"#,
            model.id, model.created, model.owned_by
        ));
    }

    code.push_str(
        r#"];

/// Check if a model ID is a known Cerebras model
pub fn is_valid_cerebras_model(model_id: &str) -> bool {
    CEREBRAS_MODELS.iter().any(|m| m.id == model_id)
}

/// Get model info by ID
pub fn get_cerebras_model(model_id: &str) -> Option<&'static CerebrasModel> {
    CEREBRAS_MODELS.iter().find(|m| m.id == model_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cerebras_models_not_empty() {
        assert!(!CEREBRAS_MODELS.is_empty(), "CEREBRAS_MODELS should not be empty");
    }

    #[test]
    fn test_is_valid_cerebras_model() {
        assert!(is_valid_cerebras_model("llama3.1-8b"));
        assert!(!is_valid_cerebras_model("invalid-model-xyz"));
    }

    #[test]
    fn test_get_cerebras_model() {
        let model = get_cerebras_model("llama3.1-8b");
        assert!(model.is_some());
        assert_eq!(model.unwrap().owned_by, "Meta");
    }
}
"#,
    );

    code
}

// ============================================================================
// Kimi Functions
// ============================================================================

/// Fetch models from Kimi API
fn fetch_kimi_models() -> Result<Vec<KimiModelData>, String> {
    let api_key = match env::var("KIMI_API_KEY") {
        Ok(key) if !key.is_empty() => key,
        _ => return Err("KIMI_API_KEY not set, skipping API fetch".to_string()),
    };

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let request = client
        .get("https://api.kimi.com/coding/v1/models")
        .header("Authorization", format!("Bearer {}", api_key));

    let response = request
        .send()
        .map_err(|e| format!("HTTP request failed: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("API returned status {}", response.status()));
    }

    let api_response: KimiModelsResponse = response
        .json()
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    let models = api_response
        .data
        .into_iter()
        .filter(|m| !m.id.is_empty())
        .map(|m| {
            let id = m.id;
            KimiModelData {
                id: id.clone(),
                created: m.created,
                display_name: if m.display_name.is_empty() {
                    id
                } else {
                    m.display_name
                },
                context_length: m.context_length,
                supports_reasoning: m.supports_reasoning,
            }
        })
        .collect();

    Ok(models)
}

/// Get fallback Kimi models from static list
fn get_kimi_fallback_models() -> Vec<KimiModelData> {
    vec![KimiModelData {
        id: "kimi-for-coding".to_string(),
        created: 1761264000,
        display_name: "Kimi For Coding".to_string(),
        context_length: 262144,
        supports_reasoning: true,
    }]
}

/// Generate Rust code for the Kimi models module
fn generate_kimi_rust_code(models: &[KimiModelData]) -> String {
    let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC");

    let mut code = format!(
        r#"// Auto-generated Kimi model definitions
// Generated by build.rs - DO NOT EDIT
// Last updated: {}

/// Kimi model information
#[derive(Debug, Clone, Copy)]
pub struct KimiModel {{
    pub id: &'static str,
    pub created: u64,
    pub display_name: &'static str,
    pub context_length: u64,
    pub supports_reasoning: bool,
}}

/// All known Kimi models
pub const KIMI_MODELS: &[KimiModel] = &[
"#,
        timestamp
    );

    for model in models {
        code.push_str(&format!(
            r#"    KimiModel {{
        id: "{}",
        created: {},
        display_name: "{}",
        context_length: {},
        supports_reasoning: {},
    }},
"#,
            model.id,
            model.created,
            model.display_name,
            model.context_length,
            model.supports_reasoning
        ));
    }

    code.push_str(
        r#"];

/// Check if a model ID is a known Kimi model
pub fn is_valid_kimi_model(model_id: &str) -> bool {
    KIMI_MODELS.iter().any(|m| m.id == model_id)
}

/// Get model info by ID
pub fn get_kimi_model(model_id: &str) -> Option<&'static KimiModel> {
    KIMI_MODELS.iter().find(|m| m.id == model_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kimi_models_not_empty() {
        assert!(!KIMI_MODELS.is_empty(), "KIMI_MODELS should not be empty");
    }

    #[test]
    fn test_is_valid_kimi_model() {
        assert!(is_valid_kimi_model("kimi-for-coding"));
        assert!(!is_valid_kimi_model("invalid-model-xyz"));
    }

    #[test]
    fn test_get_kimi_model() {
        let model = get_kimi_model("kimi-for-coding");
        assert!(model.is_some());
        assert_eq!(model.unwrap().display_name, "Kimi For Coding");
    }
}
"#,
    );

    code
}
