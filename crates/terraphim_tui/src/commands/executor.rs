//! Command execution engine
//!
//! This module provides the main command execution engine that coordinates
//! between different executors and handles command lifecycle.

use super::{
    CommandDefinition, CommandExecutionError, CommandExecutionResult, HookContext, HookManager,
};
use std::collections::HashMap;
use std::sync::Arc;

/// Main command executor
pub struct CommandExecutor {
    #[allow(dead_code)]
    api_client: Option<crate::client::ApiClient>,
    hook_manager: Arc<HookManager>,
}

impl CommandExecutor {
    /// Create a new command executor
    pub fn new() -> Self {
        Self {
            api_client: None,
            hook_manager: Arc::new(HookManager::new()),
        }
    }

    /// Create a command executor with API client
    pub fn with_api_client(api_client: crate::client::ApiClient) -> Self {
        Self {
            api_client: Some(api_client),
            hook_manager: Arc::new(HookManager::new()),
        }
    }

    /// Create a command executor with custom hook manager
    pub fn with_hooks(mut self, hooks: Vec<Box<dyn super::CommandHook + Send + Sync>>) -> Self {
        for hook in hooks {
            Arc::get_mut(&mut self.hook_manager)
                .unwrap()
                .add_pre_hook(hook);
        }
        self
    }

    /// Execute a command with the given definition and parameters
    pub async fn execute(
        &self,
        definition: &CommandDefinition,
        parameters: &HashMap<String, String>,
    ) -> Result<CommandExecutionResult, CommandExecutionError> {
        self.execute_with_context(definition, parameters, "default", "default", "default", ".")
            .await
    }

    /// Execute a command with full context information
    pub async fn execute_with_context(
        &self,
        definition: &CommandDefinition,
        parameters: &HashMap<String, String>,
        command: &str,
        user: &str,
        role: &str,
        working_directory: &str,
    ) -> Result<CommandExecutionResult, CommandExecutionError> {
        // Create hook context
        let hook_context = HookContext {
            command: command.to_string(),
            parameters: parameters.clone(),
            user: user.to_string(),
            role: role.to_string(),
            execution_mode: definition.execution_mode.clone(),
            working_directory: std::path::PathBuf::from(working_directory),
        };

        // Execute pre-command hooks
        self.hook_manager.execute_pre_hooks(&hook_context).await?;

        // Execute the actual command
        let result = match self.execute_command_internal(definition, parameters).await {
            Ok(result) => {
                // Execute post-command hooks
                if let Err(hook_error) = self
                    .hook_manager
                    .execute_post_hooks(&hook_context, &result)
                    .await
                {
                    // Log hook error but don't fail the entire execution
                    eprintln!("Warning: Post-command hooks failed: {}", hook_error);
                }
                result
            }
            Err(e) => {
                // Execute post-command hooks even on failure
                let failed_result = CommandExecutionResult {
                    command: command.to_string(),
                    execution_mode: definition.execution_mode.clone(),
                    exit_code: 1,
                    stdout: String::new(),
                    stderr: e.to_string(),
                    duration_ms: 0,
                    resource_usage: None,
                };

                if let Err(hook_error) = self
                    .hook_manager
                    .execute_post_hooks(&hook_context, &failed_result)
                    .await
                {
                    eprintln!(
                        "Warning: Post-command hooks failed on error: {}",
                        hook_error
                    );
                }

                return Err(e);
            }
        };

        Ok(result)
    }

    /// Internal command execution without hooks
    async fn execute_command_internal(
        &self,
        definition: &CommandDefinition,
        parameters: &HashMap<String, String>,
    ) -> Result<CommandExecutionResult, CommandExecutionError> {
        // Delegate to the appropriate executor based on execution mode
        let executor = super::modes::create_executor(definition.execution_mode.clone());
        executor.execute_command(definition, parameters).await
    }
}

impl Default for CommandExecutor {
    fn default() -> Self {
        Self::new()
    }
}
