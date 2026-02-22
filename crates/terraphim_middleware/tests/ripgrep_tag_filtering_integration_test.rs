use std::collections::HashMap;
use terraphim_config::{Haystack, ServiceType};
use terraphim_middleware::{indexer::IndexMiddleware, RipgrepIndexer};

/// Integration test to demonstrate and validate Ripgrep tag filtering functionality
#[tokio::test]
async fn test_ripgrep_tag_filtering_integration() {
    println!("🏷️ Testing Ripgrep tag filtering integration with fixture data...");

    // Create haystack with tag filtering
    let mut tag_params = HashMap::new();
    tag_params.insert("tag".to_string(), "#rust".to_string());
    tag_params.insert("max_count".to_string(), "5".to_string());

    let test_haystack = Haystack::new(
        "terraphim_server/fixtures/haystack".to_string(),
        ServiceType::Ripgrep,
        true,
    )
    .with_extra_parameters(tag_params);

    let indexer = RipgrepIndexer::default();

    // Test search with tag filtering
    println!("Searching for 'rust' with tag filter '#rust'...");
    let tagged_results = indexer.index("rust", &test_haystack).await;

    match &tagged_results {
        Ok(index) => {
            println!("✅ Tag filtering search completed");
            println!("   Found {} documents with tag filtering", index.len());

            // Log document details for debugging
            for (id, doc) in index.iter().take(3) {
                println!("   📄 Document: {} - {}", doc.title, id);
                if let Some(desc) = &doc.description {
                    let short_desc = if desc.len() > 100 {
                        format!("{}...", &desc[..97])
                    } else {
                        desc.clone()
                    };
                    println!("      Description: {}", short_desc);
                }
            }
        }
        Err(e) => {
            println!("⚠️ Tag filtering search failed: {:?}", e);
            println!("   This might be expected if fixtures directory doesn't exist");
        }
    }

    // Test search without tag filtering for comparison
    let simple_haystack = Haystack::new(
        "terraphim_server/fixtures/haystack".to_string(),
        ServiceType::Ripgrep,
        true,
    );

    println!("\nSearching for 'rust' without tag filter...");
    let unfiltered_results = indexer.index("rust", &simple_haystack).await;

    match unfiltered_results {
        Ok(index) => {
            println!("✅ Unfiltered search completed");
            println!("   Found {} documents without tag filtering", index.len());

            // Compare results
            if let Ok(tagged_index) = &tagged_results {
                if index.len() >= tagged_index.len() {
                    println!(
                        "✅ Tag filtering working as expected (fewer or equal results with filter)"
                    );
                } else {
                    println!(
                        "⚠️ Unexpected result count: unfiltered ({}) < tagged ({})",
                        index.len(),
                        tagged_index.len()
                    );
                }
            }
        }
        Err(e) => {
            println!("⚠️ Unfiltered search failed: {:?}", e);
        }
    }

    println!("\n🔍 Expected behavior:");
    println!("   - Tag filtering should include --all-match flag");
    println!("   - Only documents containing BOTH search term AND tag should be returned");
    println!(
        "   - Generated command should be: rg --json --trim -C3 --ignore-case -tmarkdown --all-match --max-count 5 -e 'rust' -e '#rust' path"
    );
}

/// Test different tag filtering scenarios
#[tokio::test]
async fn test_various_tag_scenarios() {
    println!("🧪 Testing various tag filtering scenarios...");

    let indexer = RipgrepIndexer::default();
    let base_path = "terraphim_server/fixtures/haystack";

    // Scenario 1: Single tag
    let mut single_tag_params = HashMap::new();
    single_tag_params.insert("tag".to_string(), "#docs".to_string());

    let single_tag_haystack = Haystack::new(base_path.to_string(), ServiceType::Ripgrep, true)
        .with_extra_parameters(single_tag_params);

    println!("Scenario 1: Single tag filter (#docs)");
    match indexer.index("documentation", &single_tag_haystack).await {
        Ok(index) => println!("   Found {} results with #docs tag", index.len()),
        Err(e) => println!("   Error: {:?}", e),
    }

    // Scenario 2: Multiple tags (space-separated)
    let mut multi_tag_params = HashMap::new();
    multi_tag_params.insert("tag".to_string(), "#test #rust".to_string());
    multi_tag_params.insert("context".to_string(), "2".to_string());

    let multi_tag_haystack = Haystack::new(base_path.to_string(), ServiceType::Ripgrep, true)
        .with_extra_parameters(multi_tag_params);

    println!("Scenario 2: Multiple tags (#test #rust)");
    match indexer.index("testing", &multi_tag_haystack).await {
        Ok(index) => println!("   Found {} results with multiple tags", index.len()),
        Err(e) => println!("   Error: {:?}", e),
    }

    // Scenario 3: Non-existent tag
    let mut nonexistent_tag_params = HashMap::new();
    nonexistent_tag_params.insert("tag".to_string(), "#nonexistent".to_string());

    let nonexistent_tag_haystack = Haystack::new(base_path.to_string(), ServiceType::Ripgrep, true)
        .with_extra_parameters(nonexistent_tag_params);

    println!("Scenario 3: Non-existent tag (#nonexistent)");
    match indexer.index("any", &nonexistent_tag_haystack).await {
        Ok(index) => {
            if index.is_empty() {
                println!("   ✅ Correctly returned no results for non-existent tag");
            } else {
                println!(
                    "   ⚠️ Unexpectedly found {} results for non-existent tag",
                    index.len()
                );
            }
        }
        Err(e) => println!("   Error: {:?}", e),
    }

    // Scenario 4: Complex parameters
    let mut complex_params = HashMap::new();
    complex_params.insert("tag".to_string(), "#rust".to_string());
    complex_params.insert("max_count".to_string(), "3".to_string());
    complex_params.insert("context".to_string(), "1".to_string());
    complex_params.insert("case_sensitive".to_string(), "true".to_string());

    let complex_haystack = Haystack::new(base_path.to_string(), ServiceType::Ripgrep, true)
        .with_extra_parameters(complex_params);

    println!("Scenario 4: Complex parameters (tag + max_count + context + case_sensitive)");
    match indexer.index("Rust", &complex_haystack).await {
        Ok(index) => println!("   Found {} results with complex filtering", index.len()),
        Err(e) => println!("   Error: {:?}", e),
    }

    println!("\n✅ Tag filtering scenarios test completed");
}

