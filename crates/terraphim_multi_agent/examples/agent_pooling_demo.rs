//! Agent Pooling System Demonstration
//! 
//! This example demonstrates the advanced agent pooling system for TerraphimAgent,
//! showing how to optimize performance through intelligent agent reuse, load balancing,
//! and resource management.

use std::sync::Arc;
use std::time::Duration;
use tokio::time::{sleep, Instant};

use terraphim_multi_agent::{
    CommandInput, CommandType, LoadBalancingStrategy, PoolConfig, PoolManager,
    PoolManagerConfig, test_utils::create_test_role,
};
use terraphim_persistence::DeviceStorage;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    env_logger::init();
    
    println!("🚀 TerraphimAgent Pooling System Demo");
    println!("=====================================\n");

    // Initialize persistence
    DeviceStorage::init_memory_only().await?;
    let storage = {
        let storage_ref = DeviceStorage::instance().await?;
        use std::ptr;
        let storage_copy = unsafe { ptr::read(storage_ref) };
        Arc::new(storage_copy)
    };

    // Configure the pool manager with optimized settings
    let pool_config = PoolConfig {
        min_pool_size: 3,                                  // Start with 3 agents per pool
        max_pool_size: 8,                                  // Maximum 8 agents per pool
        max_idle_duration: Duration::from_secs(120),       // 2 minute idle timeout
        maintenance_interval: Duration::from_secs(30),     // 30 second maintenance
        max_concurrent_operations: 3,                      // 3 operations per agent
        agent_creation_timeout: Duration::from_secs(10),   // 10 second creation timeout
        enable_pool_warming: true,                         // Pre-warm pools
        load_balancing_strategy: LoadBalancingStrategy::LeastConnections,
    };

    let manager_config = PoolManagerConfig {
        default_pool_config: pool_config,
        max_pools: 5,                                      // Support 5 different roles
        create_pools_on_demand: true,                      // Create pools as needed
        cleanup_interval_seconds: 60,                      // Cleanup every minute
        pool_max_idle_duration_seconds: 300,              // Pool idle timeout: 5 min
    };

    // Create the pool manager
    println!("📦 Creating Pool Manager...");
    let pool_manager = PoolManager::new(storage, Some(manager_config)).await?;
    println!("✅ Pool Manager created successfully\n");

    // Demo 1: Basic Pool Usage
    println!("🎯 Demo 1: Basic Pool Usage");
    println!("---------------------------");
    demo_basic_pool_usage(&pool_manager).await?;
    
    // Demo 2: Load Balancing and Concurrency
    println!("\n⚖️  Demo 2: Load Balancing and Concurrency");
    println!("------------------------------------------");
    demo_load_balancing(&pool_manager).await?;
    
    // Demo 3: Pool Management and Statistics
    println!("\n📊 Demo 3: Pool Management and Statistics");
    println!("-----------------------------------------");
    demo_pool_management(&pool_manager).await?;
    
    // Demo 4: Performance Optimization
    println!("\n⚡ Demo 4: Performance Optimization");
    println!("-----------------------------------");
    demo_performance_optimization(&pool_manager).await?;
    
    // Demo 5: Multiple Role Types
    println!("\n🎭 Demo 5: Multiple Role Types");
    println!("------------------------------");
    demo_multiple_roles(&pool_manager).await?;

    // Final cleanup
    println!("\n🧹 Shutting down all pools...");
    pool_manager.shutdown_all().await?;
    println!("✅ All pools shut down successfully");

    println!("\n🎉 Agent Pooling Demo completed!");
    Ok(())
}

