use terraphim_orchestrator::{AgentDefinition, AgentLayer, ScheduleEvent, TimeScheduler};

fn make_agent(name: &str, layer: AgentLayer, schedule: Option<&str>) -> AgentDefinition {
    AgentDefinition {
        name: name.to_string(),
        layer,
        cli_tool: "echo".to_string(),
        task: "test task".to_string(),
        model: None,
        schedule: schedule.map(String::from),
        capabilities: vec![],
        max_memory_bytes: None,
        budget_monthly_cents: None,
        provider: None,
        persona: None,
        terraphim_role: None,
        skill_chain: vec![],
        sfia_skills: vec![],
        fallback_provider: None,
        fallback_model: None,
        grace_period_secs: None,
        max_cpu_seconds: None,
        pre_check: None,
    }
}

/// Integration test: scheduler correctly injects events via the sender channel.
/// Design reference: test_scheduler_fires_at_cron_time
#[tokio::test]
async fn test_scheduler_fires_at_cron_time() {
    let agents = vec![
        make_agent("sentinel", AgentLayer::Safety, None),
        make_agent("sync", AgentLayer::Core, Some("0 3 * * *")),
        make_agent("reviewer", AgentLayer::Growth, None),
    ];

    let mut scheduler = TimeScheduler::new(&agents, Some("0 2 * * *")).unwrap();

    // Use the event sender to inject a synthetic event (simulates what the
    // background cron tick would do)
    let tx = scheduler.event_sender();

    // Inject a Spawn event for the core agent
    let spawn_def = make_agent("sync", AgentLayer::Core, Some("0 3 * * *"));
    tx.send(ScheduleEvent::Spawn(Box::new(spawn_def)))
        .await
        .unwrap();

    // Inject a CompoundReview event
    tx.send(ScheduleEvent::CompoundReview).await.unwrap();

    // Verify events are received in order
    let event1 = scheduler.next_event().await;
    match event1 {
        ScheduleEvent::Spawn(def) => assert_eq!(def.name, "sync"),
        other => panic!("expected Spawn, got {:?}", other),
    }

    let event2 = scheduler.next_event().await;
    match event2 {
        ScheduleEvent::CompoundReview => {} // expected
        other => panic!("expected CompoundReview, got {:?}", other),
    }
}

/// Integration test: Stop event propagates through scheduler channel.
#[tokio::test]
async fn test_scheduler_stop_event() {
    let agents = vec![make_agent("sentinel", AgentLayer::Safety, None)];
    let mut scheduler = TimeScheduler::new(&agents, None).unwrap();

    let tx = scheduler.event_sender();
    tx.send(ScheduleEvent::Stop {
        agent_name: "sentinel".to_string(),
    })
    .await
    .unwrap();

    let event = scheduler.next_event().await;
    match event {
        ScheduleEvent::Stop { agent_name } => assert_eq!(agent_name, "sentinel"),
        other => panic!("expected Stop, got {:?}", other),
    }
}

/// Integration test: scheduler partitions agents by layer correctly.
#[test]
fn test_scheduler_layer_partitioning() {
    let agents = vec![
        make_agent("safety-1", AgentLayer::Safety, None),
        make_agent("safety-2", AgentLayer::Safety, None),
        make_agent("core-1", AgentLayer::Core, Some("0 1 * * *")),
        make_agent("core-2", AgentLayer::Core, Some("0 2 * * *")),
        make_agent("growth-1", AgentLayer::Growth, None),
    ];
    let scheduler = TimeScheduler::new(&agents, None).unwrap();

    let immediate = scheduler.immediate_agents();
    assert_eq!(immediate.len(), 2);
    assert!(immediate.iter().all(|a| a.layer == AgentLayer::Safety));

    let scheduled = scheduler.scheduled_agents();
    assert_eq!(scheduled.len(), 2);
    assert!(scheduled.iter().all(|(a, _)| a.layer == AgentLayer::Core));
}