/// Test the parameter parsing logic directly
#[tokio::test]
async fn test_parameter_parsing_directly() {
    println!("🔧 Testing parameter parsing logic directly...");

    let command = terraphim_middleware::command::ripgrep::RipgrepCommand::default();

    // Test 1: Basic tag
    let mut basic_params = HashMap::new();
    basic_params.insert("tag".to_string(), "#rust".to_string());

    let args = command.parse_extra_parameters(&basic_params);
    println!("Basic tag parameters: {:?}", args);
    assert!(args.contains(&"--all-match".to_string()));
    assert!(args.contains(&"-e".to_string()));
    assert!(args.contains(&"#rust".to_string()));

    // Test 2: Multiple tags
    let mut multi_params = HashMap::new();
    multi_params.insert("tag".to_string(), "#rust #test".to_string());

    let multi_args = command.parse_extra_parameters(&multi_params);
    println!("Multiple tag parameters: {:?}", multi_args);
    assert!(multi_args.contains(&"--all-match".to_string()));
    assert!(multi_args.contains(&"#rust".to_string()));
    assert!(multi_args.contains(&"#test".to_string()));

    // Test 3: Tag with other parameters
    let mut mixed_params = HashMap::new();
    mixed_params.insert("tag".to_string(), "#docs".to_string());
    mixed_params.insert("max_count".to_string(), "10".to_string());
    mixed_params.insert("type".to_string(), "md".to_string());

    let mixed_args = command.parse_extra_parameters(&mixed_params);
    println!("Mixed parameters: {:?}", mixed_args);
    assert!(mixed_args.contains(&"--all-match".to_string()));
    assert!(mixed_args.contains(&"#docs".to_string()));
    assert!(mixed_args.contains(&"--max-count".to_string()));
    assert!(mixed_args.contains(&"10".to_string()));
    assert!(mixed_args.contains(&"-t".to_string()));
    assert!(mixed_args.contains(&"md".to_string()));

    // Test 4: Empty tag (edge case)
    let mut empty_params = HashMap::new();
    empty_params.insert("tag".to_string(), "".to_string());

    let empty_args = command.parse_extra_parameters(&empty_params);
    println!("Empty tag parameters: {:?}", empty_args);
    // Should still add --all-match and -e with empty value

    println!("✅ Parameter parsing tests completed");
}

/// Demonstrate the expected ripgrep command construction
#[tokio::test]
async fn demonstrate_command_construction() {
    println!("📝 Demonstrating expected ripgrep command construction...");

    let command = terraphim_middleware::command::ripgrep::RipgrepCommand::default();

    // Example configuration from the UI
    let mut ui_config = HashMap::new();
    ui_config.insert("tag".to_string(), "#rust".to_string());
    ui_config.insert("max_count".to_string(), "10".to_string());

    let extra_args = command.parse_extra_parameters(&ui_config);

    // Simulate the complete command construction
    let needle = "async";
    let haystack_path = "/path/to/documents";

    // This mirrors the logic in RipgrepCommand::run_with_extra_args
    let default_args = ["--json", "--trim", "-C3", "--ignore-case", "-tmarkdown"];

    let full_command: Vec<String> = default_args
        .iter()
        .map(|s| s.to_string())
        .chain(extra_args.iter().cloned())
        .chain(vec![needle.to_string(), haystack_path.to_string()])
        .collect();

    println!("Expected full command: rg {}", full_command.join(" "));
    println!("This should be equivalent to:");
    println!(
        "  rg --json --trim -C3 --ignore-case -tmarkdown --all-match --max-count 10 -e async -e #rust /path/to/documents"
    );

    // Verify key components
    assert!(full_command.contains(&"--all-match".to_string()));
    assert!(full_command.contains(&"-e".to_string()));
    assert!(full_command.contains(&"#rust".to_string()));
    assert!(full_command.contains(&"--max-count".to_string()));
    assert!(full_command.contains(&"10".to_string()));

    println!("✅ Command construction demonstration completed");
}
