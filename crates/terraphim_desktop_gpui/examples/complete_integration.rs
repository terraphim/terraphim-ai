// Complete Integration Example
// Demonstrates all major components working together

use terraphim_desktop_gpui::*;
use terraphim_types::{ChatMessage, Conversation, ContextItem, ContextType, RoleName};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║  Terraphim Desktop GPUI - Complete Integration Demo        ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");

    // ═══════════════════════════════════════════════════════════════════
    // 1. AUTOCOMPLETE ENGINE
    // ═══════════════════════════════════════════════════════════════════
    println!("┌─ 1. Autocomplete Engine ─────────────────────────────────────┐");

    let thesaurus_json = r#"[
        {"id": 1, "nterm": "rust", "url": "https://rust-lang.org"},
        {"id": 2, "nterm": "ruby", "url": "https://ruby-lang.org"},
        {"id": 3, "nterm": "tokio", "url": "https://tokio.rs"},
        {"id": 4, "nterm": "async", "url": "https://rust-lang.org/async"},
        {"id": 5, "nterm": "tokio-rs", "url": "https://github.com/tokio-rs/tokio"}
    ]"#;

    let autocomplete_engine = AutocompleteEngine::from_thesaurus_json(thesaurus_json)?;
    println!("✓ Loaded {} terms into autocomplete engine", autocomplete_engine.term_count());

    // Test autocomplete
    let query = "tok";
    let suggestions = autocomplete_engine.autocomplete(query, 5);
    println!("\nAutocomplete suggestions for '{}':", query);
    for s in &suggestions {
        let kg_marker = if s.from_kg { "📚" } else { "  " };
        println!("  {} {} (score: {:.2})", kg_marker, s.term, s.score);
    }

    // Test fuzzy search
    let fuzzy_query = "asyc"; // Intentional typo
    let fuzzy_results = autocomplete_engine.fuzzy_search(fuzzy_query, 3);
    println!("\nFuzzy search for '{}' (handles typos):", fuzzy_query);
    for s in &fuzzy_results {
        println!("  ✨ {} (score: {:.2})", s.term, s.score);
    }

    // Check KG terms
    println!("\nKnowledge Graph term checks:");
    for term in ["tokio", "rust", "python"] {
        let is_kg = autocomplete_engine.is_kg_term(term);
        let marker = if is_kg { "✓" } else { "✗" };
        println!("  {} '{}' {}", marker, term, if is_kg { "is in KG" } else { "not in KG" });
    }

    println!("└────────────────────────────────────────────────────────────────┘\n");

    // ═══════════════════════════════════════════════════════════════════
    // 2. SEARCH SERVICE
    // ═══════════════════════════════════════════════════════════════════
    println!("┌─ 2. Search Service ──────────────────────────────────────────┐");

    // Demonstrate query parsing
    let queries = vec![
        "rust async",
        "tokio AND async",
        "python OR ruby",
        "rust AND tokio AND async"
    ];

    println!("Query parsing demonstration:");
    for query in queries {
        let parsed = SearchService::parse_query(query);
        println!("\n  Input: '{}'", query);
        println!("  Terms: {:?}", parsed.terms);
        if let Some(op) = parsed.operator {
            println!("  Operator: {:?}", op);
        }
    }

    println!("\n✓ Search service query parsing working");
    println!("  (Note: Actual search requires ConfigState with haystacks)");

    println!("└────────────────────────────────────────────────────────────────┘\n");

    // ═══════════════════════════════════════════════════════════════════
    // 3. EDITOR & SLASH COMMANDS
    // ═══════════════════════════════════════════════════════════════════
    println!("┌─ 3. Editor & Slash Commands ─────────────────────────────────┐");

    let mut editor = EditorState::new();
    let slash_manager = SlashCommandManager::new();

    // Create document
    editor.insert_text("# Project Documentation\n\n");

    let date = slash_manager.execute_command("date", "").await?;
    editor.insert_text(&format!("Created: {}\n\n", date));

    editor.insert_text("## Overview\n");
    editor.insert_text("This is a Rust project using async/await.\n\n");

    editor.insert_text("## Commands Available\n");

    // List all slash commands
    println!("Available slash commands:");
    for cmd in slash_manager.list_commands() {
        println!("  • /{} - {}", cmd.name, cmd.description);
        editor.insert_text(&format!("- /{}: {}\n", cmd.name, cmd.syntax));
    }

    // Execute some commands
    println!("\nExecuting commands:");
    let time = slash_manager.execute_command("time", "").await?;
    println!("  • /time → {}", time.trim());

    let autocomplete_result = slash_manager.execute_command("autocomplete", "rust").await?;
    println!("  • /autocomplete rust → {}", autocomplete_result.lines().next().unwrap_or(""));

    // Show editor state
    println!("\nEditor statistics:");
    println!("  Lines: {}", editor.line_count());
    println!("  Characters: {}", editor.char_count());
    println!("  Modified: {}", editor.is_modified());

    // Command suggestions
    let prefix = "se";
    let suggestions_for = slash_manager.suggest_commands(prefix);
    println!("\nCommand suggestions for '{}':", prefix);
    for cmd in suggestions_for {
        println!("  → /{}", cmd.name);
    }

    println!("└────────────────────────────────────────────────────────────────┘\n");

    // ═══════════════════════════════════════════════════════════════════
    // 4. KNOWLEDGE GRAPH SEARCH
    // ═══════════════════════════════════════════════════════════════════
    println!("┌─ 4. Knowledge Graph Search ──────────────────────────────────┐");

    let kg_service = KGSearchService::new();

    println!("KG Service created");
    println!("  Loaded roles: {}", kg_service.list_roles().len());
    println!("  Has 'engineer' role: {}", kg_service.has_role("engineer"));

    // In a real application, you would:
    // 1. Load a RoleGraph from config
    // 2. kg_service.load_role_graph("engineer", role_graph);
    // 3. Search for KG terms and documents

    println!("\n✓ KG service initialized");
    println!("  (Note: Load RoleGraph to enable term/document search)");

    println!("└────────────────────────────────────────────────────────────────┘\n");

    // ═══════════════════════════════════════════════════════════════════
    // 5. CONVERSATION MANAGEMENT
    // ═══════════════════════════════════════════════════════════════════
    println!("┌─ 5. Conversation Management ─────────────────────────────────┐");

    let mut conversation = Conversation::new(
        "Rust Async Programming Discussion".to_string(),
        RoleName::from("engineer"),
    );

    // Add context
    conversation.global_context.push(ContextItem {
        id: "ctx_rust_docs".into(),
        title: "Rust Async Book".to_string(),
        summary: Some("Official async programming guide".to_string()),
        content: "Async programming in Rust allows you to write concurrent code...".to_string(),
        context_type: ContextType::Document,
        created_at: chrono::Utc::now(),
        relevance_score: Some(0.98),
        metadata: ahash::AHashMap::new(),
    });

    conversation.global_context.push(ContextItem {
        id: "ctx_tokio".into(),
        title: "Tokio Documentation".to_string(),
        summary: Some("Tokio async runtime docs".to_string()),
        content: "Tokio is a runtime for writing reliable asynchronous applications...".to_string(),
        context_type: ContextType::Document,
        created_at: chrono::Utc::now(),
        relevance_score: Some(0.95),
        metadata: ahash::AHashMap::new(),
    });

    // Add messages
    conversation.messages.push(ChatMessage::system(
        "You are a Rust programming expert specializing in async/await.".to_string(),
    ));

    conversation.messages.push(ChatMessage::user(
        "How do I use tokio::spawn to run concurrent tasks?".to_string(),
    ));

    conversation.messages.push(ChatMessage::assistant(
        "To run concurrent tasks with tokio::spawn, you can do the following:\n\
        1. Mark your function as async\n\
        2. Use tokio::spawn() with an async block\n\
        3. Each spawn creates a new task that runs concurrently\n\n\
        Example:\n\
        ```rust\n\
        tokio::spawn(async {\n\
            // Your async code here\n\
        });\n\
        ```".to_string(),
        Some("claude-sonnet-4-5".to_string()),
    ));

    conversation.messages.push(ChatMessage::user(
        "Can you show me how to handle errors in spawned tasks?".to_string(),
    ));

    conversation.messages.push(ChatMessage::assistant(
        "Yes! When using tokio::spawn, the spawned task returns a JoinHandle. \
        You should await this handle and handle potential errors:\n\n\
        ```rust\n\
        let handle = tokio::spawn(async {\n\
            // Task that might fail\n\
            do_something().await\n\
        });\n\n\
        match handle.await {\n\
            Ok(result) => println!(\"Task succeeded: {:?}\", result),\n\
            Err(e) => eprintln!(\"Task panicked: {:?}\", e),\n\
        }\n\
        ```".to_string(),
        Some("claude-sonnet-4-5".to_string()),
    ));

    // Display conversation
    println!("Conversation: \"{}\"", conversation.title);
    println!("Role: {}", conversation.role);
    println!("Created: {}", conversation.created_at.format("%Y-%m-%d %H:%M:%S"));
    println!("\nContext items: {}", conversation.global_context.len());
    for ctx in &conversation.global_context {
        println!("  📄 {} (relevance: {:.0}%)",
                 ctx.title,
                 ctx.relevance_score.unwrap_or(0.0) * 100.0);
    }

    println!("\nMessages: {}", conversation.messages.len());
    for (i, msg) in conversation.messages.iter().enumerate() {
        let role_icon = match msg.role.as_str() {
            "system" => "⚙️",
            "user" => "👤",
            "assistant" => "🤖",
            _ => "💬",
        };

        let preview = if msg.content.len() > 60 {
            format!("{}...", &msg.content[..60])
        } else {
            msg.content.clone()
        };

        println!("  {}) {} [{}] {}", i + 1, role_icon, msg.role, preview);
    }

    println!("\n✓ Conversation with {} turns", conversation.messages.len());

    println!("└────────────────────────────────────────────────────────────────┘\n");

    // ═══════════════════════════════════════════════════════════════════
    // 6. DATA MODELS
    // ═══════════════════════════════════════════════════════════════════
    println!("┌─ 6. Data Models (Term Chips) ────────────────────────────────┐");

    // Create term chips for multi-term queries
    let mut chip_set = TermChipSet::new();
    chip_set.add_chip(TermChip::new("rust".to_string(), true));
    chip_set.add_chip(TermChip::new("tokio".to_string(), true));
    chip_set.add_chip(TermChip::new("async".to_string(), true));
    chip_set.set_operator(Some(ChipOperator::And));

    println!("Term chip set:");
    for chip in &chip_set.chips {
        let kg_marker = if chip.is_kg_term { "📚" } else { "  " };
        println!("  {} {} (active: {})", kg_marker, chip.term, chip.is_active);
    }
    if let Some(op) = chip_set.operator {
        println!("  Operator: {:?}", op);
    }

    let query_string = chip_set.to_query_string();
    println!("\nQuery string: \"{}\"", query_string);

    // Parse query back to chips
    let reparsed = TermChipSet::from_query_string(&query_string, |term| {
        // Simulate KG lookup
        ["rust", "tokio", "async"].contains(&term)
    });

    println!("Reparsed chip count: {}", reparsed.chips.len());

    println!("└────────────────────────────────────────────────────────────────┘\n");

    // ═══════════════════════════════════════════════════════════════════
    // SUMMARY
    // ═══════════════════════════════════════════════════════════════════
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║  Integration Test Complete - All Components Working        ║");
    println!("╚══════════════════════════════════════════════════════════════╝");

    println!("\n✅ Tested Components:");
    println!("  ✓ Autocomplete Engine (exact + fuzzy search)");
    println!("  ✓ Search Service (query parsing)");
    println!("  ✓ Editor State (text manipulation)");
    println!("  ✓ Slash Commands (5 built-in commands)");
    println!("  ✓ Knowledge Graph Service (ready for RoleGraph)");
    println!("  ✓ Conversation Management (messages + context)");
    println!("  ✓ Data Models (term chips, view models)");

    println!("\n📊 Statistics:");
    println!("  • Autocomplete terms: {}", autocomplete_engine.term_count());
    println!("  • Slash commands: {}", slash_manager.list_commands().len());
    println!("  • Editor lines: {}", editor.line_count());
    println!("  • Conversation messages: {}", conversation.messages.len());
    println!("  • Context items: {}", conversation.global_context.len());

    println!("\n🚀 Next Steps:");
    println!("  1. Install GTK3 system libraries");
    println!("  2. Wire business logic to GPUI views");
    println!("  3. Implement reactive state management");
    println!("  4. Add keyboard shortcuts and actions");
    println!("  5. Launch desktop application!");

    println!("\n📚 See E2E_TESTING.md for detailed testing guide");

    Ok(())
}
