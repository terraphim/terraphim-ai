use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use terraphim_persistence::{Persistable, Result};

// Import multi-agent system for enhanced persistence capabilities
use std::sync::Arc;
use terraphim_multi_agent::{
    test_utils::create_test_role, CommandInput, CommandType, TerraphimAgent,
};
use terraphim_persistence::DeviceStorage;

/// Enhanced struct that can work with both traditional persistence and multi-agent system
#[derive(Debug, Clone, Serialize, Deserialize)]
struct MyStruct {
    name: String,
    age: u8,
    /// Additional metadata that can be utilized by intelligent agents
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tags: Option<Vec<String>>,
}
#[async_trait]
impl Persistable for MyStruct {
    fn new(name: String) -> Self {
        MyStruct {
            name,
            age: 0,
            description: None,
            tags: None,
        }
    }

    async fn save_to_one(&self, profile_name: &str) -> Result<()> {
        self.save_to_profile(profile_name).await?;
        Ok(())
    }

    // saves to all profiles
    async fn save(&self) -> Result<()> {
        let _op = &self.load_config().await?.1;
        let _ = self.save_to_all().await?;
        Ok(())
    }

    async fn load(&mut self) -> Result<Self> {
        let op = &self.load_config().await?.1;
        let key = self.get_key();
        let obj = self.load_from_operator(&key, op).await?;
        Ok(obj)
    }

    fn get_key(&self) -> String {
        self.name.clone()
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let _ = tracing_subscriber::fmt().with_env_filter("info").try_init();

    println!("🚀 Enhanced Persistence with Multi-Agent Intelligence");
    println!("====================================================");

    // Example 1: Traditional Persistence
    println!("\n📋 Example 1: Traditional Persistence");
    println!("====================================");

    let obj = MyStruct {
        name: "No vampire".to_string(),
        age: 110,
        description: Some("A mysterious individual with enhanced longevity".to_string()),
        tags: Some(vec!["supernatural".to_string(), "longevity".to_string()]),
    };
    obj.save_to_one("s3").await?;
    obj.save().await?;
    println!("saved obj: {:?} to all", obj);
    let (_ops, fastest_op) = obj.load_config().await?;
    println!("fastest_op: {:#?}", fastest_op);

    let mut obj1 = MyStruct::new("obj".to_string());
    let key = obj.get_key();
    println!("key: {}", key);
    obj1 = obj1.load().await?;
    println!("loaded obj: {:?}", obj1);

    // Example 2: Multi-Agent Enhanced Persistence
    {
        println!("\n🤖 Example 2: Multi-Agent Enhanced Persistence");
        println!("=============================================");

        // Initialize storage for multi-agent system
        DeviceStorage::init_memory_only()
            .await
            .map_err(|e| terraphim_persistence::Error::Serde(e.to_string()))?;
        let storage_ref = DeviceStorage::instance()
            .await
            .map_err(|e| terraphim_persistence::Error::Serde(e.to_string()))?;

        use std::ptr;
        let storage_copy = unsafe { ptr::read(storage_ref) };
        let persistence = Arc::new(storage_copy);

        // Create intelligent agent for data management
        let role = create_test_role();
        let agent = TerraphimAgent::new(role, persistence, None)
            .await
            .map_err(|e| terraphim_persistence::Error::Serde(e.to_string()))?;
        agent
            .initialize()
            .await
            .map_err(|e| terraphim_persistence::Error::Serde(e.to_string()))?;

        println!("✅ Intelligent persistence agent created:");
        println!("   Agent ID: {}", agent.agent_id);
        println!("   Status: {:?}", agent.status);

        // Demonstrate intelligent data analysis
        let data_analysis_query = format!(
            "Analyze this data structure and suggest improvements: {:?}",
            obj
        );
        let input = CommandInput::new(data_analysis_query, CommandType::Analyze);
        let output = agent
            .process_command(input)
            .await
            .map_err(|e| terraphim_persistence::Error::Serde(e.to_string()))?;

        println!("\n🔍 Intelligent Data Analysis:");
        println!("   Query: Analyze data structure and suggest improvements");
        println!("   AI Response: {}", output.text);

        // Demonstrate intelligent data generation
        let generation_query = "Generate a similar data structure with different characteristics";
        let input = CommandInput::new(generation_query.to_string(), CommandType::Generate);
        let output = agent
            .process_command(input)
            .await
            .map_err(|e| terraphim_persistence::Error::Serde(e.to_string()))?;

        println!("\n🎯 Intelligent Data Generation:");
        println!("   Query: {}", generation_query);
        println!("   AI Response: {}", output.text);

        // Show tracking information
        let token_tracker = agent.token_tracker.read().await;
        let cost_tracker = agent.cost_tracker.read().await;

        println!("\n📊 Intelligence Tracking:");
        println!(
            "   Tokens: {} in / {} out",
            token_tracker.total_input_tokens, token_tracker.total_output_tokens
        );
        println!("   Cost: ${:.6}", cost_tracker.current_month_spending);
    }

    if false {
        println!("\n💡 Multi-Agent Enhanced Persistence Available");
        println!("=============================================");
        println!("To enable intelligent persistence capabilities:");
        println!("   1. Add 'multi_agent' feature flag");
        println!("   2. Combine traditional persistence with AI analysis");
        println!("   3. Get intelligent insights about your data structures");
        println!("   4. Generate enhanced data automatically");
        println!("\n   Example: cargo run --features multi_agent --example simple_struct");
    }

    println!("\n🎉 Enhanced Persistence Demo Complete!");
    println!("\n✅ Key Benefits:");
    println!("   • Traditional persistence with metadata enhancement");
    println!("   • AI-powered data structure analysis");
    println!("   • Intelligent data generation and suggestions");
    println!("   • Performance tracking for data operations");
    println!("   • Seamless integration with existing persistence patterns");

    Ok(())
}
