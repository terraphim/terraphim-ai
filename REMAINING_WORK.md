# RAG Workflow - Remaining Implementation

**Branch:** feature/rag-workflow-context-chat
**Issue:** #269
**Status:** 5 commits, ~60% complete

## What's Done ✅

1. **TuiService** - 17 RAG methods with persistence
2. **Command Enums** - ContextSubcommand, ConversationSubcommand
3. **Command Parsing** - /context and /conversation parsing
4. **Session State** - current_conversation_id, last_search_results
5. **Match Arms** - Routing to handlers

## What's Left (Est. 150 lines)

### 1. Implement handle_context()

Add after line 1467 in `handler.rs`:

```rust
#[cfg(feature = "repl-chat")]
async fn handle_context(&mut self, subcommand: ContextSubcommand) -> Result<()> {
    use colored::Colorize;

    let service = self.tui_service.as_ref()
        .ok_or_else(|| anyhow::anyhow!("No service available"))?;

    // Ensure conversation exists
    if self.current_conversation_id.is_none() {
        let title = format!("Session {}", chrono::Utc::now().format("%Y-%m-%d %H:%M"));
        let conv_id = service.create_conversation(title.clone()).await?;
        self.current_conversation_id = Some(conv_id);
        println!("📝 Created conversation: {}", title.green());
    }

    let conv_id = self.current_conversation_id.as_ref().unwrap();

    match subcommand {
        ContextSubcommand::Add { indices } => {
            // Parse indices: "1,2,3" or "1-5"
            let index_list = parse_indices(&indices)?;

            for idx in index_list {
                if let Some(doc) = self.last_search_results.get(idx) {
                    service.add_document_to_context(conv_id, doc).await?;
                    println!("✅ Added [{}]: {}", idx, doc.title.green());
                } else {
                    println!("⚠️  Index {} out of range (max: {})",
                        idx, self.last_search_results.len() - 1);
                }
            }
        }

        ContextSubcommand::List => {
            let items = service.list_context(conv_id).await?;
            if items.is_empty() {
                println!("No context items");
            } else {
                println!("Context items ({}):", items.len());
                for (i, item) in items.iter().enumerate() {
                    println!("  [{}] {} (score: {:?})",
                        format!("{:2}", i).yellow(),
                        item.title,
                        item.relevance_score
                    );
                }
            }
        }

        ContextSubcommand::Clear => {
            service.clear_context(conv_id).await?;
            println!("✅ Context cleared");
        }

        ContextSubcommand::Remove { index } => {
            let items = service.list_context(conv_id).await?;
            if let Some(item) = items.get(index) {
                service.remove_context_item(conv_id, &item.id).await?;
                println!("✅ Removed: {}", item.title);
            } else {
                println!("⚠️  Index {} out of range", index);
            }
        }
    }

    Ok(())
}

// Helper function
fn parse_indices(indices_str: &str) -> Result<Vec<usize>> {
    let mut result = Vec::new();
    for part in indices_str.split(',') {
        if part.contains('-') {
            // Range: "1-5"
            let range: Vec<&str> = part.split('-').collect();
            if range.len() == 2 {
                let start: usize = range[0].trim().parse()?;
                let end: usize = range[1].trim().parse()?;
                for i in start..=end {
                    result.push(i);
                }
            }
        } else {
            // Single index: "3"
            result.push(part.trim().parse()?);
        }
    }
    Ok(result)
}
```

### 2. Implement handle_conversation()

Add after handle_context():