/// Demonstrate basic pool usage and agent lifecycle
async fn demo_basic_pool_usage(pool_manager: &PoolManager) -> Result<(), Box<dyn std::error::Error>> {
    let role = create_test_role();
    
    // Get initial statistics
    let initial_stats = pool_manager.get_global_stats().await;
    println!("📈 Initial stats: {} pools, {} agents", 
             initial_stats.total_pools, initial_stats.total_agents);

    // First command - should create a new pool
    println!("🔄 Executing first command (pool creation)...");
    let start_time = Instant::now();
    
    let command = CommandInput {
        command_type: CommandType::Generate,
        text: "Generate a brief summary of agent pooling benefits".to_string(),
        metadata: std::collections::HashMap::new(),
    };
    
    let result = pool_manager.execute_command(&role, command).await?;
    let duration = start_time.elapsed();
    
    println!("✅ Command completed in {:?}", duration);
    println!("📝 Response length: {} characters", result.text.len());

    // Check updated statistics
    let stats = pool_manager.get_global_stats().await;
    println!("📈 Updated stats: {} pools, {} agents", 
             stats.total_pools, stats.total_agents);
    println!("🎯 Pool hits: {}, misses: {}", stats.total_pool_hits, stats.total_pool_misses);

    // Second command - should reuse existing pool
    println!("🔄 Executing second command (pool reuse)...");
    let start_time = Instant::now();
    
    let command = CommandInput {
        command_type: CommandType::Answer,
        text: "What are the key benefits of agent pooling?".to_string(),
        metadata: std::collections::HashMap::new(),
    };
    
    let _result = pool_manager.execute_command(&role, command).await?;
    let duration = start_time.elapsed();
    
    println!("✅ Command completed in {:?} (faster due to pool reuse)", duration);

    // Final statistics
    let final_stats = pool_manager.get_global_stats().await;
    println!("📊 Final stats: {} operations, avg time: {:.2}ms", 
             final_stats.total_operations, final_stats.average_operation_time_ms);

    Ok(())
}

/// Demonstrate load balancing across multiple agents
async fn demo_load_balancing(pool_manager: &PoolManager) -> Result<(), Box<dyn std::error::Error>> {
    let role = create_test_role();
    
    println!("🔄 Executing 5 concurrent commands to test load balancing...");
    
    let mut tasks = Vec::new();
    let start_time = Instant::now();
    
    for i in 0..5 {
        let pm = pool_manager.clone();
        let role_clone = role.clone();
        
        let task = tokio::spawn(async move {
            let command = CommandInput {
                command_type: CommandType::Analyze,
                text: format!("Analyze the performance characteristics of operation #{}", i + 1),
                metadata: std::collections::HashMap::new(),
            };
            
            let task_start = Instant::now();
            let result = pm.execute_command(&role_clone, command).await;
            let task_duration = task_start.elapsed();
            
            (i + 1, result, task_duration)
        });
        
        tasks.push(task);
    }
    
    // Wait for all tasks to complete
    let results = futures::future::join_all(tasks).await;
    let total_duration = start_time.elapsed();
    
    println!("📊 Concurrent Execution Results:");
    for result in results {
        match result {
            Ok((id, Ok(_output), duration)) => {
                println!("  ✅ Operation #{}: completed in {:?}", id, duration);
            }
            Ok((id, Err(e), duration)) => {
                println!("  ❌ Operation #{}: failed in {:?} - {}", id, duration, e);
            }
            Err(e) => {
                println!("  💥 Task failed: {}", e);
            }
        }
    }
    
    println!("⏱️  Total execution time: {:?}", total_duration);
    println!("📈 Pool utilization demonstrates load balancing effectiveness");

    Ok(())
}

/// Demonstrate pool management and monitoring
async fn demo_pool_management(pool_manager: &PoolManager) -> Result<(), Box<dyn std::error::Error>> {
    // List all pools
    let pools = pool_manager.list_pools().await;
    println!("📋 Active pools ({})", pools.len());
    
    for pool_info in &pools {
        println!("  🏊 Pool: {}", pool_info.role_name);
        println!("    📅 Created: {}", pool_info.created_at.format("%H:%M:%S"));
        println!("    🕐 Last used: {}", pool_info.last_used.format("%H:%M:%S"));
        println!("    📊 Operations: {}", pool_info.stats.total_operations_processed);
        println!("    🎯 Current size: {}", pool_info.stats.current_pool_size);
        println!("    💼 Busy agents: {}", pool_info.stats.current_busy_agents);
        println!("    ⚡ Avg time: {:.2}ms", pool_info.stats.average_operation_time_ms);
    }

    // Get global statistics
    let global_stats = pool_manager.get_global_stats().await;
    println!("\n🌍 Global Statistics:");
    println!("  📦 Total pools: {}", global_stats.total_pools);
    println!("  🤖 Total agents: {}", global_stats.total_agents);
    println!("  🔄 Total operations: {}", global_stats.total_operations);
    println!("  ⏱️  Average operation time: {:.2}ms", global_stats.average_operation_time_ms);
    println!("  🎯 Pool hit rate: {:.1}%", 
             (global_stats.total_pool_hits as f64 / 
              (global_stats.total_pool_hits + global_stats.total_pool_misses) as f64) * 100.0);

    Ok(())
}

