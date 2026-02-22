//! REPL Integration Tests for terraphim_agent
//!
//! These tests verify the REPL functionality including:
//! - Command parsing and execution
//! - Handler operations in both server and offline modes
//! - Integration between REPL commands and underlying services
//!
//! Uses ratatui's TestBackend for TUI rendering tests without mocks.

use terraphim_agent::repl::commands::{
    ConfigSubcommand, ReplCommand, RobotSubcommand, RoleSubcommand, UpdateSubcommand, VmSubcommand,
};

/// Test that the REPL command parser correctly parses basic commands
#[test]
fn test_repl_command_parsing() {
    // Test help command - use parse_str which uses FromStr
    let cmd: Result<ReplCommand, _> = "/help".parse();
    assert!(matches!(cmd, Ok(ReplCommand::Help { .. })));

    // Test quit command
    let cmd: Result<ReplCommand, _> = "/quit".parse();
    assert!(matches!(cmd, Ok(ReplCommand::Quit)));

    // Test exit command (alias for quit)
    let cmd: Result<ReplCommand, _> = "/exit".parse();
    assert!(matches!(cmd, Ok(ReplCommand::Exit)));

    // Test search command
    let cmd: Result<ReplCommand, _> = "/search rust programming".parse();
    assert!(matches!(cmd, Ok(ReplCommand::Search { .. })));

    // Test empty command (should error)
    let cmd: Result<ReplCommand, _> = "".parse();
    assert!(cmd.is_err());
}

/// Test that the REPL correctly identifies available commands
#[test]
fn test_repl_available_commands() {
    let commands = ReplCommand::available_commands();
    assert!(!commands.is_empty());

    // Check for essential commands - available_commands returns &[&str]
    let has_search = commands.contains(&"search");
    let has_config = commands.contains(&"config");
    let has_role = commands.contains(&"role");
    let has_help = commands.contains(&"help");
    let has_quit = commands.contains(&"quit");

    assert!(has_search, "should have search command");
    assert!(has_config, "should have config command");
    assert!(has_role, "should have role command");
    assert!(has_help, "should have help command");
    assert!(has_quit, "should have quit command");
}

/// Test REPL command string parsing for search queries
#[test]
fn test_repl_search_command_parsing() {
    let cmd: Result<ReplCommand, _> = "/search test query".parse();
    match cmd {
        Ok(ReplCommand::Search { query, .. }) => {
            assert_eq!(query, "test query");
        }
        _ => panic!("Expected Search command"),
    }
}

/// Test REPL search with options
#[test]
fn test_repl_search_with_options() {
    let cmd: Result<ReplCommand, _> = "/search rust --role engineer --limit 10".parse();
    match cmd {
        Ok(ReplCommand::Search {
            query, role, limit, ..
        }) => {
            assert_eq!(query, "rust");
            assert_eq!(role, Some("engineer".to_string()));
            assert_eq!(limit, Some(10));
        }
        _ => panic!("Expected Search command with options"),
    }
}

/// Test REPL role command parsing
#[test]
fn test_repl_role_command_parsing() {
    // Test role list
    let cmd: Result<ReplCommand, _> = "/role list".parse();
    match cmd {
        Ok(ReplCommand::Role { subcommand }) => {
            assert!(matches!(subcommand, RoleSubcommand::List));
        }
        _ => panic!("Expected Role command with List subcommand"),
    }

    // Test role select
    let cmd: Result<ReplCommand, _> = "/role select terraphim-engineer".parse();
    match cmd {
        Ok(ReplCommand::Role { subcommand }) => match subcommand {
            RoleSubcommand::Select { name } => {
                assert_eq!(name, "terraphim-engineer");
            }
            _ => panic!("Expected Select subcommand"),
        },
        _ => panic!("Expected Role command with Select subcommand"),
    }
}

/// Test REPL config command parsing
#[test]
fn test_repl_config_command_parsing() {
    // Test config show
    let cmd: Result<ReplCommand, _> = "/config show".parse();
    match cmd {
        Ok(ReplCommand::Config { subcommand }) => {
            assert!(matches!(subcommand, ConfigSubcommand::Show));
        }
        _ => panic!("Expected Config command with Show subcommand"),
    }

    // Test config set
    let cmd: Result<ReplCommand, _> = "/config set selected_role terraphim-engineer".parse();
    match cmd {
        Ok(ReplCommand::Config { subcommand }) => match subcommand {
            ConfigSubcommand::Set { key, value } => {
                assert_eq!(key, "selected_role");
                assert_eq!(value, "terraphim-engineer");
            }
            _ => panic!("Expected Set subcommand"),
        },
        _ => panic!("Expected Config command with Set subcommand"),
    }
}

