//! Learning via Negativa: Command Correction Example
//!
//! This example demonstrates how Terraphim can learn from failed commands
//! and create a knowledge graph that helps correct common mistakes.
//!
//! ## The Problem
//!
//! Developers frequently make the same mistakes:
//! - Running `git push -f` when they meant `git push`
//! - Using `rm -rf *` in the wrong directory
//! - Typing `cargo run` instead of `cargo build`
//!
//! ## The Solution: Learning via Negativa
//!
//! 1. Capture failed commands with error context
//! 2. Create correction knowledge graph (wrong -> right)
//! 3. Use replace tool to suggest corrections

use terraphim_rolegraph::RoleGraph;
use terraphim_types::{
    Document, DocumentType, NormalizedTerm, NormalizedTermValue, RoleName, Thesaurus,
};

/// Represents a failed command with its correction
#[derive(Debug, Clone)]
struct FailedCommand {
    wrong_command: String,
    error_output: String,
    #[allow(dead_code)]
    correct_command: String,
    context: String,
}

/// Build knowledge graph for command corrections
fn build_correction_thesaurus() -> Thesaurus {
    let mut thesaurus = Thesaurus::new("Command Corrections".to_string());

    // Git corrections
    let corrections = vec![
        // Git force push corrections
        (
            "git push --force",
            vec![
                "git push -f",
                "git force push",
                "git push --force-with-lease",
            ],
            "git push",
            "Force pushing can overwrite remote changes. Always use --force-with-lease or avoid force push.",
        ),
        (
            "git push",
            vec!["git push origin main", "git push origin master"],
            "git push origin main:main",
            "Consider pushing with explicit remote and refspec.",
        ),
        // Git reset corrections
        (
            "git reset --soft HEAD~1",
            vec!["git reset --mixed", "git reset HEAD~1", "git undo"],
            "git reset --soft HEAD~1",
            "Use --soft to keep changes staged, --hard to discard.",
        ),
        // Cargo corrections
        (
            "cargo build",
            vec!["cargo compile", "cargo make"],
            "cargo build",
            "cargo build compiles the project. Use for verification.",
        ),
        (
            "cargo run",
            vec!["cargo execute", "cargo start"],
            "cargo run --release",
            "cargo run compiles and executes. Use --release for production.",
        ),
        // Docker corrections
        (
            "docker compose up",
            vec!["docker-compose up", "docker compose start"],
            "docker compose up -d",
            "Use -d for detached mode. docker-compose is deprecated.",
        ),
        // npm/yarn corrections
        (
            "yarn install",
            vec!["yarn add", "npm install"],
            "yarn install",
            "yarn install installs all dependencies from package.json.",
        ),
        // Rust toolchain
        (
            "rustup update",
            vec!["rustup upgrade", "rustup self update"],
            "rustup update",
            "Updates all installed Rust toolchains.",
        ),
    ];

    let mut id = 1u64;
    for (correct, wrong_aliases, _primary_wrong, _description) in corrections {
        // Add the correct command
        let term = NormalizedTerm::new(id, NormalizedTermValue::new(correct.to_string()));
        thesaurus.insert(NormalizedTermValue::new(correct.to_string()), term);

        // Add wrong aliases pointing to the correct one
        for wrong in wrong_aliases {
            let wrong_term = NormalizedTerm::new(id, NormalizedTermValue::new(correct.to_string()));
            thesaurus.insert(NormalizedTermValue::new(wrong.to_string()), wrong_term);
        }
        id += 1;
    }

    thesaurus
}

