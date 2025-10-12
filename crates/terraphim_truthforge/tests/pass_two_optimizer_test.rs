use chrono::Utc;
use terraphim_truthforge::{
    AudienceType, NarrativeContext, NarrativeInput, PassOneOrchestrator, PassTwoOptimizer,
    StakeType, TwoPassDebateWorkflow, UrgencyLevel,
};
use uuid::Uuid;

#[tokio::test]
async fn test_pass_two_optimizer_executes() {
    let session_id = Uuid::new_v4();

    let narrative = NarrativeInput {
        session_id,
        text: "We reduced costs by 40%. Shareholders benefited greatly.".to_string(),
        context: NarrativeContext {
            urgency: UrgencyLevel::High,
            stakes: vec![StakeType::Reputational, StakeType::Financial],
            audience: AudienceType::PublicMedia,
        },
        submitted_at: Utc::now(),
    };

    let pass_one = PassOneOrchestrator::new();
    let pass_one_result = pass_one
        .execute(&narrative)
        .await
        .expect("Pass 1 should succeed");

    let workflow = TwoPassDebateWorkflow::new();
    let pass_one_debate = workflow
        .generate_pass_one_debate_mock(&narrative, &pass_one_result)
        .await
        .expect("Pass 1 debate generation should succeed");

    let pass_two = PassTwoOptimizer::new();
    let result = pass_two
        .execute(&narrative, &pass_one_result, &pass_one_debate)
        .await;

    assert!(
        result.is_ok(),
        "PassTwoOptimizer should execute successfully"
    );

    let pass_two_result = result.unwrap();
    assert_eq!(
        pass_two_result.debate.pass,
        terraphim_truthforge::DebatePass::PassTwo
    );
    assert!(
        !pass_two_result.exploited_vulnerabilities.is_empty(),
        "Should have exploited vulnerabilities"
    );
    assert!(
        pass_two_result.exploited_vulnerabilities.len() <= 7,
        "Should exploit at most 7 vulnerabilities"
    );
}

#[tokio::test]
async fn test_pass_two_shows_vulnerability_amplification() {
    let session_id = Uuid::new_v4();

    let narrative = NarrativeInput {
        session_id,
        text: "We reduced costs by 40%. Shareholders benefited greatly.".to_string(),
        context: NarrativeContext {
            urgency: UrgencyLevel::High,
            stakes: vec![StakeType::Reputational, StakeType::Financial],
            audience: AudienceType::PublicMedia,
        },
        submitted_at: Utc::now(),
    };

    let pass_one = PassOneOrchestrator::new();
    let pass_one_result = pass_one
        .execute(&narrative)
        .await
        .expect("Pass 1 should succeed");

    let workflow = TwoPassDebateWorkflow::new();
    let pass_one_debate = workflow
        .generate_pass_one_debate_mock(&narrative, &pass_one_result)
        .await
        .expect("Pass 1 debate generation should succeed");

    let pass_two = PassTwoOptimizer::new();
    let pass_two_result = pass_two
        .execute(&narrative, &pass_one_result, &pass_one_debate)
        .await
        .expect("Pass 2 should succeed");

    let pass_one_opposing = pass_one_debate.evaluation.scores.opposing_strength;
    let pass_two_opposing = pass_two_result.debate.evaluation.scores.opposing_strength;

    assert!(
        pass_two_opposing > pass_one_opposing,
        "Pass 2 opposing strength ({}) should be greater than Pass 1 ({})",
        pass_two_opposing,
        pass_one_opposing
    );
}

#[tokio::test]
async fn test_pass_two_defensive_weakens() {
    let session_id = Uuid::new_v4();

    let narrative = NarrativeInput {
        session_id,
        text: "We reduced costs by 40%. Shareholders benefited greatly.".to_string(),
        context: NarrativeContext {
            urgency: UrgencyLevel::High,
            stakes: vec![StakeType::Reputational, StakeType::Financial],
            audience: AudienceType::PublicMedia,
        },
        submitted_at: Utc::now(),
    };

    let pass_one = PassOneOrchestrator::new();
    let pass_one_result = pass_one
        .execute(&narrative)
        .await
        .expect("Pass 1 should succeed");

    let workflow = TwoPassDebateWorkflow::new();
    let pass_one_debate = workflow
        .generate_pass_one_debate_mock(&narrative, &pass_one_result)
        .await
        .expect("Pass 1 debate generation should succeed");

    let pass_two = PassTwoOptimizer::new();
    let pass_two_result = pass_two
        .execute(&narrative, &pass_one_result, &pass_one_debate)
        .await
        .expect("Pass 2 should succeed");

    let pass_one_supporting = pass_one_debate.evaluation.scores.supporting_strength;
    let pass_two_supporting = pass_two_result.debate.evaluation.scores.supporting_strength;

    assert!(
        pass_two_supporting < pass_one_supporting,
        "Pass 2 supporting strength ({}) should be weaker than Pass 1 ({})",
        pass_two_supporting,
        pass_one_supporting
    );
}

#[tokio::test]
async fn test_pass_two_exploitation_targets_omissions() {
    let session_id = Uuid::new_v4();

    let narrative = NarrativeInput {
        session_id,
        text: "We reduced costs by 40%. Shareholders benefited greatly.".to_string(),
        context: NarrativeContext {
            urgency: UrgencyLevel::High,
            stakes: vec![StakeType::Reputational, StakeType::Financial],
            audience: AudienceType::PublicMedia,
        },
        submitted_at: Utc::now(),
    };

    let pass_one = PassOneOrchestrator::new();
    let pass_one_result = pass_one
        .execute(&narrative)
        .await
        .expect("Pass 1 should succeed");

    let workflow = TwoPassDebateWorkflow::new();
    let pass_one_debate = workflow
        .generate_pass_one_debate_mock(&narrative, &pass_one_result)
        .await
        .expect("Pass 1 debate generation should succeed");

    let pass_two = PassTwoOptimizer::new();
    let pass_two_result = pass_two
        .execute(&narrative, &pass_one_result, &pass_one_debate)
        .await
        .expect("Pass 2 should succeed");

    let exploited_ids = &pass_two_result.exploited_vulnerabilities;
    let supporting_refs = &pass_two_result
        .debate
        .supporting_argument
        .omissions_referenced;
    let opposing_refs = &pass_two_result
        .debate
        .opposing_argument
        .omissions_referenced;

    assert_eq!(
        supporting_refs, exploited_ids,
        "Supporting argument should reference exploited vulnerabilities"
    );

    assert_eq!(
        opposing_refs, exploited_ids,
        "Opposing argument should reference exploited vulnerabilities"
    );

    let opposing_ref_percentage = (opposing_refs.len() as f64 / exploited_ids.len() as f64) * 100.0;
    assert!(
        opposing_ref_percentage >= 80.0,
        "Opposing argument should reference ≥80% of omissions, got {:.1}%",
        opposing_ref_percentage
    );
}
