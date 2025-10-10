//! Specialized Agents Example
//!
//! This example demonstrates how to use the new SummarizationAgent and ChatAgent
//! that leverage the generic LLM interface instead of OpenRouter-specific code.

use terraphim_multi_agent::{
    ChatAgent, ChatConfig, SummarizationAgent, SummarizationConfig, SummaryStyle, test_utils,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🤖 Specialized Agents Example - Generic LLM Interface");
    println!("====================================================");

    // Example 1: SummarizationAgent
    println!("\n📝 Example 1: SummarizationAgent");
    println!("=================================");

    let base_agent = test_utils::create_test_agent().await?;
    let config = SummarizationConfig {
        max_summary_words: 100,
        summary_style: SummaryStyle::Brief,
        include_quotes: false,
        focus_areas: vec!["technology".to_string(), "innovation".to_string()],
    };

    let summarization_agent = SummarizationAgent::new(base_agent, Some(config)).await?;
    println!(
        "✅ Created SummarizationAgent with provider: {}",
        summarization_agent.llm_client().provider()
    );

    let sample_article = r#"
        Artificial Intelligence (AI) has revolutionized numerous industries over the past decade. 
        From healthcare to finance, AI technologies are being deployed to automate processes, 
        enhance decision-making, and improve efficiency. Machine learning algorithms can now 
        diagnose diseases with remarkable accuracy, while natural language processing has enabled 
        sophisticated chatbots and virtual assistants. The rapid advancement in AI has also 
        raised important questions about ethics, privacy, and the future of work. As we move 
        forward, it's crucial to develop AI systems that are not only powerful but also 
        responsible and aligned with human values.
    "#;

    println!(
        "📄 Original article length: {} characters",
        sample_article.len()
    );

    match summarization_agent.summarize(sample_article).await {
        Ok(summary) => {
            println!("✅ Generated summary ({} characters):", summary.len());
            println!("   {}", summary);
        }
        Err(e) => {
            println!("❌ Summarization failed: {}", e);
            println!("💡 Note: This requires Ollama to be running with gemma3:270m model");
        }
    }

    // Example 2: ChatAgent
    println!("\n💬 Example 2: ChatAgent");
    println!("========================");

    let base_agent2 = test_utils::create_test_agent().await?;
    let chat_config = ChatConfig {
        max_context_messages: 10,
        system_prompt: Some(
            "You are a helpful AI assistant specialized in technology topics.".to_string(),
        ),
        temperature: 0.7,
        max_response_tokens: 200,
        enable_context_summarization: true,
    };

    let mut chat_agent = ChatAgent::new(base_agent2, Some(chat_config)).await?;
    println!(
        "✅ Created ChatAgent with provider: {}",
        chat_agent.llm_client().provider()
    );

    // Start a conversation
    let session_id = chat_agent.start_new_session();
    println!("📝 Started new chat session: {}", session_id);

    let questions = vec![
        "What is Rust programming language?",
        "How does Rust compare to Python for system programming?",
        "What are the main benefits of using Rust?",
    ];

    for (i, question) in questions.iter().enumerate() {
        println!("\n👤 User: {}", question);
        match chat_agent.chat(question.to_string()).await {
            Ok(response) => {
                println!("🤖 Assistant: {}", response);

                if let Some(session) = chat_agent.get_chat_history() {
                    println!("   💾 Session has {} messages", session.messages.len());
                }
            }
            Err(e) => {
                println!("❌ Chat failed: {}", e);
                println!("💡 Note: This requires Ollama to be running with gemma3:270m model");
                break;
            }
        }
    }

    // Example 3: Multi-document summarization
    println!("\n📚 Example 3: Multi-Document Summarization");
    println!("==========================================");

    let documents = vec![
        (
            "AI in Healthcare",
            "AI is transforming healthcare through improved diagnostics, personalized treatment plans, and drug discovery. Machine learning models can analyze medical images with high accuracy and identify patterns that human doctors might miss.",
        ),
        (
            "AI in Finance",
            "The financial sector leverages AI for fraud detection, algorithmic trading, risk assessment, and customer service automation. AI systems can process vast amounts of financial data in real-time to make split-second decisions.",
        ),
        (
            "AI Ethics",
            "As AI becomes more prevalent, questions about bias, fairness, transparency, and accountability become increasingly important. Developing ethical AI requires careful consideration of how these systems impact different groups of people.",
        ),
    ];

    match summarization_agent.summarize_multiple(&documents).await {
        Ok(consolidated_summary) => {
            println!("✅ Consolidated summary:");
            println!("   {}", consolidated_summary);
        }
        Err(e) => {
            println!("❌ Multi-document summarization failed: {}", e);
            println!("💡 Note: This requires Ollama to be running with gemma3:270m model");
        }
    }

    println!("\n🎉 Specialized agents example completed!");
    println!("💡 All agents use the generic LLM interface, supporting multiple providers:");
    println!("   - Ollama (local models like gemma3:270m)");
    println!("   - OpenAI (with API key)");
    println!("   - Anthropic (with API key)");
    println!("   - Future providers can be easily added");

    Ok(())
}
