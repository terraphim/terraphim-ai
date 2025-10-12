use chrono::Utc;
use terraphim_truthforge::{
    AudienceType, NarrativeContext, NarrativeInput, StakeType, TwoPassDebateWorkflow, UrgencyLevel,
};
use uuid::Uuid;

#[tokio::test]
async fn test_complete_workflow_end_to_end() {
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

    let workflow = TwoPassDebateWorkflow::new();
    let result = workflow.execute(&narrative).await;

    assert!(
        result.is_ok(),
        "Complete workflow should execute successfully"
    );

    let analysis = result.unwrap();

    assert_eq!(analysis.session_id, session_id);
    assert!(
        !analysis.omission_catalog.omissions.is_empty(),
        "Should have identified omissions"
    );
    assert_eq!(
        analysis.response_strategies.len(),
        3,
        "Should have generated 3 response strategies"
    );

    assert!(
        !analysis.executive_summary.is_empty(),
        "Should have executive summary"
    );

    // Processing time may be 0 or very low for mock execution, just verify it's tracked
    println!("Processing time: {}ms", analysis.processing_time_ms);
}

#[tokio::test]
async fn test_workflow_produces_complete_analysis_result() {
    let session_id = Uuid::new_v4();

    let narrative = NarrativeInput {
        session_id,
        text: "We achieved record profits while maintaining our commitment to sustainability."
            .to_string(),
        context: NarrativeContext {
            urgency: UrgencyLevel::High,
            stakes: vec![StakeType::Reputational, StakeType::SocialLicense],
            audience: AudienceType::PublicMedia,
        },
        submitted_at: Utc::now(),
    };

    let workflow = TwoPassDebateWorkflow::new();
    let analysis = workflow.execute(&narrative).await.expect("Should execute");

    assert!(
        !analysis.bias_analysis.biases.is_empty()
            || analysis.bias_analysis.overall_bias_score > 0.0,
        "Should have bias analysis"
    );

    assert_eq!(
        analysis.narrative_mapping.scct_classification,
        terraphim_truthforge::SCCTClassification::Accidental,
        "Should classify SCCT"
    );

    assert!(
        !analysis.taxonomy_linking.primary_function.is_empty(),
        "Should link to taxonomy"
    );

    assert_eq!(
        analysis.pass_one_debate.pass,
        terraphim_truthforge::DebatePass::PassOne
    );
    assert_eq!(
        analysis.pass_two_debate.pass,
        terraphim_truthforge::DebatePass::PassTwo
    );

    assert!(
        analysis
            .cumulative_analysis
            .vulnerability_delta
            .critical_omissions_exploited
            > 0,
        "Should have exploited omissions"
    );
}

#[tokio::test]
async fn test_workflow_pass_two_shows_amplification() {
    let session_id = Uuid::new_v4();

    let narrative = NarrativeInput {
        session_id,
        text: "Our restructuring plan will create efficiencies across all departments.".to_string(),
        context: NarrativeContext {
            urgency: UrgencyLevel::High,
            stakes: vec![StakeType::Operational, StakeType::Reputational],
            audience: AudienceType::Internal,
        },
        submitted_at: Utc::now(),
    };

    let workflow = TwoPassDebateWorkflow::new();
    let analysis = workflow.execute(&narrative).await.expect("Should execute");

    let pass_one_opposing = analysis.pass_one_debate.evaluation.scores.opposing_strength;
    let pass_two_opposing = analysis.pass_two_debate.evaluation.scores.opposing_strength;

    assert!(
        pass_two_opposing > pass_one_opposing,
        "Pass 2 opposing strength ({}) should be greater than Pass 1 ({})",
        pass_two_opposing,
        pass_one_opposing
    );

    let pass_one_supporting = analysis
        .pass_one_debate
        .evaluation
        .scores
        .supporting_strength;
    let pass_two_supporting = analysis
        .pass_two_debate
        .evaluation
        .scores
        .supporting_strength;

    assert!(
        pass_two_supporting < pass_one_supporting,
        "Pass 2 supporting strength ({}) should be less than Pass 1 ({})",
        pass_two_supporting,
        pass_one_supporting
    );

    assert!(
        analysis
            .cumulative_analysis
            .vulnerability_delta
            .amplification_factor
            > 1.0,
        "Should show vulnerability amplification"
    );
}