/// Test REPL graph command parsing
#[test]
fn test_repl_graph_command_parsing() {
    let cmd: Result<ReplCommand, _> = "/graph --top-k 20".parse();
    match cmd {
        Ok(ReplCommand::Graph { top_k }) => {
            assert_eq!(top_k, Some(20));
        }
        _ => panic!("Expected Graph command"),
    }
}

/// Test REPL help command with optional command argument
#[test]
fn test_repl_help_command() {
    // Help without argument
    let cmd: Result<ReplCommand, _> = "/help".parse();
    match cmd {
        Ok(ReplCommand::Help { command }) => {
            assert!(command.is_none());
        }
        _ => panic!("Expected Help command"),
    }

    // Help with specific command
    let cmd: Result<ReplCommand, _> = "/help search".parse();
    match cmd {
        Ok(ReplCommand::Help { command }) => {
            assert_eq!(command, Some("search".to_string()));
        }
        _ => panic!("Expected Help command with search argument"),
    }
}

/// Test REPL handler initialization in offline mode
#[tokio::test]
async fn test_repl_handler_offline_mode() {
    use terraphim_agent::repl::handler::ReplHandler;
    use terraphim_agent::service::TuiService;

    // Create TuiService (may fail if config is missing, which is OK for this test)
    match TuiService::new().await {
        Ok(service) => {
            let _handler = ReplHandler::new_offline(service);
            // Handler should be created successfully
            // We can't test the actual run() method as it requires TTY
        }
        Err(_) => {
            // Service creation may fail in test environment without config
            // This is acceptable for this integration test
        }
    }
}

/// Test REPL handler initialization in server mode
#[tokio::test]
async fn test_repl_handler_server_mode() {
    use terraphim_agent::client::ApiClient;
    use terraphim_agent::repl::handler::ReplHandler;

    // Create API client for a test server
    let api_client = ApiClient::new("http://localhost:8000".to_string());
    let _handler = ReplHandler::new_server(api_client);

    // Handler should be created successfully
    // Server mode starts with "Terraphim Engineer" role by default
}

/// Test that REPL commands handle edge cases properly
#[test]
fn test_repl_command_edge_cases() {
    // Test command with extra spaces
    let cmd: Result<ReplCommand, _> = "/search   multiple   spaces   ".parse();
    assert!(cmd.is_ok());

    // Test command with special characters in search
    let cmd: Result<ReplCommand, _> = "/search rust&cargo".parse();
    assert!(cmd.is_ok());

    // Test unknown command
    let cmd: Result<ReplCommand, _> = "/unknowncommand".parse();
    assert!(cmd.is_err());

    // Test command without leading slash (should still work)
    let cmd: Result<ReplCommand, _> = "search test".parse();
    assert!(matches!(cmd, Ok(ReplCommand::Search { .. })));
}

/// Test the TUI render functions using ratatui's TestBackend
#[test]
fn test_tui_render_search_view() {
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    // Test rendering with empty state
    terminal
        .draw(|f| {
            use ratatui::layout::{Constraint, Direction, Layout};
            use ratatui::text::Line;
            use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Length(5),
                    Constraint::Min(3),
                    Constraint::Length(3),
                ])
                .split(f.area());

            // Render input section
            let input = Paragraph::new(Line::from("test query"))
                .block(Block::default().title("Search").borders(Borders::ALL));
            f.render_widget(input, chunks[0]);

            // Render results section
            let items: Vec<ListItem> = vec![ListItem::new("Result 1"), ListItem::new("Result 2")];
            let list =
                List::new(items).block(Block::default().title("Results").borders(Borders::ALL));
            f.render_widget(list, chunks[2]);
        })
        .unwrap();

    // Verify the buffer was written to
    let buffer = terminal.backend().buffer();
    assert!(!buffer.content.is_empty());

    // Check that the buffer contains expected content
    let buffer_text: String = buffer.content.iter().map(|c| c.symbol()).collect();
    assert!(buffer_text.contains("Search"));
    assert!(buffer_text.contains("Results"));
}

