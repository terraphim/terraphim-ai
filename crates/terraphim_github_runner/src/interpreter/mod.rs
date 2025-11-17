//! LLM-powered action interpretation
//!
//! This module translates GitHub Actions and shell commands into
//! VM-executable commands using semantic understanding.

pub mod semantic_analyzer;
pub mod command_translator;

pub use semantic_analyzer::SemanticAnalyzer;
pub use command_translator::CommandTranslator;

use crate::{InterpretedAction, RunnerResult, Step};
use crate::knowledge_graph::ActionGraph;

/// Action interpreter combining semantic analysis and command translation
pub struct ActionInterpreter {
    /// Semantic analyzer for understanding action purpose
    analyzer: SemanticAnalyzer,
    /// Command translator for generating executable commands
    translator: CommandTranslator,
    /// Action graph for context
    graph: Option<ActionGraph>,
}

impl ActionInterpreter {
    /// Create a new action interpreter
    pub fn new() -> Self {
        Self {
            analyzer: SemanticAnalyzer::new(),
            translator: CommandTranslator::new(),
            graph: None,
        }
    }

    /// Set the action graph for context
    pub fn with_graph(mut self, graph: ActionGraph) -> Self {
        self.graph = Some(graph);
        self
    }

    /// Configure LLM for semantic analysis
    pub fn with_llm(mut self, provider: &str, model: &str, base_url: Option<&str>) -> Self {
        self.analyzer = self.analyzer.with_llm(provider, model, base_url);
        self
    }

    /// Interpret a workflow step
    pub async fn interpret_step(&self, step: &Step) -> RunnerResult<InterpretedAction> {
        // First, try to translate using known patterns
        if let Some(uses) = &step.uses {
            let interpreted = self.translator.translate_action(uses, step.with.as_ref())?;

            // If confidence is low and LLM is available, enhance with semantic analysis
            if interpreted.confidence < 0.8 && self.analyzer.has_llm() {
                return self.analyzer.analyze_action(uses, &interpreted).await;
            }

            return Ok(interpreted);
        }

        // For run commands, translate directly
        if let Some(run) = &step.run {
            let shell = step.shell.as_deref().unwrap_or("bash");
            return self.translator.translate_command(run, shell);
        }

        // No action or command
        Ok(InterpretedAction {
            original: "empty step".to_string(),
            purpose: "No operation".to_string(),
            prerequisites: Vec::new(),
            produces: Vec::new(),
            cacheable: false,
            commands: Vec::new(),
            required_env: Vec::new(),
            kg_terms: Vec::new(),
            confidence: 1.0,
        })
    }

    /// Interpret multiple steps and validate sequence
    pub async fn interpret_steps(&self, steps: &[Step]) -> RunnerResult<Vec<InterpretedAction>> {
        let mut results = Vec::new();

        for step in steps {
            let interpreted = self.interpret_step(step).await?;
            results.push(interpreted);
        }

        // Validate sequence against graph if available
        if let Some(graph) = &self.graph {
            let terms: Vec<_> = results.iter().map(|i| i.original.clone()).collect();
            if !graph.is_sequence_valid(&terms) {
                log::warn!("Step sequence may have missing dependencies");
            }
        }

        Ok(results)
    }
}

impl Default for ActionInterpreter {
    fn default() -> Self {
        Self::new()
    }
}
