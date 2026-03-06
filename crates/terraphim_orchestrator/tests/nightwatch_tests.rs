use terraphim_orchestrator::{CorrectionLevel, NightwatchConfig, NightwatchMonitor};
use terraphim_spawner::health::HealthStatus;
use terraphim_spawner::output::OutputEvent;
use terraphim_types::capability::ProcessId;

/// Integration test: OutputEvents accumulate into drift metrics across module boundaries.
/// Design reference: test_nightwatch_accumulates_from_output
#[test]
fn test_nightwatch_accumulates_from_output() {
    let mut monitor = NightwatchMonitor::new(NightwatchConfig::default());
    let pid = ProcessId::new();

    // Feed a mix of stdout and stderr events
    for i in 0..80 {
        let event = OutputEvent::Stdout {
            process_id: pid.clone(),
            line: format!("output line {}", i),
        };
        monitor.observe("test-agent", &event);
    }
    for i in 0..20 {
        let event = OutputEvent::Stderr {
            process_id: pid.clone(),
            line: format!("error line {}", i),
        };
        monitor.observe("test-agent", &event);
    }

    // Feed health observations
    for _ in 0..8 {
        monitor.observe_health("test-agent", HealthStatus::Healthy);
    }
    for _ in 0..2 {
        monitor.observe_health("test-agent", HealthStatus::Degraded);
    }

    // Verify drift score is computed from accumulated data
    let ds = monitor.drift_score("test-agent").unwrap();
    assert!(ds.score > 0.0, "drift should be non-zero with errors");
    assert_eq!(ds.metrics.sample_count, 110); // 100 output lines + 10 health checks

    // error_rate = 20/100 = 0.20
    assert!((ds.metrics.error_rate - 0.20).abs() < 0.01);
    // health_score = 8/10 = 0.80
    assert!((ds.metrics.health_score - 0.80).abs() < 0.01);

    // With 20% error rate and 80% health, drift should be in Minor-Moderate range
    assert!(
        ds.level >= CorrectionLevel::Minor,
        "expected at least Minor drift"
    );
}

/// Integration test: multiple agents tracked independently.
#[test]
fn test_nightwatch_multi_agent_independent_tracking() {
    let mut monitor = NightwatchMonitor::new(NightwatchConfig::default());
    let pid = ProcessId::new();

    // Agent A: high error rate (80% errors)
    for _ in 0..80 {
        monitor.observe(
            "agent-a",
            &OutputEvent::Stderr {
                process_id: pid.clone(),
                line: "error".to_string(),
            },
        );
    }
    for _ in 0..20 {
        monitor.observe(
            "agent-a",
            &OutputEvent::Stdout {
                process_id: pid.clone(),
                line: "ok".to_string(),
            },
        );
    }

    // Agent B: low error rate (5% errors)
    for _ in 0..95 {
        monitor.observe(
            "agent-b",
            &OutputEvent::Stdout {
                process_id: pid.clone(),
                line: "ok".to_string(),
            },
        );
    }
    for _ in 0..5 {
        monitor.observe(
            "agent-b",
            &OutputEvent::Stderr {
                process_id: pid.clone(),
                line: "warning".to_string(),
            },
        );
    }
    monitor.observe_health("agent-b", HealthStatus::Healthy);

    // Verify independent tracking
    let all_scores = monitor.all_drift_scores();
    assert_eq!(all_scores.len(), 2);

    let score_a = monitor.drift_score("agent-a").unwrap();
    let score_b = monitor.drift_score("agent-b").unwrap();

    // Agent A should have much higher drift than Agent B
    assert!(
        score_a.score > score_b.score,
        "agent-a (high errors) should have higher drift than agent-b"
    );
    assert!(score_a.level >= CorrectionLevel::Severe);
    assert!(score_b.level <= CorrectionLevel::Minor);
}

/// Integration test: reset clears metrics for one agent without affecting others.
#[test]
fn test_nightwatch_reset_isolated_to_agent() {
    let mut monitor = NightwatchMonitor::new(NightwatchConfig::default());
    let pid = ProcessId::new();

    // Both agents accumulate errors
    for agent in &["agent-x", "agent-y"] {
        for _ in 0..50 {
            monitor.observe(
                agent,
                &OutputEvent::Stderr {
                    process_id: pid.clone(),
                    line: "error".to_string(),
                },
            );
        }
    }

    let before_x = monitor.drift_score("agent-x").unwrap().score;
    let before_y = monitor.drift_score("agent-y").unwrap().score;
    assert!(before_x > 0.5);
    assert!(before_y > 0.5);

    // Reset only agent-x
    monitor.reset("agent-x");

    let after_x = monitor.drift_score("agent-x").unwrap();
    let after_y = monitor.drift_score("agent-y").unwrap();

    assert!(after_x.score < f64::EPSILON, "agent-x should be reset");
    assert!(
        (after_y.score - before_y).abs() < f64::EPSILON,
        "agent-y should be unaffected"
    );
}
