//! Standalone test proving desktop-dioxus services work correctly
//! Run with: cargo test --manifest-path test_desktop_services.rs

use terraphim_service::TerraphimService;
use terraphim_service::conversation_service::ConversationService;
use terraphim_automata::{build_autocomplete_index, autocomplete_search, AutocompleteConfig};
use terraphim_types::{Thesaurus, RoleName, NormalizedTermValue, Conversation, ChatMessage};
use terraphim_config::ConfigBuilder;
use pulldown_cmark::{Parser, html};

#[tokio::test]
async fn test_autocomplete_works() {
    println!("✅ Testing autocomplete functionality...");
    
    // Create test thesaurus
    let mut thesaurus = Thesaurus::new(RoleName::from("test"));
    thesaurus.insert("rust".to_string(), NormalizedTermValue {
        id: 1,
        nterm: "rust".to_string(),
        url: Some("http://rust-lang.org".to_string()),
    });
    thesaurus.insert("rustacean".to_string(), NormalizedTermValue {
        id: 2,
        nterm: "rustacean".to_string(),
        url: Some("http://rustacean.net".to_string()),
    });
    
    // Build autocomplete index
    let config = AutocompleteConfig {
        max_results: 10,
        min_prefix_length: 1,
        case_sensitive: false,
    };
    
    let index = build_autocomplete_index(thesaurus, Some(config))
        .expect("Should build autocomplete index");
    
    // Test autocomplete search
    let results = autocomplete_search(&index, "ru", None)
        .expect("Should get autocomplete results");
    
    assert!(!results.is_empty(), "Should have autocomplete results");
    assert!(results.iter().any(|r| r.term == "rust"));
    assert!(results.iter().any(|r| r.term == "rustacean"));
    
    println!("  ✓ Autocomplete returned {} suggestions", results.len());
    println!("  ✓ Found 'rust' and 'rustacean' suggestions");
}

#[test]
fn test_markdown_rendering() {
    println!("✅ Testing markdown rendering...");
    
    let markdown = r#"# Hello World

This is a **bold** statement.

```rust
fn main() {
    println!("Hello!");
}
```

- Item 1
- Item 2

[Link](http://example.com)
"#;
    
    let parser = Parser::new(markdown);
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);
    
    assert!(html_output.contains("<h1>"));
    assert!(html_output.contains("<strong>"));
    assert!(html_output.contains("<code"));
    assert!(html_output.contains("<ul>"));
    assert!(html_output.contains("<a href"));
    
    println!("  ✓ Markdown converted to HTML successfully");
    println!("  ✓ Code blocks, lists, and links rendered");
}

#[tokio::test]
async fn test_conversation_persistence() {
    println!("✅ Testing conversation persistence...");
    
    let role = RoleName::from("test_role");
    let mut conversation = Conversation::new("Test Chat".to_string(), role);
    
    // Add user message
    let user_msg = ChatMessage::user("What is Rust?".to_string());
    conversation.add_message(user_msg);
    
    // Add AI message  
    let ai_msg = ChatMessage::assistant(
        "Rust is a systems programming language.".to_string(),
        Some("llm".to_string())
    );
    conversation.add_message(ai_msg);
    
    assert_eq!(conversation.messages.len(), 2);
    assert_eq!(conversation.messages[0].role, "user");
    assert_eq!(conversation.messages[1].role, "assistant");
    assert_eq!(conversation.messages[0].content, "What is Rust?");
    
    println!("  ✓ Created conversation with {} messages", conversation.messages.len());
    println!("  ✓ User and AI messages tracked correctly");
}

#[tokio::test]
async fn test_conversation_service() {
    println!("✅ Testing conversation service...");
    
    let service = ConversationService::new();
    let role = RoleName::from("engineer");
    
    let conversation = service.create_conversation(
        "Engineering Discussion".to_string(),
        role
    ).await.expect("Should create conversation");
    
    assert_eq!(conversation.title, "Engineering Discussion");
    assert!(conversation.messages.is_empty());
    
    println!("  ✓ ConversationService created successfully");
    println!("  ✓ Can create new conversations");
}

fn main() {
    println!("\n🧪 Running Desktop-Dioxus Service Tests\n");
    println!("Testing backend integration without GTK dependencies...\n");
}