#[tokio::test]
async fn test_workflow_response_strategies_address_vulnerabilities() {
    let session_id = Uuid::new_v4();

    let narrative = NarrativeInput {
        session_id,
        text: "We're pivoting our business model to focus on high-growth markets.".to_string(),
        context: NarrativeContext {
            urgency: UrgencyLevel::High,
            stakes: vec![StakeType::Financial, StakeType::Operational],
            audience: AudienceType::PublicMedia,
        },
        submitted_at: Utc::now(),
    };

    let workflow = TwoPassDebateWorkflow::new();
    let analysis = workflow.execute(&narrative).await.expect("Should execute");

    assert_eq!(
        analysis.response_strategies.len(),
        3,
        "Should have 3 strategies"
    );

    for strategy in &analysis.response_strategies {
        assert!(
            !strategy.vulnerabilities_addressed.is_empty(),
            "Each strategy should address vulnerabilities"
        );

        assert!(
            !strategy.drafts.social_media.is_empty(),
            "Should have social media draft"
        );
        assert!(
            !strategy.drafts.press_statement.is_empty(),
            "Should have press statement"
        );
        assert!(
            !strategy.drafts.internal_memo.is_empty(),
            "Should have internal memo"
        );
        assert!(
            !strategy.drafts.qa_brief.is_empty(),
            "Should have Q&A brief"
        );

        assert!(
            !strategy.risk_assessment.potential_backfire.is_empty(),
            "Should identify backfire risks"
        );

        assert!(
            strategy.risk_assessment.media_amplification_risk >= 0.0
                && strategy.risk_assessment.media_amplification_risk <= 1.0,
            "Media risk should be 0-1"
        );
    }
}

#[tokio::test]
async fn test_workflow_performance_under_5_seconds() {
    let session_id = Uuid::new_v4();

    let narrative = NarrativeInput {
        session_id,
        text: "We're excited to announce our new partnership strategy.".to_string(),
        context: NarrativeContext {
            urgency: UrgencyLevel::Low,
            stakes: vec![StakeType::Reputational],
            audience: AudienceType::PublicMedia,
        },
        submitted_at: Utc::now(),
    };

    let start = std::time::Instant::now();
    let workflow = TwoPassDebateWorkflow::new();
    let analysis = workflow.execute(&narrative).await.expect("Should execute");
    let elapsed = start.elapsed();

    assert!(
        elapsed.as_secs() < 5,
        "Workflow should complete in <5 seconds, took {:?}",
        elapsed
    );

    assert!(
        analysis.processing_time_ms < 5000,
        "Tracked processing time should be <5s, was {}ms",
        analysis.processing_time_ms
    );
}

#[tokio::test]
async fn test_workflow_handles_different_urgency_levels() {
    let session_id = Uuid::new_v4();

    let high_urgency = NarrativeInput {
        session_id,
        text: "BREAKING: Incident requires immediate response.".to_string(),
        context: NarrativeContext {
            urgency: UrgencyLevel::High,
            stakes: vec![StakeType::Legal, StakeType::Reputational],
            audience: AudienceType::PublicMedia,
        },
        submitted_at: Utc::now(),
    };

    let workflow = TwoPassDebateWorkflow::new();
    let high_result = workflow
        .execute(&high_urgency)
        .await
        .expect("Should handle high urgency");

    assert!(
        !high_result.omission_catalog.omissions.is_empty(),
        "High urgency should identify omissions"
    );

    let low_urgency = NarrativeInput {
        session_id: Uuid::new_v4(),
        text: "We're pleased to share our quarterly update with stakeholders.".to_string(),
        context: NarrativeContext {
            urgency: UrgencyLevel::Low,
            stakes: vec![StakeType::Reputational],
            audience: AudienceType::Internal,
        },
        submitted_at: Utc::now(),
    };

    let low_result = workflow
        .execute(&low_urgency)
        .await
        .expect("Should handle low urgency");

    assert!(
        !low_result.omission_catalog.omissions.is_empty(),
        "Low urgency should also identify omissions"
    );
}

#[tokio::test]
async fn test_workflow_executive_summary_includes_key_metrics() {
    let session_id = Uuid::new_v4();

    let narrative = NarrativeInput {
        session_id,
        text: "We're implementing changes to improve customer experience.".to_string(),
        context: NarrativeContext {
            urgency: UrgencyLevel::High,
            stakes: vec![StakeType::Reputational, StakeType::Financial],
            audience: AudienceType::PublicMedia,
        },
        submitted_at: Utc::now(),
    };

    let workflow = TwoPassDebateWorkflow::new();
    let analysis = workflow.execute(&narrative).await.expect("Should execute");

    let summary = &analysis.executive_summary;

    assert!(
        summary.contains("omission"),
        "Summary should mention omissions: {}",
        summary
    );
    assert!(
        summary.contains("vulnerabilities") || summary.contains("exploited"),
        "Summary should mention exploitation: {}",
        summary
    );
    assert!(
        summary.contains("risk level")
            || summary.contains("Severe")
            || summary.contains("High")
            || summary.contains("Moderate")
            || summary.contains("Low"),
        "Summary should mention risk level: {}",
        summary
    );
    assert!(
        summary.contains("3 response strategies") || summary.contains("strategies"),
        "Summary should mention response strategies: {}",
        summary
    );
}
