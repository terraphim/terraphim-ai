//! Integration tests for the messaging system

use std::time::Duration;

use serde_json::json;
use tokio::time::sleep;

use terraphim_agent_messaging::{
    AgentMessage, AgentPid, DeliveryConfig, DeliveryGuarantee, DeliveryOptions, MessageEnvelope,
    MessagePriority, MessageSystem, RouterConfig,
};

#[tokio::test]
async fn test_basic_message_flow() {
    env_logger::try_init().ok();

    let config = RouterConfig::default();
    let system = MessageSystem::new(config);

    let agent1 = AgentPid::new();
    let agent2 = AgentPid::new();

    // Register agents
    system.register_agent(agent1.clone()).await.unwrap();
    system.register_agent(agent2.clone()).await.unwrap();

    // Send message from agent1 to agent2
    let envelope = MessageEnvelope::new(
        agent2.clone(),
        "greeting".to_string(),
        json!({"message": "Hello, Agent2!"}),
        DeliveryOptions::default(),
    )
    .with_from(agent1.clone());

    system.send_message(envelope).await.unwrap();

    // Check delivery statistics
    let (router_stats, _mailbox_stats) = system.get_stats().await;
    assert_eq!(router_stats.messages_delivered, 1);
    assert_eq!(router_stats.messages_failed, 0);

    system.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_message_priorities() {
    env_logger::try_init().ok();

    let config = RouterConfig::default();
    let system = MessageSystem::new(config);

    let agent_id = AgentPid::new();
    system.register_agent(agent_id.clone()).await.unwrap();

    // Send messages with different priorities
    let priorities = vec![
        MessagePriority::Low,
        MessagePriority::Normal,
        MessagePriority::High,
        MessagePriority::Critical,
    ];

    for (i, priority) in priorities.into_iter().enumerate() {
        let mut options = DeliveryOptions::default();
        options.priority = priority;

        let envelope = MessageEnvelope::new(
            agent_id.clone(),
            format!("message_{}", i),
            json!({"priority": format!("{:?}", options.priority)}),
            options,
        );

        system.send_message(envelope).await.unwrap();
    }

    // All messages should be delivered
    let (router_stats, _) = system.get_stats().await;
    assert_eq!(router_stats.messages_delivered, 4);

    system.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_delivery_guarantees() {
    env_logger::try_init().ok();

    // Test At-Most-Once delivery
    let mut config = RouterConfig::default();
    config.delivery_config.guarantee = DeliveryGuarantee::AtMostOnce;

    let system = MessageSystem::new(config);
    let agent_id = AgentPid::new();

    system.register_agent(agent_id.clone()).await.unwrap();

    let envelope = MessageEnvelope::new(
        agent_id.clone(),
        "test_message".to_string(),
        json!({"data": "test"}),
        DeliveryOptions::default(),
    );

    system.send_message(envelope).await.unwrap();

    let (router_stats, _) = system.get_stats().await;
    assert_eq!(router_stats.messages_delivered, 1);

    system.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_message_routing_failure() {
    env_logger::try_init().ok();

    let config = RouterConfig::default();
    let system = MessageSystem::new(config);

    let non_existent_agent = AgentPid::new();

    // Try to send message to non-existent agent
    let envelope = MessageEnvelope::new(
        non_existent_agent,
        "test_message".to_string(),
        json!({"data": "test"}),
        DeliveryOptions::default(),
    );

    let result = system.send_message(envelope).await;
    assert!(result.is_err());

    system.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_agent_registration_lifecycle() {
    env_logger::try_init().ok();

    let config = RouterConfig::default();
    let system = MessageSystem::new(config);

    let agent_id = AgentPid::new();

    // Register agent
    system.register_agent(agent_id.clone()).await.unwrap();

    // Check stats
    let (router_stats, mailbox_stats) = system.get_stats().await;
    assert_eq!(router_stats.active_routes, 1);
    assert_eq!(mailbox_stats.len(), 1);

    // Unregister agent
    system.unregister_agent(&agent_id).await.unwrap();

    // Check stats after unregistration
    let (router_stats, mailbox_stats) = system.get_stats().await;
    assert_eq!(router_stats.active_routes, 0);
    assert_eq!(mailbox_stats.len(), 0);

    system.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_multiple_agents_communication() {
    env_logger::try_init().ok();

    let config = RouterConfig::default();
    let system = MessageSystem::new(config);

    // Create multiple agents
    let agents: Vec<AgentPid> = (0..5).map(|_| AgentPid::new()).collect();

    // Register all agents
    for agent in &agents {
        system.register_agent(agent.clone()).await.unwrap();
    }

    // Send messages between agents
    for (i, sender) in agents.iter().enumerate() {
        for (j, receiver) in agents.iter().enumerate() {
            if i != j {
                let envelope = MessageEnvelope::new(
                    receiver.clone(),
                    "peer_message".to_string(),
                    json!({
                        "from": format!("agent_{}", i),
                        "to": format!("agent_{}", j),
                        "message": "Hello peer!"
                    }),
                    DeliveryOptions::default(),
                )
                .with_from(sender.clone());

                system.send_message(envelope).await.unwrap();
            }
        }
    }

    // Check that all messages were delivered
    // 5 agents * 4 other agents = 20 messages
    let (router_stats, _) = system.get_stats().await;
    assert_eq!(router_stats.messages_delivered, 20);
    assert_eq!(router_stats.messages_failed, 0);

    system.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_message_timeout_and_retry() {
    env_logger::try_init().ok();

    let mut config = RouterConfig::default();
    config.delivery_config.guarantee = DeliveryGuarantee::AtLeastOnce;
    config.delivery_config.max_retries = 2;
    config.retry_interval = Duration::from_millis(100);

    let system = MessageSystem::new(config);
    let agent_id = AgentPid::new();

    system.register_agent(agent_id.clone()).await.unwrap();

    // Send a message
    let envelope = MessageEnvelope::new(
        agent_id.clone(),
        "test_message".to_string(),
        json!({"data": "test"}),
        DeliveryOptions::default(),
    );

    system.send_message(envelope).await.unwrap();

    // Wait a bit for potential retries
    sleep(Duration::from_millis(500)).await;

    // Message should be delivered successfully
    let (router_stats, _) = system.get_stats().await;
    assert!(router_stats.messages_delivered >= 1);

    system.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_system_shutdown() {
    env_logger::try_init().ok();

    let config = RouterConfig::default();
    let system = MessageSystem::new(config);

    let agent_id = AgentPid::new();
    system.register_agent(agent_id.clone()).await.unwrap();

    // Send a message
    let envelope = MessageEnvelope::new(
        agent_id.clone(),
        "test_message".to_string(),
        json!({"data": "test"}),
        DeliveryOptions::default(),
    );

    system.send_message(envelope).await.unwrap();

    // Shutdown system
    system.shutdown().await.unwrap();

    // Stats should be reset
    let (router_stats, mailbox_stats) = system.get_stats().await;
    assert_eq!(router_stats.active_routes, 0);
    assert_eq!(mailbox_stats.len(), 0);
}

#[tokio::test]
async fn test_high_throughput_messaging() {
    env_logger::try_init().ok();

    let config = RouterConfig::default();
    let system = MessageSystem::new(config);

    let sender_agent = AgentPid::new();
    let receiver_agent = AgentPid::new();

    system.register_agent(sender_agent.clone()).await.unwrap();
    system.register_agent(receiver_agent.clone()).await.unwrap();

    // Send many messages quickly
    let message_count = 100;
    for i in 0..message_count {
        let envelope = MessageEnvelope::new(
            receiver_agent.clone(),
            "bulk_message".to_string(),
            json!({"sequence": i, "data": format!("message_{}", i)}),
            DeliveryOptions::default(),
        )
        .with_from(sender_agent.clone());

        system.send_message(envelope).await.unwrap();
    }

    // All messages should be delivered
    let (router_stats, _) = system.get_stats().await;
    assert_eq!(router_stats.messages_delivered, message_count);
    assert_eq!(router_stats.messages_failed, 0);

    system.shutdown().await.unwrap();
}