/// Demonstrate performance optimization through pooling
async fn demo_performance_optimization(pool_manager: &PoolManager) -> Result<(), Box<dyn std::error::Error>> {
    let role = create_test_role();
    
    println!("🔬 Testing cold start vs warm pool performance...");
    
    // Measure multiple operations to show pool warming effects
    let mut operation_times = Vec::new();
    
    for i in 0..10 {
        let start_time = Instant::now();
        
        let command = CommandInput {
            command_type: CommandType::Create,
            text: format!("Create a performance test response #{}", i + 1),
            metadata: std::collections::HashMap::new(),
        };
        
        let _result = pool_manager.execute_command(&role, command).await?;
        let duration = start_time.elapsed();
        
        operation_times.push(duration);
        
        if i < 3 {
            println!("  ⏱️  Operation #{}: {:?} (warm-up phase)", i + 1, duration);
        } else if i == 3 {
            println!("  🔥 Pool fully warmed up...");
        }
        
        // Small delay between operations
        sleep(Duration::from_millis(100)).await;
    }
    
    // Analyze performance improvements
    let early_avg: Duration = operation_times[0..3].iter().sum::<Duration>() / 3;
    let late_avg: Duration = operation_times[7..10].iter().sum::<Duration>() / 3;
    
    println!("📊 Performance Analysis:");
    println!("  🥶 Cold start average: {:?}", early_avg);
    println!("  🔥 Warm pool average: {:?}", late_avg);
    
    if late_avg < early_avg {
        let improvement = ((early_avg.as_millis() - late_avg.as_millis()) as f64 / 
                          early_avg.as_millis() as f64) * 100.0;
        println!("  ⚡ Performance improvement: {:.1}%", improvement);
    }

    Ok(())
}

/// Demonstrate multiple role types and pool isolation
async fn demo_multiple_roles(pool_manager: &PoolManager) -> Result<(), Box<dyn std::error::Error>> {
    // Create different role types
    let mut engineering_role = create_test_role();
    engineering_role.name = "Engineering Agent".into();
    
    let mut research_role = create_test_role();
    research_role.name = "Research Agent".into();
    
    let mut documentation_role = create_test_role();
    documentation_role.name = "Documentation Agent".into();
    
    println!("🎭 Testing multiple specialized agent roles...");
    
    // Execute commands for different roles
    let roles = vec![
        (&engineering_role, "Design a scalable microservice architecture"),
        (&research_role, "Research the latest trends in AI agent systems"),
        (&documentation_role, "Document the agent pooling system architecture"),
    ];
    
    for (role, task) in roles {
        let start_time = Instant::now();
        
        let command = CommandInput {
            command_type: CommandType::Generate,
            text: task.to_string(),
            metadata: std::collections::HashMap::new(),
        };
        
        let _result = pool_manager.execute_command(role, command).await?;
        let duration = start_time.elapsed();
        
        println!("  🎯 {}: completed in {:?}", role.name, duration);
    }
    
    // Show pool isolation
    let pools = pool_manager.list_pools().await;
    println!("\n🏊 Pool isolation demonstrated:");
    for pool_info in pools {
        if let Some(stats) = pool_manager.get_pool_stats(&pool_info.role_name).await {
            println!("  📦 {}: {} agents, {} operations", 
                     pool_info.role_name, 
                     stats.current_pool_size + stats.current_busy_agents,
                     stats.total_operations_processed);
        }
    }
    
    println!("✅ Each role maintains its own optimized agent pool");

    Ok(())
}

/// Helper trait to make PoolManager cloneable for demo purposes
impl Clone for PoolManager {
    fn clone(&self) -> Self {
        // This is a simplified clone for demo purposes
        // In production, you would typically use Arc<PoolManager>
        panic!("PoolManager clone not implemented for production use")
    }
}