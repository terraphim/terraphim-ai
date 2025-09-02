use terraphim_config::{Haystack, Role, ServiceType};

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
    let mut role = Role::new("ClickUp");
    role.shortname = Some("ClickUp".to_string());
    role.theme = "lumen".to_string();
    role.haystacks = vec![Haystack::new("clickup".into(), ServiceType::ClickUp, true)];
    let json = serde_json::to_string(&role).unwrap();
    assert!(json.contains("ClickUp"));
}
