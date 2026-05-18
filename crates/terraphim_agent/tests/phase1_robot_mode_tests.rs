use terraphim_agent::forgiving::{AliasRegistry, ForgivingParser};
use terraphim_agent::robot::schema::{SearchResultItem, SearchResultsData};
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
fn test_format_flag_table() {
    let config = RobotConfig::new().with_format(OutputFormat::Table);
    let formatter = RobotFormatter::new(config);
    let data = serde_json::json!({"status": "ok"});
    let output = formatter.format(&data).unwrap();
    assert!(output.contains("status"));
}

#[test]
fn test_robot_response_serialized_in_all_formats() {
    let formats = [
        OutputFormat::Json,
        OutputFormat::Jsonl,
        OutputFormat::Minimal,
        OutputFormat::Table,
    ];
    for format in formats {
        let meta = ResponseMeta::new("search");
        let response = RobotResponse::success(serde_json::json!({"results": []}), meta);
        let config = RobotConfig::new().with_format(format);
        let formatter = RobotFormatter::new(config);
        let output = formatter.format(&response).unwrap();
        let json: serde_json::Value =
            serde_json::from_str(&output).expect("robot response must be valid JSON");
        assert_eq!(
            json.get("success").and_then(|v| v.as_bool()),
            Some(true),
            "format {:?}: response.success should be true",
            format
        );
        assert!(
            json.get("meta").is_some(),
            "format {:?}: response.meta must be present",
            format
        );
    }
}

#[test]
fn test_robot_response_error_serialized_in_all_formats() {
    let formats = [
        OutputFormat::Json,
        OutputFormat::Jsonl,
        OutputFormat::Minimal,
        OutputFormat::Table,
    ];
    for format in formats {
        use terraphim_agent::robot::RobotError;
        let meta = ResponseMeta::new("search");
        let errors = vec![RobotError::no_results("nothing found")];
        let response = RobotResponse::<()>::error(errors, meta);
        let config = RobotConfig::new().with_format(format);
        let formatter = RobotFormatter::new(config);
        let output = formatter.format(&response).unwrap();
        let json: serde_json::Value =
            serde_json::from_str(&output).expect("error response must be valid JSON");
        assert_eq!(
            json.get("success").and_then(|v| v.as_bool()),
            Some(false),
            "format {:?}: error response.success should be false",
            format
        );
        assert!(
            json.get("errors").is_some(),
            "format {:?}: error response.errors must be present",
            format
        );
    }
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

/// AC-a: when concepts_matched is non-empty the field is populated in the JSON envelope.
#[test]
fn test_concepts_matched_populated_in_search_results_data() {
    let data = SearchResultsData {
        results: vec![],
        total_matches: 0,
        concepts_matched: vec!["knowledge graph".to_string()],
        wildcard_fallback: false,
    };
    let meta = ResponseMeta::new("search");
    let response = RobotResponse::success(data, meta);
    let config = RobotConfig::new().with_format(OutputFormat::Json);
    let formatter = RobotFormatter::new(config);
    let output = formatter.format(&response).unwrap();
    let json: serde_json::Value = serde_json::from_str(&output).unwrap();
    let concepts = json
        .pointer("/data/concepts_matched")
        .and_then(|v| v.as_array())
        .expect("concepts_matched must be present and an array");
    assert!(
        concepts
            .iter()
            .any(|c| c.as_str() == Some("knowledge graph")),
        "concepts_matched must contain the matched concept; got {:?}",
        concepts
    );
    assert_eq!(
        json.pointer("/data/wildcard_fallback")
            .and_then(|v| v.as_bool()),
        Some(false),
        "wildcard_fallback must be false when concepts matched"
    );
}

/// AC-b: when concepts_matched is empty the wildcard_fallback flag must be true.
#[test]
fn test_wildcard_fallback_true_when_no_concepts_matched() {
    // Simulate the logic applied at both search emission sites:
    // wildcard_fallback = concepts_matched.is_empty()
    let concepts_matched: Vec<String> = vec![];
    let wildcard_fallback = concepts_matched.is_empty();

    let data = SearchResultsData {
        results: vec![SearchResultItem {
            rank: 1,
            id: "doc-1".to_string(),
            title: "Raw text match".to_string(),
            url: None,
            score: 0.5,
            preview: None,
            source: None,
            date: None,
            preview_truncated: false,
        }],
        total_matches: 1,
        concepts_matched,
        wildcard_fallback,
    };

    assert!(
        data.wildcard_fallback,
        "wildcard_fallback must be true when concepts_matched is empty"
    );

    let meta = ResponseMeta::new("search");
    let response = RobotResponse::success(data, meta);
    let config = RobotConfig::new().with_format(OutputFormat::Json);
    let formatter = RobotFormatter::new(config);
    let output = formatter.format(&response).unwrap();
    let json: serde_json::Value = serde_json::from_str(&output).unwrap();
    assert_eq!(
        json.pointer("/data/wildcard_fallback")
            .and_then(|v| v.as_bool()),
        Some(true),
        "wildcard_fallback must be true in serialised output"
    );
    // concepts_matched omitted from JSON due to skip_serializing_if = "Vec::is_empty"
    assert!(
        json.pointer("/data/concepts_matched").is_none(),
        "concepts_matched must be absent from JSON when empty (serde skip_serializing_if)"
    );
}
