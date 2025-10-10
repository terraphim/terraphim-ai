use terraphim_truthforge::workflows::PassOneOrchestrator;
use terraphim_truthforge::{
    AudienceType, NarrativeContext, NarrativeInput, StakeType, UrgencyLevel,
};

#[tokio::test]
async fn test_pass_one_orchestrator_parallel_execution() {
    let orchestrator = PassOneOrchestrator::new();

    let narrative = NarrativeInput {
        session_id: uuid::Uuid::new_v4(),
        text: "We achieved significant cost reductions through operational efficiency improvements. Our shareholders have benefited from improved margins.".to_string(),
        context: NarrativeContext {
            urgency: UrgencyLevel::High,
            stakes: vec![StakeType::Reputational, StakeType::Financial],
            audience: AudienceType::PublicMedia,
        },
        submitted_at: chrono::Utc::now(),
    };

    let result = orchestrator.execute(&narrative).await;
    assert!(result.is_ok(), "Pass 1 orchestration should succeed");

    let pass_one = result.unwrap();

    assert!(
        !pass_one.omission_catalog.omissions.is_empty(),
        "Should detect omissions"
    );

    assert!(
        pass_one.bias_analysis.overall_bias_score > 0.0,
        "Should have bias score"
    );

    let scct_is_valid = matches!(
        pass_one.narrative_mapping.scct_classification,
        terraphim_truthforge::SCCTClassification::Victim
            | terraphim_truthforge::SCCTClassification::Accidental
            | terraphim_truthforge::SCCTClassification::Preventable
    );
    assert!(scct_is_valid, "Should classify SCCT");

    assert!(
        !pass_one.taxonomy_linking.subfunctions.is_empty(),
        "Should link to taxonomy"
    );
}

#[tokio::test]
async fn test_pass_one_omission_detection_integration() {
    let orchestrator = PassOneOrchestrator::new();

    let narrative = NarrativeInput {
        session_id: uuid::Uuid::new_v4(),
        text: "40% cost reduction achieved. Efficiency gains delivered.".to_string(),
        context: NarrativeContext {
            urgency: UrgencyLevel::High,
            stakes: vec![StakeType::Financial],
            audience: AudienceType::PublicMedia,
        },
        submitted_at: chrono::Utc::now(),
    };

    let result = orchestrator.execute(&narrative).await.unwrap();

    assert!(
        result.omission_catalog.omissions.len() >= 2,
        "Should detect multiple omissions for vague claims"
    );

    assert!(
        result.omission_catalog.total_risk_score > 0.0,
        "Should calculate risk score"
    );

    let has_missing_evidence = result.omission_catalog.omissions.iter().any(|o| {
        matches!(
            o.category,
            terraphim_truthforge::OmissionCategory::MissingEvidence
        )
    });

    assert!(
        has_missing_evidence,
        "Should detect missing evidence for percentage claims"
    );
}

#[tokio::test]
async fn test_pass_one_handles_empty_narrative() {
    let orchestrator = PassOneOrchestrator::new();

    let narrative = NarrativeInput {
        session_id: uuid::Uuid::new_v4(),
        text: "".to_string(),
        context: NarrativeContext {
            urgency: UrgencyLevel::Low,
            stakes: vec![],
            audience: AudienceType::Internal,
        },
        submitted_at: chrono::Utc::now(),
    };

    let result = orchestrator.execute(&narrative).await;

    assert!(result.is_ok(), "Should handle empty narrative gracefully");
}

#[tokio::test]
async fn test_pass_one_concurrent_agent_execution() {
    let orchestrator = PassOneOrchestrator::new();

    let narrative = NarrativeInput {
        session_id: uuid::Uuid::new_v4(),
        text: "Strategic restructuring completed with positive outcomes for key stakeholders."
            .to_string(),
        context: NarrativeContext {
            urgency: UrgencyLevel::High,
            stakes: vec![StakeType::Reputational, StakeType::SocialLicense],
            audience: AudienceType::PublicMedia,
        },
        submitted_at: chrono::Utc::now(),
    };

    let start = std::time::Instant::now();
    let result = orchestrator.execute(&narrative).await.unwrap();
    let elapsed = start.elapsed();

    println!("Pass 1 execution time: {:?}", elapsed);

    assert!(
        elapsed.as_millis() < 5000,
        "Parallel execution should be fast"
    );

    assert!(result.omission_catalog.omissions.len() > 0);
    assert!(result.bias_analysis.confidence > 0.0);
}
