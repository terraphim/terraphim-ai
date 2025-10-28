//! Command registry for managing markdown-defined commands
//!
//! This module provides the command registry that handles loading, storing, and managing
//! command definitions discovered from markdown files.

use super::{CommandRegistryError, ParsedCommand};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Command registry that manages all discovered commands
#[derive(Debug)]
pub struct CommandRegistry {
    /// Registry storage
    commands: Arc<RwLock<HashMap<String, Arc<ParsedCommand>>>>,
    /// Command aliases mapping
    aliases: Arc<RwLock<HashMap<String, String>>>,
    /// Category-based command mapping
    categories: Arc<RwLock<HashMap<String, Vec<String>>>>,
    /// Command parser
    parser: super::markdown_parser::MarkdownCommandParser,
    /// Command directories to watch
    command_directories: Vec<PathBuf>,
}

impl CommandRegistry {
    /// Create a new command registry
    pub fn new() -> Result<Self, CommandRegistryError> {
        let parser = super::markdown_parser::MarkdownCommandParser::new()?;

        Ok(Self {
            commands: Arc::new(RwLock::new(HashMap::new())),
            aliases: Arc::new(RwLock::new(HashMap::new())),
            categories: Arc::new(RwLock::new(HashMap::new())),
            parser,
            command_directories: Vec::new(),
        })
    }

    /// Add a command directory to watch
    pub fn add_command_directory<P: AsRef<Path>>(&mut self, directory: P) {
        self.command_directories
            .push(directory.as_ref().to_path_buf());
    }

    /// Load all commands from configured directories
    pub async fn load_all_commands(&self) -> Result<usize, CommandRegistryError> {
        let mut total_loaded = 0;

        for directory in &self.command_directories {
            if directory.exists() {
                match self.parser.parse_directory(directory).await {
                    Ok(commands) => {
                        for command in commands {
                            self.register_command(command).await?;
                            total_loaded += 1;
                        }
                    }
                    Err(e) => {
                        eprintln!(
                            "Warning: Failed to load commands from {}: {}",
                            directory.display(),
                            e
                        );
                    }
                }
            } else {
                eprintln!(
                    "Warning: Command directory does not exist: {}",
                    directory.display()
                );
            }
        }

        Ok(total_loaded)
    }

    /// Register a single command in the registry
    pub async fn register_command(
        &self,
        command: ParsedCommand,
    ) -> Result<(), CommandRegistryError> {
        let name = command.definition.name.clone();

        // Check for duplicates
        {
            let commands = self.commands.read().await;
            if commands.contains_key(&name) {
                return Err(CommandRegistryError::DuplicateCommand(name));
            }
        }

        // Register the command
        {
            let mut commands = self.commands.write().await;
            commands.insert(name.clone(), Arc::new(command.clone()));
        }

        // Register aliases
        {
            let mut aliases = self.aliases.write().await;
            for alias in &command.definition.aliases {
                aliases.insert(alias.clone(), name.clone());
            }
        }

        // Register category
        if let Some(ref category) = command.definition.category {
            let mut categories = self.categories.write().await;
            categories
                .entry(category.clone())
                .or_insert_with(Vec::new)
                .push(name.clone());
        }

        Ok(())
    }

    /// Get registry statistics
    pub async fn get_stats(&self) -> RegistryStats {
        let commands = self.commands.read().await;
        let categories = self.categories.read().await;

        RegistryStats {
            total_commands: commands.len(),
            total_categories: categories.len(),
        }
    }

    /// List all registered commands
    pub async fn list_all_commands(&self) -> Vec<super::CommandDefinition> {
        let commands = self.commands.read().await;
        commands
            .values()
            .map(|cmd| cmd.definition.clone())
            .collect()
    }

    /// Get a specific command by name
    pub async fn get_command(&self, name: &str) -> Option<super::CommandDefinition> {
        let commands = self.commands.read().await;
        commands.get(name).map(|cmd| cmd.definition.clone())
    }
}

/// Registry statistics
#[derive(Debug, Clone)]
pub struct RegistryStats {
    pub total_commands: usize,
    pub total_categories: usize,
}

impl Default for CommandRegistry {
    fn default() -> Self {
        Self::new().expect("Failed to create CommandRegistry")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Import required types for testing
    use crate::commands::{CommandDefinition, ExecutionMode, ParsedCommand};

    #[tokio::test]
    async fn test_duplicate_command() {
        let registry = CommandRegistry::new().unwrap();

        let command = ParsedCommand {
            definition: CommandDefinition {
                name: "test".to_string(),
                description: "Test command".to_string(),
                execution_mode: ExecutionMode::Local,
                ..Default::default()
            },
        };

        assert!(registry.register_command(command.clone()).await.is_ok());
        assert!(registry.register_command(command).await.is_err());
    }
}