/// Build enhanced thesaurus with more command patterns
fn build_enhanced_correction_thesaurus() -> Thesaurus {
    let mut thesaurus = build_correction_thesaurus();

    let more_corrections = vec![
        // System commands
        (
            "sudo apt update && sudo apt upgrade",
            vec!["sudo apt-get update", "apt update", "apt upgrade"],
            "sudo apt update && sudo apt upgrade -y",
            "Update package lists and upgrade packages.",
        ),
        (
            "ps aux | grep",
            vec!["ps -a", "ps -x", "top"],
            "ps aux | grep pattern",
            "Find processes matching pattern.",
        ),
        (
            "kill -9",
            vec!["killall", "pkill -9"],
            "kill PID",
            "Use SIGKILL as last resort. Prefer graceful shutdown.",
        ),
        // Git more corrections
        (
            "git stash push -m",
            vec!["git stash save", "git stash"],
            "git stash push -m 'message'",
            "Always add a message to stashes for clarity.",
        ),
        (
            "git rebase -i HEAD~n",
            vec!["git rebase interactive", "git rebase"],
            "git rebase -i HEAD~n",
            "Interactive rebase for cleaning up commit history.",
        ),
        // Docker more
        (
            "docker system prune -a",
            vec!["docker clean", "docker prune"],
            "docker system prune -a --volumes",
            "Remove unused containers, networks, and images.",
        ),
    ];

    let mut id = 20u64;
    for (correct, wrong_aliases, _primary_wrong, _description) in more_corrections {
        let term = NormalizedTerm::new(id, NormalizedTermValue::new(correct.to_string()));
        thesaurus.insert(NormalizedTermValue::new(correct.to_string()), term);

        for wrong in wrong_aliases {
            let wrong_term = NormalizedTerm::new(id, NormalizedTermValue::new(correct.to_string()));
            thesaurus.insert(NormalizedTermValue::new(wrong.to_string()), wrong_term);
        }
        id += 1;
    }

    thesaurus
}

/// Create documents representing error contexts and corrections
fn create_correction_documents() -> Vec<Document> {
    vec![
        Document {
            id: "git_force_push".to_string(),
            title: "Git Force Push Correction".to_string(),
            url: "file:///learnings/git-force-push.md".to_string(),
            body: r#"
                # Git Force Push Warning

                Command: git push --force
                Error: remote: rejected - refusing to force push

                Why it fails:
                - Force push overwrites remote history
                - Can cause data loss for collaborators
                - Many repositories protect main branch

                Correct approach:
                - git push (normal push)
                - git push --force-with-lease (safer)
                - git push --force-if-includes (verify commits)
            "#
            .to_string(),
            description: Some("How to avoid force push errors".to_string()),
            rank: None,
            tags: Some(vec!["git".to_string(), "correction".to_string()]),
            source_haystack: None,
            doc_type: DocumentType::KgEntry,
            synonyms: None,
            route: None,
            priority: None,
            stub: None,
            summarization: None,
        },
        Document {
            id: "cargo_run_vs_build".to_string(),
            title: "Cargo Run vs Build".to_string(),
            url: "file:///learnings/cargo-run-build.md".to_string(),
            body: r#"
                # Cargo Run vs Build

                cargo run: Compiles AND executes the binary
                cargo build: Only compiles, does NOT execute

                When to use cargo build:
                - Just checking compilation
                - CI/CD pipelines
                - Release builds

                When to use cargo run:
                - Actually running the application
                - Development with hot reload
            "#
            .to_string(),
            description: Some("Understanding cargo run vs build".to_string()),
            rank: None,
            tags: Some(vec!["rust".to_string(), "correction".to_string()]),
            source_haystack: None,
            doc_type: DocumentType::KgEntry,
            synonyms: None,
            route: None,
            priority: None,
            stub: None,
            summarization: None,
        },
        Document {
            id: "docker_compose".to_string(),
            title: "Docker Compose Command".to_string(),
            url: "file:///learnings/docker-compose.md".to_string(),
            body: r#"
                # Docker Compose Commands

                Old: docker-compose up
                New: docker compose up

                The docker-compose command is deprecated.
                Use 'docker compose' (with space) instead.

                Best practices:
                - docker compose up -d (detached mode)
                - docker compose down (stop services)
                - docker compose logs -f (follow logs)
            "#
            .to_string(),
            description: Some("Docker compose command corrections".to_string()),
            rank: None,
            tags: Some(vec!["docker".to_string(), "correction".to_string()]),
            source_haystack: None,
            doc_type: DocumentType::KgEntry,
            synonyms: None,
            route: None,
            priority: None,
            stub: None,
            summarization: None,
        },
    ]
}

/// Simulate capturing a failed command
fn simulate_failed_command() -> FailedCommand {
    FailedCommand {
        wrong_command: "git push -f origin main".to_string(),
        error_output: "remote: error: denied by remote protection policy".to_string(),
        correct_command: "git push origin main".to_string(),
        context: "Trying to push to protected main branch".to_string(),
    }
}