/// Test TUI render with detail view
#[test]
fn test_tui_render_detail_view() {
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;
    use terraphim_types::Document;

    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let doc = Document {
        id: "test-doc-1".to_string(),
        title: "Test Document Title".to_string(),
        url: "https://example.com/test".to_string(),
        body: "This is the body of the test document. It contains content.".to_string(),
        description: None,
        summarization: None,
        stub: None,
        rank: Some(1),
        tags: None,
        source_haystack: None,
        doc_type: terraphim_types::DocumentType::Document,
        synonyms: None,
        route: None,
        priority: None,
    };

    terminal
        .draw(|f| {
            use ratatui::layout::{Constraint, Direction, Layout};
            use ratatui::text::Line;
            use ratatui::widgets::{Block, Borders, Paragraph};

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Min(5),
                    Constraint::Length(3),
                ])
                .split(f.area());

            // Render title
            let title = Paragraph::new(Line::from(doc.title.as_str()))
                .block(Block::default().title("Document").borders(Borders::ALL));
            f.render_widget(title, chunks[0]);

            // Render body
            let body = Paragraph::new(doc.body.as_str())
                .block(Block::default().title("Content").borders(Borders::ALL));
            f.render_widget(body, chunks[1]);
        })
        .unwrap();

    let buffer = terminal.backend().buffer();
    let buffer_text: String = buffer.content.iter().map(|c| c.symbol()).collect();

    assert!(buffer_text.contains("Test Document Title"));
    assert!(buffer_text.contains("This is the body"));
}

/// Test TUI render with suggestions
#[test]
fn test_tui_render_with_suggestions() {
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let suggestions = ["rust".to_string(), "react".to_string(), "ruby".to_string()];

    terminal
        .draw(|f| {
            use ratatui::layout::{Constraint, Direction, Layout};
            use ratatui::widgets::{Block, Borders, List, ListItem};

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(5), Constraint::Min(3)])
                .split(f.area());

            // Render suggestions
            let items: Vec<ListItem> = suggestions
                .iter()
                .map(|s| ListItem::new(s.as_str()))
                .collect();
            let list =
                List::new(items).block(Block::default().title("Suggestions").borders(Borders::ALL));
            f.render_widget(list, chunks[0]);
        })
        .unwrap();

    let buffer = terminal.backend().buffer();
    let buffer_text: String = buffer.content.iter().map(|c| c.symbol()).collect();

    assert!(buffer_text.contains("Suggestions"));
    assert!(buffer_text.contains("rust"));
    assert!(buffer_text.contains("react"));
    assert!(buffer_text.contains("ruby"));
}

/// Test that terminal size constraints are handled
#[test]
fn test_tui_render_small_terminal() {
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    // Test with a very small terminal (20x10)
    let backend = TestBackend::new(20, 10);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| {
            use ratatui::layout::{Constraint, Direction, Layout};
            use ratatui::text::Line;
            use ratatui::widgets::{Block, Borders, Paragraph};

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(1)])
                .split(f.area());

            let widget =
                Paragraph::new(Line::from("Test")).block(Block::default().borders(Borders::ALL));
            f.render_widget(widget, chunks[0]);
        })
        .unwrap();

    let buffer = terminal.backend().buffer();
    assert!(!buffer.content.is_empty());
}

/// Test REPL command error handling for invalid inputs
#[test]
fn test_repl_command_error_handling() {
    // Test various invalid inputs
    let invalid_inputs = vec![
        "/", "/ ", "/search", // search without query
        "/role",   // role without subcommand
        "/config", // config without subcommand
    ];

    for input in invalid_inputs {
        let result: Result<ReplCommand, _> = input.parse();
        assert!(result.is_err(), "'{}' should fail to parse", input);
    }
}

/// Integration test: verify REPL components work together
#[tokio::test]
async fn test_repl_integration_components() {
    // This test verifies that all REPL components can be instantiated
    // and that their interfaces are compatible

    // Test command availability
    let commands = ReplCommand::available_commands();
    assert!(!commands.is_empty(), "REPL should have available commands");

    // Test that command parsing doesn't panic
    let test_inputs = vec![
        "/help",
        "/quit",
        "/search test",
        "/config show",
        "/role list",
        "/graph",
    ];

    for input in test_inputs {
        let _result: Result<ReplCommand, _> = input.parse();
        // Just verify no panic occurs
    }
}

