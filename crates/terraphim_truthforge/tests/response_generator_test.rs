use chrono::Utc;
use terraphim_truthforge::{
    AudienceType, NarrativeContext, NarrativeInput, PassOneOrchestrator, PassTwoOptimizer,
    ResponseGenerator, StakeType, StrategyType, ToneGuidance, TwoPassDebateWorkflow, UrgencyLevel,
};
use uuid::Uuid;

#[tokio::test]
async fn test_response_generator_creates_three_strategies() {
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

    let cumulative_analysis = workflow
        .generate_cumulative_analysis_mock(
            &pass_one_debate,
            &pass_two_result.debate,
            &pass_two_result.exploited_vulnerabilities,
        )
        .await
        .expect("Cumulative analysis should succeed");

    let response_gen = ResponseGenerator::new();
    let strategies = response_gen
        .generate_strategies(
            &narrative,
            &cumulative_analysis,
            &pass_one_result.omission_catalog,
        )
        .await;

    assert!(
        strategies.is_ok(),
        "ResponseGenerator should execute successfully"
    );

    let strategies = strategies.unwrap();
    assert_eq!(strategies.len(), 3, "Should generate exactly 3 strategies");
}

#[tokio::test]
async fn test_response_generator_strategy_types() {
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

    let cumulative_analysis = workflow
        .generate_cumulative_analysis_mock(
            &pass_one_debate,
            &pass_two_result.debate,
            &pass_two_result.exploited_vulnerabilities,
        )
        .await
        .expect("Cumulative analysis should succeed");

    let response_gen = ResponseGenerator::new();
    let strategies = response_gen
        .generate_strategies(
            &narrative,
            &cumulative_analysis,
            &pass_one_result.omission_catalog,
        )
        .await
        .expect("Should generate strategies");

    assert_eq!(strategies[0].strategy_type, StrategyType::Reframe);
    assert_eq!(strategies[1].strategy_type, StrategyType::CounterArgue);
    assert_eq!(strategies[2].strategy_type, StrategyType::Bridge);
}

#[tokio::test]
async fn test_response_generator_includes_all_drafts() {
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

    let cumulative_analysis = workflow
        .generate_cumulative_analysis_mock(
            &pass_one_debate,
            &pass_two_result.debate,
            &pass_two_result.exploited_vulnerabilities,
        )
        .await
        .expect("Cumulative analysis should succeed");

    let response_gen = ResponseGenerator::new();
    let strategies = response_gen
        .generate_strategies(
            &narrative,
            &cumulative_analysis,
            &pass_one_result.omission_catalog,
        )
        .await
        .expect("Should generate strategies");

    for strategy in &strategies {
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
            strategy.drafts.qa_brief.len() >= 2,
            "Should have at least 2 Q&A pairs"
        );
    }
}

#[tokio::test]
async fn test_response_generator_tone_guidance() {
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

    let cumulative_analysis = workflow
        .generate_cumulative_analysis_mock(
            &pass_one_debate,
            &pass_two_result.debate,
            &pass_two_result.exploited_vulnerabilities,
        )
        .await
        .expect("Cumulative analysis should succeed");

    let response_gen = ResponseGenerator::new();
    let strategies = response_gen
        .generate_strategies(
            &narrative,
            &cumulative_analysis,
            &pass_one_result.omission_catalog,
        )
        .await
        .expect("Should generate strategies");

    assert_eq!(
        strategies[0].tone_guidance,
        ToneGuidance::Empathetic,
        "Reframe should be empathetic"
    );
    assert_eq!(
        strategies[1].tone_guidance,
        ToneGuidance::Assertive,
        "CounterArgue should be assertive"
    );
    assert_eq!(
        strategies[2].tone_guidance,
        ToneGuidance::Collaborative,
        "Bridge should be collaborative"
    );
}

#[tokio::test]
async fn test_response_generator_risk_assessments() {
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

    let cumulative_analysis = workflow
        .generate_cumulative_analysis_mock(
            &pass_one_debate,
            &pass_two_result.debate,
            &pass_two_result.exploited_vulnerabilities,
        )
        .await
        .expect("Cumulative analysis should succeed");

    let response_gen = ResponseGenerator::new();
    let strategies = response_gen
        .generate_strategies(
            &narrative,
            &cumulative_analysis,
            &pass_one_result.omission_catalog,
        )
        .await
        .expect("Should generate strategies");

    for strategy in &strategies {
        assert!(
            !strategy.risk_assessment.potential_backfire.is_empty(),
            "Should identify backfire risks"
        );
        assert!(
            !strategy
                .risk_assessment
                .stakeholder_reaction
                .supporters
                .is_empty(),
            "Should predict supporter reaction"
        );
        assert!(
            !strategy
                .risk_assessment
                .stakeholder_reaction
                .skeptics
                .is_empty(),
            "Should predict skeptic reaction"
        );
        assert!(
            !strategy
                .risk_assessment
                .stakeholder_reaction
                .media
                .is_empty(),
            "Should predict media reaction"
        );
        assert!(
            strategy.risk_assessment.media_amplification_risk >= 0.0
                && strategy.risk_assessment.media_amplification_risk <= 1.0,
            "Media amplification risk should be 0-1"
        );
    }

    assert!(
        strategies[1].risk_assessment.media_amplification_risk
            > strategies[2].risk_assessment.media_amplification_risk,
        "CounterArgue (0.7) should have higher media risk than Bridge (0.3)"
    );
}
