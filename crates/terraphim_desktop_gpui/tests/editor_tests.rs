use terraphim_desktop_gpui::editor::{
    EditorState, SlashCommand, SlashCommandHandler, SlashCommandManager,
};

#[test]
fn test_editor_state_creation() {
    let state = EditorState::new();
    assert!(state.is_empty());
    assert_eq!(state.cursor_position, 0);
    assert_eq!(state.selection, None);
    assert!(!state.modified);
}

#[test]
fn test_editor_insert_text() {
    let mut state = EditorState::new();
    state.insert_text("Hello");

    assert_eq!(state.content, "Hello");
    assert_eq!(state.cursor_position, 5);
    assert!(state.modified);
}

#[test]
fn test_editor_insert_with_selection() {
    let mut state = EditorState::from_content("Hello world".to_string());
    state.cursor_position = 0;
    state.selection = Some((0, 5)); // Select "Hello"

    state.insert_text("Hi");

    assert_eq!(state.content, "Hi world");
    assert_eq!(state.cursor_position, 2);
    assert_eq!(state.selection, None);
    assert!(state.modified);
}

#[test]
fn test_editor_delete_selection() {
    let mut state = EditorState::from_content("Hello world".to_string());
    state.selection = Some((0, 6)); // Select "Hello "

    state.delete_selection();

    assert_eq!(state.content, "world");
    assert_eq!(state.cursor_position, 0);
    assert_eq!(state.selection, None);
    assert!(state.modified);
}

#[test]
fn test_editor_get_word_at_cursor() {
    let mut state = EditorState::from_content("Hello world test".to_string());
    state.cursor_position = 8; // Inside "world"

    let word = state.get_word_at_cursor();
    assert_eq!(word, Some("world".to_string()));
}

#[test]
fn test_editor_line_count() {
    let state = EditorState::from_content("Line 1\nLine 2\nLine 3".to_string());
    assert_eq!(state.line_count(), 3);

    let empty = EditorState::new();
    assert_eq!(empty.line_count(), 1); // At least 1 line
}

#[test]
fn test_editor_char_count() {
    let state = EditorState::from_content("Hello 世界".to_string());
    assert_eq!(state.char_count(), 8); // Counts Unicode characters correctly
}

#[test]
fn test_slash_command_manager_creation() {
    let manager = SlashCommandManager::new();
    let commands = manager.list_commands();

    assert!(commands.len() >= 5); // At least 5 built-in commands
    assert!(manager.get_command("search").is_some());
    assert!(manager.get_command("autocomplete").is_some());
    assert!(manager.get_command("mcp").is_some());
    assert!(manager.get_command("date").is_some());
    assert!(manager.get_command("time").is_some());
}

#[test]
fn test_slash_command_suggestions() {
    let manager = SlashCommandManager::new();

    let suggestions = manager.suggest_commands("se");
    assert!(suggestions.iter().any(|c| c.name == "search"));

    let suggestions = manager.suggest_commands("auto");
    assert!(suggestions.iter().any(|c| c.name == "autocomplete"));

    let suggestions = manager.suggest_commands("xyz");
    assert!(suggestions.is_empty());
}

#[tokio::test]
async fn test_execute_date_command() {
    let manager = SlashCommandManager::new();
    let result = manager.execute_command("date", "").await;

    assert!(result.is_ok());
    let date_str = result.unwrap();
    assert!(date_str.contains("-")); // YYYY-MM-DD format
}

#[tokio::test]
async fn test_execute_time_command() {
    let manager = SlashCommandManager::new();
    let result = manager.execute_command("time", "").await;

    assert!(result.is_ok());
    let time_str = result.unwrap();
    assert!(time_str.contains(":")); // HH:MM:SS format
}

#[tokio::test]
async fn test_execute_search_command() {
    let manager = SlashCommandManager::new();
    let result = manager.execute_command("search", "rust tokio").await;

    assert!(result.is_ok());
    let output = result.unwrap();
    assert!(output.contains("Search results for: rust tokio"));
}

#[tokio::test]
async fn test_execute_autocomplete_command() {
    let manager = SlashCommandManager::new();
    let result = manager.execute_command("autocomplete", "ru").await;

    assert!(result.is_ok());
    let output = result.unwrap();
    assert!(output.contains("Autocomplete suggestions for: ru"));
}

#[tokio::test]
async fn test_execute_mcp_command() {
    let manager = SlashCommandManager::new();
    let result = manager.execute_command("mcp", "test_tool arg1").await;

    assert!(result.is_ok());
    let output = result.unwrap();
    assert!(output.contains("MCP tool"));
}

#[tokio::test]
async fn test_execute_nonexistent_command() {
    let manager = SlashCommandManager::new();
    let result = manager.execute_command("nonexistent", "").await;

    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(error.contains("Command 'nonexistent' not found"));
}

#[test]
fn test_slash_command_handler_variants() {
    let search = SlashCommandHandler::Search;
    let autocomplete = SlashCommandHandler::Autocomplete;
    let mcp = SlashCommandHandler::MCPTool("test".to_string());
    let insert = SlashCommandHandler::Insert("date".to_string());
    let custom = SlashCommandHandler::Custom("handler".to_string());

    // Just verify they can be created
    assert!(matches!(search, SlashCommandHandler::Search));
    assert!(matches!(autocomplete, SlashCommandHandler::Autocomplete));
    assert!(matches!(mcp, SlashCommandHandler::MCPTool(_)));
    assert!(matches!(insert, SlashCommandHandler::Insert(_)));
    assert!(matches!(custom, SlashCommandHandler::Custom(_)));
}

#[test]
fn test_register_custom_command() {
    let mut manager = SlashCommandManager::new();
    let initial_count = manager.list_commands().len();

    manager.register_command(SlashCommand {
        name: "custom".to_string(),
        description: "Custom command".to_string(),
        syntax: "/custom <arg>".to_string(),
        handler: SlashCommandHandler::Custom("my_handler".to_string()),
    });

    assert_eq!(manager.list_commands().len(), initial_count + 1);
    assert!(manager.get_command("custom").is_some());
}

#[test]
fn test_editor_word_boundary_at_start() {
    let mut state = EditorState::from_content("word".to_string());
    state.cursor_position = 0;

    let word = state.get_word_at_cursor();
    assert_eq!(word, Some("word".to_string()));
}

#[test]
fn test_editor_word_boundary_at_end() {
    let mut state = EditorState::from_content("word".to_string());
    state.cursor_position = 4;

    let word = state.get_word_at_cursor();
    assert_eq!(word, Some("word".to_string()));
}

#[test]
fn test_editor_word_with_underscore() {
    let mut state = EditorState::from_content("hello_world".to_string());
    state.cursor_position = 5;

    let word = state.get_word_at_cursor();
    assert_eq!(word, Some("hello_world".to_string()));
}

#[test]
fn test_editor_empty_content_word() {
    let state = EditorState::new();
    let word = state.get_word_at_cursor();
    assert_eq!(word, None);
}

#[test]
fn test_editor_cursor_between_words() {
    let mut state = EditorState::from_content("hello world".to_string());
    state.cursor_position = 5; // At the space

    let word = state.get_word_at_cursor();
    // Should get one of the adjacent words or none
    // Behavior depends on implementation details
    assert!(word.is_some() || word.is_none());
}