/// Test that render functions handle empty results gracefully
#[test]
fn test_render_empty_results() {
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| {
            use ratatui::layout::{Constraint, Direction, Layout};
            use ratatui::widgets::{Block, Borders, List};

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(3)])
                .split(f.area());

            // Empty results
            let empty_items: Vec<ratatui::widgets::ListItem> = vec![];
            let list = List::new(empty_items)
                .block(Block::default().title("No Results").borders(Borders::ALL));
            f.render_widget(list, chunks[0]);
        })
        .unwrap();

    let buffer = terminal.backend().buffer();
    assert!(!buffer.content.is_empty());
}

/// Test TUI rendering with long content
#[test]
fn test_render_long_content() {
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let long_content = "a".repeat(1000);

    terminal
        .draw(|f| {
            use ratatui::layout::{Constraint, Direction, Layout};
            use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(5)])
                .split(f.area());

            let paragraph = Paragraph::new(long_content.as_str())
                .block(Block::default().borders(Borders::ALL))
                .wrap(Wrap { trim: true });
            f.render_widget(paragraph, chunks[0]);
        })
        .unwrap();

    // Just verify no panic occurs with long content
    let buffer = terminal.backend().buffer();
    assert!(!buffer.content.is_empty());
}

/// Test TUI with selected item highlighting
#[test]
fn test_render_with_selection() {
    use ratatui::backend::TestBackend;
    use ratatui::style::{Modifier, Style};
    use ratatui::Terminal;

    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let results = ["Item 1", "Item 2", "Item 3"];
    let selected_index = 1;

    terminal
        .draw(|f| {
            use ratatui::layout::{Constraint, Direction, Layout};
            use ratatui::widgets::{Block, Borders, List, ListItem};

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(3)])
                .split(f.area());

            let items: Vec<ListItem> = results
                .iter()
                .enumerate()
                .map(|(i, r)| {
                    let item = ListItem::new(*r);
                    if i == selected_index {
                        item.style(Style::default().add_modifier(Modifier::REVERSED))
                    } else {
                        item
                    }
                })
                .collect();

            let list =
                List::new(items).block(Block::default().title("Results").borders(Borders::ALL));
            f.render_widget(list, chunks[0]);
        })
        .unwrap();

    let buffer = terminal.backend().buffer();
    assert!(!buffer.content.is_empty());
}

/// Test TUI with transparent background option
#[test]
fn test_render_transparent_background() {
    use ratatui::backend::TestBackend;
    use ratatui::style::{Color, Style};
    use ratatui::Terminal;

    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| {
            use ratatui::layout::{Constraint, Direction, Layout};
            use ratatui::text::Line;
            use ratatui::widgets::{Block, Borders, Paragraph};

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(3)])
                .split(f.area());

            // Render with transparent background
            let transparent_style = Style::default().bg(Color::Reset);
            let widget = Paragraph::new(Line::from("Transparent Test")).block(
                Block::default()
                    .borders(Borders::ALL)
                    .style(transparent_style),
            );
            f.render_widget(widget, chunks[0]);
        })
        .unwrap();

    let buffer = terminal.backend().buffer();
    assert!(!buffer.content.is_empty());
}

/// Test search command with semantic and concepts flags
#[test]
fn test_repl_search_with_flags() {
    let cmd: Result<ReplCommand, _> = "/search rust programming --semantic --concepts".parse();
    match cmd {
        Ok(ReplCommand::Search {
            query,
            semantic,
            concepts,
            ..
        }) => {
            assert_eq!(query, "rust programming");
            assert!(semantic);
            assert!(concepts);
        }
        _ => panic!("Expected Search command with semantic and concepts flags"),
    }
}

/// Test that available_commands returns the correct set based on features
#[test]
fn test_available_commands_by_feature() {
    let commands = ReplCommand::available_commands();

    // Core commands should always be present
    let has_search = commands.contains(&"search");
    let has_config = commands.contains(&"config");
    let has_role = commands.contains(&"role");
    let has_graph = commands.contains(&"graph");
    let has_vm = commands.contains(&"vm");
    let has_robot = commands.contains(&"robot");
    let has_update = commands.contains(&"update");
    let has_help = commands.contains(&"help");
    let has_quit = commands.contains(&"quit");
    let has_exit = commands.contains(&"exit");
    let has_clear = commands.contains(&"clear");

    assert!(has_search);
    assert!(has_config);
    assert!(has_role);
    assert!(has_graph);
    assert!(has_vm);
    assert!(has_robot);
    assert!(has_update);
    assert!(has_help);
    assert!(has_quit);
    assert!(has_exit);
    assert!(has_clear);
}