/// Demonstrate the correction lookup
async fn demonstrate_correction(rolegraph: &RoleGraph, query: &str) {
    println!("\n--- Correction Lookup ---");
    println!("Query: '{}'", query);

    let results = rolegraph
        .query_graph(query, Some(0), Some(5))
        .unwrap_or_default();

    if results.is_empty() {
        println!("  No correction found");
    } else {
        for (i, (doc_id, indexed_doc)) in results.iter().enumerate() {
            println!("  {}. {} (rank: {})", i + 1, doc_id, indexed_doc.rank);
            println!("     Tags: {:?}", indexed_doc.tags);
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n================================================================================");
    println!("  Learning via Negativa: Command Correction with Terraphim");
    println!("================================================================================");

    // Create sample failed command
    let failed = simulate_failed_command();
    println!("\n📝 Captured Failed Command:");
    println!("   Wrong:   {}", failed.wrong_command);
    println!("   Error:   {}", failed.error_output);
    println!("   Context: {}", failed.context);

    // Build correction knowledge graphs
    let initial_thesaurus = build_correction_thesaurus();
    let enhanced_thesaurus = build_enhanced_correction_thesaurus();

    let initial_len = initial_thesaurus.len();
    let enhanced_len = enhanced_thesaurus.len();

    println!("\n📚 Thesaurus Comparison:");
    println!("   Initial:  {} terms", initial_len);
    println!(
        "   Enhanced: {} terms (+{})",
        enhanced_len,
        enhanced_len - initial_len
    );

    // Create role graphs
    let role_name = RoleName::new("Command Corrector");
    let mut initial_graph = RoleGraph::new(role_name.clone(), initial_thesaurus).await?;
    let mut enhanced_graph = RoleGraph::new(role_name, enhanced_thesaurus).await?;

    // Index documents
    let docs = create_correction_documents();
    println!("\n📄 Indexing {} correction documents...", docs.len());

    for doc in &docs {
        initial_graph.insert_document(&doc.id, doc.clone());
        enhanced_graph.insert_document(&doc.id, doc.clone());
    }

    // Demonstrate correction lookups
    println!("\n================================================================================");
    println!("  Correction Lookups (Learning via Negativa in Action)");
    println!("================================================================================");

    let test_queries = vec![
        "git push -f",
        "docker-compose up",
        "cargo run",
        "apt update",
    ];

    for query in test_queries {
        println!("\n--- Query: '{}' ---", query);

        // Use demonstrate_correction function
        demonstrate_correction(&initial_graph, query).await;
        demonstrate_correction(&enhanced_graph, query).await;

        // Initial
        let initial_results = initial_graph
            .query_graph(query, Some(0), Some(3))
            .unwrap_or_default();
        if initial_results.is_empty() {
            println!("  Initial: No correction found");
        } else {
            println!("  Initial: {} results", initial_results.len());
            for (doc_id, _doc) in initial_results.iter().take(1) {
                println!("    -> {}", doc_id);
            }
        }

        // Enhanced
        let enhanced_results = enhanced_graph
            .query_graph(query, Some(0), Some(3))
            .unwrap_or_default();
        if enhanced_results.is_empty() {
            println!("  Enhanced: No correction found");
        } else {
            println!("  Enhanced: {} results", enhanced_results.len());
            for (doc_id, doc) in enhanced_results.iter().take(1) {
                println!("    -> {} (rank: {})", doc_id, doc.rank);
            }
        }

        // Show improvement
        if enhanced_results.len() > initial_results.len() {
            println!("  ✓ Enhanced finds MORE corrections!");
        } else if !enhanced_results.is_empty() && initial_results.is_empty() {
            println!("  ✓ Enhanced ENABLES correction lookup!");
        }
    }

    // Summary
    println!("\n================================================================================");
    println!("  Summary: Learning via Negativa");
    println!("================================================================================");

    println!("\n✅ How It Works:");
    println!("   1. Capture failed commands with error context");
    println!("   2. Build knowledge graph mapping wrong -> correct");
    println!("   3. When similar query is made, suggest corrections");

    println!("\n📈 Results:");
    println!("   - With basic thesaurus: {} corrections", initial_len);
    println!("   - With enhanced thesaurus: {} corrections", enhanced_len);
    println!(
        "   - Improvement: +{} common mistakes covered",
        enhanced_len - initial_len
    );

    println!("\n🎯 Key Insight:");
    println!("   Adding command aliases ('git push -f', 'docker-compose up')");
    println!("   to the knowledge graph enables automatic correction suggestions.");
    println!("   This is \"learning via negativa\" - learning from mistakes!");

    Ok(())
}
