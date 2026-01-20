use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Editor state for markdown editing
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EditorState {
    pub content: String,
    pub cursor_position: usize,
    pub selection: Option<(usize, usize)>,
    pub modified: bool,
}

/// Slash command definition
#[derive(Clone, Debug)]
pub struct SlashCommand {
    pub name: String,
    pub description: String,
    pub syntax: String,
    pub handler: SlashCommandHandler,
}

#[derive(Clone, Debug)]
pub enum SlashCommandHandler {
    Search,
    Autocomplete,
    MCPTool(String),
    Insert(String),
    Custom(String),
}

/// Slash command manager
#[derive(Clone)]
pub struct SlashCommandManager {
    commands: HashMap<String, SlashCommand>,
}

impl SlashCommandManager {
    pub fn new() -> Self {
        let mut manager = Self {
            commands: HashMap::new(),
        };

        // Register built-in commands
        manager.register_builtin_commands();
        manager
    }

    fn register_builtin_commands(&mut self) {
        // Search command
        self.register_command(SlashCommand {
            name: "search".to_string(),
            description: "Search knowledge graph".to_string(),
            syntax: "/search <query>".to_string(),
            handler: SlashCommandHandler::Search,
        });

        // Autocomplete command
        self.register_command(SlashCommand {
            name: "autocomplete".to_string(),
            description: "Get term suggestions".to_string(),
            syntax: "/autocomplete <prefix>".to_string(),
            handler: SlashCommandHandler::Autocomplete,
        });

        // MCP tool commands
        self.register_command(SlashCommand {
            name: "mcp".to_string(),
            description: "Execute MCP tool".to_string(),
            syntax: "/mcp <tool_name> [args]".to_string(),
            handler: SlashCommandHandler::MCPTool("generic".to_string()),
        });

        // Insert date
        self.register_command(SlashCommand {
            name: "date".to_string(),
            description: "Insert current date".to_string(),
            syntax: "/date".to_string(),
            handler: SlashCommandHandler::Insert("date".to_string()),
        });

        // Insert time
        self.register_command(SlashCommand {
            name: "time".to_string(),
            description: "Insert current time".to_string(),
            syntax: "/time".to_string(),
            handler: SlashCommandHandler::Insert("time".to_string()),
        });

        log::info!("Registered {} slash commands", self.commands.len());
    }

    pub fn register_command(&mut self, command: SlashCommand) {
        self.commands.insert(command.name.clone(), command);
    }

    pub fn get_command(&self, name: &str) -> Option<&SlashCommand> {
        self.commands.get(name)
    }

    pub fn list_commands(&self) -> Vec<&SlashCommand> {
        self.commands.values().collect()
    }

    pub fn suggest_commands(&self, prefix: &str) -> Vec<&SlashCommand> {
        self.commands
            .values()
            .filter(|cmd| cmd.name.starts_with(prefix))
            .collect()
    }

    pub async fn execute_command(&self, command_name: &str, args: &str) -> Result<String, String> {
        let command = self
            .get_command(command_name)
            .ok_or_else(|| format!("Command '{}' not found", command_name))?;

        log::info!("Executing slash command: /{} {}", command_name, args);

        match &command.handler {
            SlashCommandHandler::Search => {
                // Would integrate with search service
                Ok(format!("Search results for: {}", args))
            }
            SlashCommandHandler::Autocomplete => {
                // Would integrate with autocomplete engine
                Ok(format!("Autocomplete suggestions for: {}", args))
            }
            SlashCommandHandler::MCPTool(tool) => {
                // Would integrate with MCP server
                Ok(format!("MCP tool '{}' executed with args: {}", tool, args))
            }
            SlashCommandHandler::Insert(insert_type) => match insert_type.as_str() {
                "date" => Ok(chrono::Local::now().format("%Y-%m-%d").to_string()),
                "time" => Ok(chrono::Local::now().format("%H:%M:%S").to_string()),
                _ => Ok(format!("Insert: {}", insert_type)),
            },
            SlashCommandHandler::Custom(handler) => {
                Ok(format!("Custom handler '{}' with args: {}", handler, args))
            }
        }
    }
}

impl Default for SlashCommandManager {
    fn default() -> Self {
        Self::new()
    }
}

impl EditorState {
    pub fn new() -> Self {
        Self {
            content: String::new(),
            cursor_position: 0,
            selection: None,
            modified: false,
        }
    }

    pub fn from_content(content: String) -> Self {
        Self {
            content,
            cursor_position: 0,
            selection: None,
            modified: false,
        }
    }

    pub fn insert_text(&mut self, text: &str) {
        if let Some((start, end)) = self.selection {
            // Replace selection
            self.content.replace_range(start..end, text);
            self.cursor_position = start + text.len();
            self.selection = None;
        } else {
            // Insert at cursor
            self.content.insert_str(self.cursor_position, text);
            self.cursor_position += text.len();
        }
        self.modified = true;
    }

    pub fn delete_selection(&mut self) {
        if let Some((start, end)) = self.selection {
            self.content.replace_range(start..end, "");
            self.cursor_position = start;
            self.selection = None;
            self.modified = true;
        }
    }

    pub fn get_word_at_cursor(&self) -> Option<String> {
        if self.content.is_empty() {
            return None;
        }

        let start = self.content[..self.cursor_position]
            .rfind(|c: char| !c.is_alphanumeric() && c != '_')
            .map(|i| i + 1)
            .unwrap_or(0);

        let end = self.content[self.cursor_position..]
            .find(|c: char| !c.is_alphanumeric() && c != '_')
            .map(|i| self.cursor_position + i)
            .unwrap_or(self.content.len());

        if start < end {
            Some(self.content[start..end].to_string())
        } else {
            None
        }
    }

    pub fn line_count(&self) -> usize {
        self.content.lines().count().max(1)
    }

    pub fn is_empty(&self) -> bool {
        self.content.is_empty()
    }

    pub fn char_count(&self) -> usize {
        self.content.chars().count()
    }

    pub fn get_content(&self) -> String {
        self.content.clone()
    }

    pub fn clear(&mut self) {
        self.content.clear();
        self.cursor_position = 0;
        self.selection = None;
        self.modified = true;
    }

    pub fn is_modified(&self) -> bool {
        self.modified
    }
}

impl Default for EditorState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_editor_state() {
        let mut state = EditorState::new();
        assert!(state.is_empty());

        state.insert_text("Hello world");
        assert_eq!(state.content, "Hello world");
        assert_eq!(state.cursor_position, 11);
        assert!(state.modified);
    }

    #[test]
    fn test_slash_command_manager() {
        let manager = SlashCommandManager::new();
        assert!(manager.commands.len() > 0);

        let search_cmd = manager.get_command("search");
        assert!(search_cmd.is_some());
        assert_eq!(search_cmd.unwrap().name, "search");
    }

    #[test]
    fn test_slash_command_suggestions() {
        let manager = SlashCommandManager::new();
        let suggestions = manager.suggest_commands("se");
        assert!(suggestions.iter().any(|c| c.name == "search"));
    }

    #[tokio::test]
    async fn test_execute_date_command() {
        let manager = SlashCommandManager::new();
        let result = manager.execute_command("date", "").await;
        assert!(result.is_ok());
        assert!(result.unwrap().contains("-")); // Date format
    }
}
