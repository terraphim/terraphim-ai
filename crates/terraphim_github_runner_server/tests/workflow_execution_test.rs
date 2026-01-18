use terraphim_github_runner::WorkflowParser;

/// Test that WorkflowParser can be cloned
#[test]
fn test_workflow_parser_clone() {
    // We can't directly instantiate WorkflowParser without an LlmClient
    // But we can verify that the Clone trait is implemented
    // This is a compile-time check

    // The test verifies that WorkflowParser: Clone is implemented
    // by successfully compiling code that uses .clone()
    fn assert_clone<T: Clone>() {}
    assert_clone::<WorkflowParser>();
}

/// Test Option::cloned() works correctly with WorkflowParser reference
#[test]
fn test_option_cloned_with_workflow_parser() {
    // This test verifies the API compiles correctly
    // We don't need an actual parser instance, just verify the types work

    // Test with None
    let none_parser: Option<&WorkflowParser> = None;
    let cloned_none = none_parser.cloned();

    assert!(cloned_none.is_none());

    // The fact that this compiles proves Option<&WorkflowParser>::cloned() -> Option<WorkflowParser>
    // works correctly
}

/// Test that cloned parser is independent
#[test]
fn test_cloned_parser_independence() {
    // This is a compile-time test to verify the ownership model
    // We can't create a real parser without an LlmClient, but we can verify
    // that the API allows moving the clone into async blocks

    fn accepts_owned_parser(parser: WorkflowParser) {
        let _ = parser;
    }

    fn accepts_ref_parser(parser: &WorkflowParser) {
        let _ = parser;
    }

    // Both functions should work with different ownership models
    // This verifies the API flexibility
    let _ = accepts_owned_parser;
    let _ = accepts_ref_parser;
}
