use chrono::Utc;
use terraphim_orchestrator::agent_run_record::{AgentRunRecord, ExitClass, RunTrigger};
use uuid::Uuid;

fn make_record(name: &str, exit_class: ExitClass, consecutive: u32) -> AgentRunRecord {
    AgentRunRecord {
        run_id: Uuid::new_v4(),
        agent_name: name.to_string(),
        started_at: Utc::now(),
        ended_at: Utc::now(),
        exit_code: Some(1),
        exit_class,
        model_used: None,
        was_fallback: false,
        wall_time_secs: 1.0,
        output_summary: String::new(),
        error_summary: String::new(),
        trigger: RunTrigger::Cron,
        matched_patterns: vec![],
        confidence: 0.9,
        mention_chain_id: None,
        mention_depth: None,
        mention_parent_agent: None,
        consecutive_config_errors: consecutive,
    }
}

#[test]
fn should_quarantine_after_three_consecutive_config_errors() {
    let r0 = make_record("bad-agent", ExitClass::ConfigError, 0);
    assert!(
        !r0.should_quarantine(),
        "0 consecutive should not quarantine"
    );

    let r2 = make_record("bad-agent", ExitClass::ConfigError, 2);
    assert!(
        !r2.should_quarantine(),
        "2 consecutive should not quarantine"
    );

    let r3 = make_record("bad-agent", ExitClass::ConfigError, 3);
    assert!(r3.should_quarantine(), "3 consecutive should quarantine");

    let r4 = make_record("bad-agent", ExitClass::ConfigError, 4);
    assert!(r4.should_quarantine(), "4 consecutive should quarantine");
}

#[test]
fn non_config_error_resets_counter() {
    // A successful run should not trigger quarantine regardless of field value
    let success = make_record("ok-agent", ExitClass::Success, 0);
    assert!(!success.should_quarantine());

    // A record with consecutive=0 after any non-ConfigError should not quarantine
    let timeout = make_record("ok-agent", ExitClass::Timeout, 0);
    assert!(!timeout.should_quarantine());
}

#[test]
fn consecutive_config_errors_serde_default() {
    // When deserialising a record that predates this field, it should default to 0
    let json = r#"{
        "run_id": "00000000-0000-0000-0000-000000000000",
        "agent_name": "legacy-agent",
        "started_at": "2026-01-01T00:00:00Z",
        "ended_at": "2026-01-01T00:01:00Z",
        "exit_code": 0,
        "exit_class": "success",
        "model_used": null,
        "was_fallback": false,
        "wall_time_secs": 60.0,
        "output_summary": "",
        "error_summary": "",
        "trigger": "cron",
        "matched_patterns": [],
        "confidence": 1.0,
        "mention_chain_id": null,
        "mention_depth": null,
        "mention_parent_agent": null
    }"#;
    let record: AgentRunRecord = serde_json::from_str(json).expect("deserialise legacy record");
    assert_eq!(record.consecutive_config_errors, 0);
    assert!(!record.should_quarantine());
}