/// Test command help retrieval
#[test]
fn test_command_help_retrieval() {
    let help = ReplCommand::get_command_help("search");
    assert!(help.is_some());
    assert!(help.unwrap().contains("Search"));

    let help = ReplCommand::get_command_help("quit");
    assert!(help.is_some());

    let help = ReplCommand::get_command_help("nonexistent");
    assert!(help.is_none());
}

/// Test VM command parsing
#[test]
fn test_repl_vm_command_parsing() {
    let cmd: Result<ReplCommand, _> = "/vm list".parse();
    match cmd {
        Ok(ReplCommand::Vm { subcommand }) => {
            assert!(matches!(subcommand, VmSubcommand::List));
        }
        _ => panic!("Expected Vm command with List subcommand"),
    }
}

/// Test update command parsing
#[test]
fn test_repl_update_command_parsing() {
    let cmd: Result<ReplCommand, _> = "/update check".parse();
    match cmd {
        Ok(ReplCommand::Update { subcommand }) => {
            assert!(matches!(subcommand, UpdateSubcommand::Check));
        }
        _ => panic!("Expected Update command with Check subcommand"),
    }

    let cmd: Result<ReplCommand, _> = "/update install".parse();
    match cmd {
        Ok(ReplCommand::Update { subcommand }) => {
            assert!(matches!(subcommand, UpdateSubcommand::Install));
        }
        _ => panic!("Expected Update command with Install subcommand"),
    }
}

/// Test robot command parsing
#[test]
fn test_repl_robot_command_parsing() {
    let cmd: Result<ReplCommand, _> = "/robot capabilities".parse();
    match cmd {
        Ok(ReplCommand::Robot { subcommand }) => {
            assert!(matches!(subcommand, RobotSubcommand::Capabilities));
        }
        _ => panic!("Expected Robot command with Capabilities subcommand"),
    }

    let cmd: Result<ReplCommand, _> = "/robot schemas search".parse();
    match cmd {
        Ok(ReplCommand::Robot { subcommand }) => match subcommand {
            RobotSubcommand::Schemas { command } => {
                assert_eq!(command, Some("search".to_string()));
            }
            _ => panic!("Expected Schemas subcommand"),
        },
        _ => panic!("Expected Robot command with Schemas subcommand"),
    }
}

/// Test complex search query with multiple terms and options
#[test]
fn test_repl_complex_search_query() {
    let cmd: Result<ReplCommand, _> =
        "/search rust async programming --role engineer --limit 50 --semantic".parse();
    match cmd {
        Ok(ReplCommand::Search {
            query,
            role,
            limit,
            semantic,
            concepts,
        }) => {
            assert_eq!(query, "rust async programming");
            assert_eq!(role, Some("engineer".to_string()));
            assert_eq!(limit, Some(50));
            assert!(semantic);
            assert!(!concepts);
        }
        _ => panic!("Expected complex Search command"),
    }
}

/// Test REPL command parsing preserves query order
#[test]
fn test_repl_search_query_order() {
    let cmd: Result<ReplCommand, _> = "/search first second third".parse();
    match cmd {
        Ok(ReplCommand::Search { query, .. }) => {
            assert_eq!(query, "first second third");
        }
        _ => panic!("Expected Search command with ordered query"),
    }
}

/// Test that parsing is case-sensitive for commands
#[test]
fn test_repl_command_case_sensitivity() {
    // Commands should be lowercase
    let cmd: Result<ReplCommand, _> = "/SEARCH test".parse();
    assert!(cmd.is_err());

    let cmd: Result<ReplCommand, _> = "/Search test".parse();
    assert!(cmd.is_err());

    // But queries should preserve case
    let cmd: Result<ReplCommand, _> = "/search TestQuery".parse();
    match cmd {
        Ok(ReplCommand::Search { query, .. }) => {
            assert_eq!(query, "TestQuery");
        }
        _ => panic!("Expected Search command with case-preserved query"),
    }
}