```rust
#[cfg(feature = "repl-chat")]
async fn handle_conversation(&mut self, subcommand: ConversationSubcommand) -> Result<()> {
    use colored::Colorize;

    let service = self.tui_service.as_ref()
        .ok_or_else(|| anyhow::anyhow!("No service available"))?;

    match subcommand {
        ConversationSubcommand::New { title } => {
            let title = title.unwrap_or_else(||
                format!("Session {}", chrono::Utc::now().format("%Y-%m-%d %H:%M"))
            );
            let conv_id = service.create_conversation(title.clone()).await?;
            self.current_conversation_id = Some(conv_id.clone());
            println!("✅ Created conversation: {} (ID: {})",
                title.green(), conv_id.as_str().yellow());
        }

        ConversationSubcommand::Load { id } => {
            let conv_id = ConversationId::from_string(id.clone());
            let conv = service.load_conversation(&conv_id).await?;
            self.current_conversation_id = Some(conv_id);
            println!("✅ Loaded: {} ({} messages, {} context items)",
                conv.title.green(),
                conv.messages.len(),
                conv.global_context.len()
            );
        }

        ConversationSubcommand::List { limit } => {
            let summaries = service.list_conversations().await?;
            let display = if let Some(limit) = limit {
                &summaries[..limit.min(summaries.len())]
            } else {
                &summaries
            };

            if display.is_empty() {
                println!("No conversations");
            } else {
                println!("Conversations ({}):", summaries.len());
                for summary in display {
                    let marker = if Some(&summary.id) == self.current_conversation_id.as_ref() {
                        "▶".green()
                    } else {
                        " ".normal()
                    };
                    println!("  {} {} - {} ({} messages, {} context)",
                        marker,
                        summary.id.as_str().yellow(),
                        summary.title,
                        summary.message_count,
                        summary.context_count
                    );
                }
            }
        }

        ConversationSubcommand::Show => {
            if let Some(conv_id) = &self.current_conversation_id {
                if let Some(conv) = service.get_conversation(conv_id).await? {
                    println!("Conversation: {}", conv.title.green());
                    println!("ID: {}", conv.id.as_str().yellow());
                    println!("Role: {}", conv.role);
                    println!("Messages: {}", conv.messages.len());
                    println!("Context Items: {}", conv.global_context.len());
                } else {
                    println!("⚠️  Conversation not found in memory");
                }
            } else {
                println!("No active conversation. Use /conversation new to create one");
            }
        }

        ConversationSubcommand::Delete { id } => {
            let conv_id = ConversationId::from_string(id.clone());
            service.delete_conversation(&conv_id).await?;
            if Some(&conv_id) == self.current_conversation_id.as_ref() {
                self.current_conversation_id = None;
            }
            println!("✅ Deleted conversation: {}", id.yellow());
        }
    }

    Ok(())
}
```

### 3. Update handle_search()

Find handle_search() around line 540, update to store results and show indices:

```rust
// After getting results, before displaying:
#[cfg(feature = "repl-chat")]
{
    self.last_search_results = documents.clone();
}

// In display loop, add index:
for (i, doc) in documents.iter().enumerate() {
    let rank = doc.rank.unwrap_or(0);
    // ... existing display code
    println!("  [{}] {} - {}",
        format!("{:2}", i).yellow(),  // Add index
        rank,
        doc.title
    );
}

// After results:
#[cfg(feature = "repl-chat")]
if !documents.is_empty() {
    println!("\n💡 Use {} to add documents to context",
        "/context add <indices>".yellow()
    );
}
```

### 4. Update handle_chat()

Find handle_chat() around line 981, update to use context:

```rust
#[cfg(feature = "repl-chat")]
async fn handle_chat(&mut self, message: Option<String>) -> Result<()> {
    let service = self.tui_service.as_ref()
        .ok_or_else(|| anyhow::anyhow!("No service available"))?;

    let msg = message.unwrap_or_else(|| {
        print!("💬 Message: ");
        io::stdout().flush().unwrap();
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        input.trim().to_string()
    });

    if self.current_conversation_id.is_some() {
        // Use context-aware chat
        let conv_id = self.current_conversation_id.as_ref().unwrap();
        let response = service.chat_with_context(conv_id, msg, None).await?;
        println!("\n🤖 {}\n", "Response:".bold());
        println!("{}", response);
    } else {
        // Direct chat without context
        let role_name = terraphim_types::RoleName::new(&self.current_role);
        let response = service.chat(&role_name, &msg, None).await?;
        println!("\n🤖 {}\n", "Response:".bold());
        println!("{}", response);
    }

    Ok(())
}
```

## Build and Test

```bash
cargo build -p terraphim_tui --features repl-full
cargo test -p terraphim_tui --features repl-full
```

## Complete Example When Done

```bash
$ terraphim-tui repl

Terraphim Engineer> /search graph
✅ Found 36 result(s)
  [ 0] 43677 - @memory
  [ 1] 38308 - Architecture
  [ 2] 24464 - knowledge-graph
💡 Use /context add <indices> to add to context

Terraphim Engineer> /context add 1,2
📝 Created conversation: Session 2025-10-27 15:30
✅ Added [1]: Architecture
✅ Added [2]: knowledge-graph

Terraphim Engineer> /chat Explain the architecture
🤖 Response:
[Uses Architecture + knowledge-graph as context]

Terraphim Engineer> /conversation list
Conversations (1):
  ▶ conv-123 - Session 2025-10-27 15:30 (2 messages, 2 context)
```

## Merge Strategy

After completion:
1. Merge feature/rag-workflow-context-chat to fix/terraphim-tui-repl-full-build
2. Test combined functionality
3. Merge fix/terraphim-tui-repl-full-build to main

Total: 14 commits ready to merge
