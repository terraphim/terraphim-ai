use terraphim_config::{Haystack, Role, ServiceType};
use terraphim_types::RelevanceFunction;

#[test]
fn clickup_haystack_serializes_extra_parameters() {
    let haystack = Haystack::new("clickup".into(), ServiceType::ClickUp, true)
        .with_extra_parameter("team_id".into(), "123456".into())
        .with_extra_parameter("include_closed".into(), "true".into());

    let json = serde_json::to_string(&haystack).unwrap();
    assert!(json.contains("\"extra_parameters\""));
    assert!(json.contains("team_id"));
}

#[test]
fn role_with_clickup_haystack_is_valid() {
    let role = Role {
        shortname: Some("ClickUp".to_string()),
        name: "ClickUp".into(),
        relevance_function: RelevanceFunction::TitleScorer,
        terraphim_it: false,
        theme: "lumen".into(),
        kg: None,
        haystacks: vec![Haystack::new("clickup".into(), ServiceType::ClickUp, true)],
        extra: ahash::AHashMap::new(),
    };
    let json = serde_json::to_string(&role).unwrap();
    assert!(json.contains("ClickUp"));
}


