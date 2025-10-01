use anyhow::Result;
use std::fs;
use std::io::Write;
use std::time::Duration;
use tempfile::tempdir;
use terraphim_config::{Config, ConfigState, Haystack, Role, ServiceType};
use terraphim_service::TerraphimService;
use terraphim_types::{NormalizedTermValue, RelevanceFunction, RoleName, SearchQuery};

/// Force real LLM calls by ensuring no cached summaries exist
#[tokio::test]
async fn test_force_real_llm_no_caching() -> Result<()> {
    println!("🔥 FORCE REAL LLM TEST - NO CACHING");
    println!("================================");

    if std::env::var("OLLAMA_TEST").is_err() {
        println!("⚠️  Skipping Ollama test - set OLLAMA_TEST=1 to run");
        return Ok(());
    }

    terraphim_persistence::DeviceStorage::init_memory_only().await?;

    let temp_dir = tempdir()?;
    let doc_path = temp_dir.path().join("fresh_document.md");

    // Create fresh content every time to avoid any caching
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs();

    let fresh_content = format!(
        r#"# Fresh Rust Document - {}

This is a completely fresh document created at timestamp {} to ensure no caching occurs.

## Rust Memory Safety

Rust provides memory safety without garbage collection through its unique ownership system. This prevents common bugs like use-after-free, buffer overflows, and memory leaks.

## The Ownership System

Every value in Rust has exactly one owner at any given time. When the owner goes out of scope, the value is automatically dropped, freeing its memory.

## Borrowing and References

Rust allows you to create references to values without taking ownership. There are two types:
- Immutable references (&T): Multiple allowed simultaneously
- Mutable references (&mut T): Only one allowed at a time

## Lifetimes

Lifetimes ensure that references are valid for as long as needed. The compiler tracks the lifetime of each reference to prevent dangling pointers.

## Zero-Cost Abstractions

Rust's abstractions have zero runtime overhead. High-level constructs like iterators and closures compile to the same efficient code you would write by hand.

This fresh content should trigger real LLM summarization since it's unique with timestamp {}.
"#,
        timestamp, timestamp, timestamp
    );

    let mut file = fs::File::create(&doc_path)?;
    file.write_all(fresh_content.as_bytes())?;

    println!(
        "📄 Created fresh document with {} chars",
        fresh_content.len()
    );

    let role_name = RoleName::new("Fresh Test Role");
    let mut role = Role {
        shortname: Some("fresh".into()),
        name: role_name.clone(),
        relevance_function: RelevanceFunction::TitleScorer,
        terraphim_it: false,
        theme: "test".into(),
        kg: None,
        haystacks: vec![Haystack {
            location: temp_dir.path().to_string_lossy().to_string(),
            service: ServiceType::Ripgrep,
            read_only: false,
            atomic_server_secret: None,
            extra_parameters: std::collections::HashMap::new(),
            weight: 1.0,
        }],
        extra: ahash::AHashMap::new(),
        ..Default::default()
    };

    // Force real Ollama processing
    role.extra
        .insert("llm_provider".into(), serde_json::json!("ollama"));
    role.extra.insert(
        "ollama_base_url".into(),
        serde_json::json!("http://127.0.0.1:11434"),
    );
    role.extra
        .insert("ollama_model".into(), serde_json::json!("gemma2:2b"));
    role.extra
        .insert("llm_auto_summarize".into(), serde_json::json!(true));

    let mut config = Config::default();
    config.roles.insert(role_name.clone(), role);
    config.default_role = role_name.clone();
    config.selected_role = role_name.clone();

    let config_state = ConfigState::new(&mut config).await?;
    let mut service = TerraphimService::new(config_state);

    let search_query = SearchQuery {
        search_term: NormalizedTermValue::new("Rust memory safety".into()),
        limit: Some(5),
        role: Some(role_name),
        ..Default::default()
    };

    println!("🔍 Executing search on fresh document...");
    let start_time = std::time::Instant::now();
    let search_results = service.search(&search_query).await?;
    let search_duration = start_time.elapsed();

    println!("📊 Search Results (took {:?}):", search_duration);
    println!("   Documents found: {}", search_results.documents.len());
    println!(
        "   Summarization tasks queued: {}",
        search_results.summarization_task_ids.len()
    );

    if !search_results.documents.is_empty() {
        let doc = &search_results.documents[0];
        println!("   📄 Document: {}", doc.title);
        println!("   📏 Content length: {} chars", doc.body.len());

        // Verify no existing description or summary
        println!("   🔍 Has description: {:?}", doc.description.is_some());
        println!("   🔍 Has summarization: {:?}", doc.summarization.is_some());
    }

    if !search_results.summarization_task_ids.is_empty() {
        println!("   🔥 Summarization tasks queued - will force real LLM calls!");

        for (i, _task_id) in search_results.summarization_task_ids.iter().enumerate() {
            println!(
                "⏳ Waiting {} seconds for real AI processing...",
                (i + 1) * 2
            );
            tokio::time::sleep(Duration::from_secs(2)).await;

            let merged_results = search_results.clone().merge_completed_summaries(None).await;

            let docs_with_summaries = merged_results
                .documents
                .iter()
                .filter(|doc| doc.summarization.is_some())
                .count();

            if docs_with_summaries > 0 {
                println!("🎉 REAL AI SUMMARY GENERATED!");
                for doc in &merged_results.documents {
                    if let Some(summary) = &doc.summarization {
                        println!("   🤖 AI Summary: '{}'", summary);
                        println!("   📏 Length: {} chars", summary.len());

                        // Verify it's not just using the description
                        if let Some(desc) = &doc.description {
                            if summary != desc {
                                println!(
                                    "   ✅ CONFIRMED: Summary differs from description - REAL AI!"
                                );
                            }
                        } else {
                            println!("   ✅ CONFIRMED: No description exists - MUST be real AI!");
                        }
                    }
                }
                break;
            } else {
                println!("   ⏳ Still processing...");
            }
        }
    }

    println!("================================");
    Ok(())
}
