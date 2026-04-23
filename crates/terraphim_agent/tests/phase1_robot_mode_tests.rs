use terraphim_agent::forgiving::{AliasRegistry, ForgivingParser};
use terraphim_agent::robot::{
    ExitCode, FieldMode, OutputFormat, ResponseMeta, RobotConfig, RobotFormatter, RobotResponse,
};

#[test]
fn test_parser_to_robot_formatter_pipeline() {
    let parser = ForgivingParser::default();
    let result = parser.parse("serach hello world");

    assert!(result.is_success());
    assert_eq!(result.command(), Some("search"));

    let config = RobotConfig::new().with_format(OutputFormat::Json);
    let formatter = RobotFormatter::new(config);

    let payload = serde_json::json!({
        "command": result.command(),
        "args": result.args(),
        "corrected": result.was_corrected(),
    });
    let output = formatter.format(&payload).unwrap();
    assert!(output.contains("search"));
    assert!(output.contains("hello world"));
}

#[test]
fn test_alias_expansion_to_robot_output() {
    let parser = ForgivingParser::default();
    let result = parser.parse("q test query");

    assert!(result.is_success());
    assert!(result.was_alias());
    assert_eq!(result.command(), Some("search"));

    let formatter = RobotFormatter::new(RobotConfig::new());
    let meta = ResponseMeta::new("search");
    let response = RobotResponse::success(
        serde_json::json!({"query": "test query", "results": []}),
        meta,
    );
    let output = formatter.format(&response).unwrap();
    assert!(output.contains("success"));
    assert!(output.contains("test query"));
}

#[test]
fn test_exit_codes_for_search_outcomes() {
    assert_eq!(ExitCode::Success.code(), 0);
    assert_eq!(ExitCode::ErrorNotFound.code(), 4);
    assert_eq!(ExitCode::ErrorUsage.code(), 2);
    assert_eq!(ExitCode::ErrorGeneral.code(), 1);
}

#[test]
fn test_format_flag_json() {
    let config = RobotConfig::new().with_format(OutputFormat::Json);
    let formatter = RobotFormatter::new(config);
    let data = serde_json::json!({"status": "ok"});
    let output = formatter.format(&data).unwrap();
    assert!(output.contains('\n'));
}

#[test]
fn test_format_flag_jsonl() {
    let config = RobotConfig::new().with_format(OutputFormat::Jsonl);
    let formatter = RobotFormatter::new(config);
    let data = serde_json::json!({"status": "ok"});
    let output = formatter.format(&data).unwrap();
    assert!(!output.contains('\n'));
}

#[test]
fn test_format_flag_minimal() {
    let config = RobotConfig::new().with_format(OutputFormat::Minimal);
    let formatter = RobotFormatter::new(config);
    let data = serde_json::json!({"status": "ok"});
    let output = formatter.format(&data).unwrap();
    assert!(!output.contains('\n'));
}

#[test]
fn test_field_modes_with_formatter() {
    let config = RobotConfig::new()
        .with_fields(FieldMode::Minimal)
        .with_max_results(5);
    assert_eq!(config.fields, FieldMode::Minimal);
    assert_eq!(config.max_results, Some(5));
}

#[test]
fn test_error_response_with_exit_code() {
    let meta = ResponseMeta::new("search");
    use terraphim_agent::robot::RobotError;
    let errors = vec![RobotError::no_results("xyz")];
    let response = RobotResponse::<()>::error(errors, meta);

    assert!(!response.success);
    assert!(response.data.is_none());
    assert!(!response.errors.is_empty());

    let exit_code = if response.success {
        ExitCode::Success
    } else {
        ExitCode::ErrorNotFound
    };
    assert_eq!(exit_code.code(), 4);
}

#[test]
fn test_custom_alias_registry_with_parser() {
    let mut aliases = AliasRegistry::empty();
    aliases.add("ls", "sessions list");
    aliases.add("ss", "sessions search");

    let parser = ForgivingParser::new(vec![
        "sessions".to_string(),
        "search".to_string(),
        "list".to_string(),
    ])
    .with_aliases(aliases);

    let result = parser.parse("ls");
    assert!(result.is_success());
    assert!(result.was_alias());
}

#[test]
fn test_robot_config_is_robot_mode() {
    let config = RobotConfig::new();
    assert!(config.is_robot_mode());

    let default_config = RobotConfig::default();
    assert!(!default_config.is_robot_mode());
}

#[test]
fn test_truncation_with_budget() {
    let config = RobotConfig::new()
        .with_max_content_length(50)
        .with_max_tokens(100);
    let formatter = RobotFormatter::new(config);

    let long_content = "x".repeat(200);
    let (truncated, was_truncated) = formatter.truncate_content(&long_content);
    assert!(was_truncated);
    assert!(truncated.len() <= 53);
}
