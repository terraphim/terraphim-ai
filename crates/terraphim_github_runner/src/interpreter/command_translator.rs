//! Translation of actions to executable commands

use crate::{InterpretedAction, RunnerResult, RunnerError};
use ahash::AHashMap;

/// Translates GitHub Actions to executable shell commands
pub struct CommandTranslator {
    /// Known action translations
    action_patterns: AHashMap<String, ActionPattern>,
}

/// Pattern for translating an action
struct ActionPattern {
    /// Purpose description
    purpose: &'static str,
    /// Command template (with {{input}} placeholders)
    commands: Vec<&'static str>,
    /// Required inputs
    required_inputs: Vec<&'static str>,
    /// What this action produces
    produces: Vec<&'static str>,
    /// Prerequisites
    prerequisites: Vec<&'static str>,
    /// Whether output is cacheable
    cacheable: bool,
}

impl CommandTranslator {
    /// Create a new command translator
    pub fn new() -> Self {
        let mut translator = Self {
            action_patterns: AHashMap::new(),
        };
        translator.register_builtin_patterns();
        translator
    }

    /// Register built-in action patterns
    fn register_builtin_patterns(&mut self) {
        // actions/checkout
        self.action_patterns.insert(
            "actions/checkout".to_string(),
            ActionPattern {
                purpose: "Checkout repository code",
                commands: vec![
                    "git clone --depth 1 $GITHUB_SERVER_URL/$GITHUB_REPOSITORY .",
                    "git fetch origin $GITHUB_REF",
                    "git checkout FETCH_HEAD",
                ],
                required_inputs: vec![],
                produces: vec!["source code", "git history"],
                prerequisites: vec!["git"],
                cacheable: false,
            },
        );

        // actions/setup-node
        self.action_patterns.insert(
            "actions/setup-node".to_string(),
            ActionPattern {
                purpose: "Install and configure Node.js runtime",
                commands: vec![
                    "curl -fsSL https://nodejs.org/dist/v{{node-version}}/node-v{{node-version}}-linux-x64.tar.gz | tar -xz -C /usr/local --strip-components=1",
                    "node --version",
                    "npm --version",
                ],
                required_inputs: vec!["node-version"],
                produces: vec!["node binary", "npm binary"],
                prerequisites: vec!["curl", "tar"],
                cacheable: true,
            },
        );

        // actions/setup-python
        self.action_patterns.insert(
            "actions/setup-python".to_string(),
            ActionPattern {
                purpose: "Install and configure Python runtime",
                commands: vec![
                    "apt-get update && apt-get install -y python{{python-version}}",
                    "python{{python-version}} --version",
                    "pip --version",
                ],
                required_inputs: vec!["python-version"],
                produces: vec!["python binary", "pip binary"],
                prerequisites: vec!["apt-get"],
                cacheable: true,
            },
        );

        // actions/cache
        self.action_patterns.insert(
            "actions/cache".to_string(),
            ActionPattern {
                purpose: "Cache dependencies for faster builds",
                commands: vec![
                    "# Cache restore: check if cache exists for key {{key}}",
                    "# If cache hit, extract to {{path}}",
                    "# If miss, cache will be saved post-job",
                ],
                required_inputs: vec!["key", "path"],
                produces: vec!["cache-hit output"],
                prerequisites: vec![],
                cacheable: true,
            },
        );

        // actions/upload-artifact
        self.action_patterns.insert(
            "actions/upload-artifact".to_string(),
            ActionPattern {
                purpose: "Upload build artifacts for later use",
                commands: vec![
                    "# Upload {{path}} as artifact {{name}}",
                    "tar -czf /tmp/artifact-{{name}}.tar.gz {{path}}",
                ],
                required_inputs: vec!["name", "path"],
                produces: vec!["artifact upload"],
                prerequisites: vec!["tar"],
                cacheable: false,
            },
        );

        // actions/download-artifact
        self.action_patterns.insert(
            "actions/download-artifact".to_string(),
            ActionPattern {
                purpose: "Download previously uploaded artifacts",
                commands: vec![
                    "# Download artifact {{name}}",
                    "tar -xzf /tmp/artifact-{{name}}.tar.gz -C {{path}}",
                ],
                required_inputs: vec!["name"],
                produces: vec!["downloaded files"],
                prerequisites: vec!["tar"],
                cacheable: false,
            },
        );

        // docker/build-push-action
        self.action_patterns.insert(
            "docker/build-push-action".to_string(),
            ActionPattern {
                purpose: "Build and push Docker image",
                commands: vec![
                    "docker build -t {{tags}} -f {{file}} {{context}}",
                    "docker push {{tags}}",
                ],
                required_inputs: vec!["tags"],
                produces: vec!["docker image"],
                prerequisites: vec!["docker", "Dockerfile"],
                cacheable: false,
            },
        );

        // docker/login-action
        self.action_patterns.insert(
            "docker/login-action".to_string(),
            ActionPattern {
                purpose: "Login to Docker registry",
                commands: vec![
                    "echo '{{password}}' | docker login {{registry}} -u {{username}} --password-stdin",
                ],
                required_inputs: vec!["username", "password"],
                produces: vec!["docker credentials"],
                prerequisites: vec!["docker"],
                cacheable: false,
            },
        );
    }

    /// Translate an action to executable commands
    pub fn translate_action(
        &self,
        action_ref: &str,
        inputs: Option<&AHashMap<String, serde_yaml::Value>>,
    ) -> RunnerResult<InterpretedAction> {
        // Extract action name (without version)
        let action_name = action_ref.split('@').next().unwrap_or(action_ref);

        // Look up pattern
        if let Some(pattern) = self.action_patterns.get(action_name) {
            let mut commands = Vec::new();

            // Substitute inputs into commands
            for cmd_template in &pattern.commands {
                let mut cmd = cmd_template.to_string();

                if let Some(inputs) = inputs {
                    for (key, value) in inputs {
                        let value_str = match value {
                            serde_yaml::Value::String(s) => s.clone(),
                            serde_yaml::Value::Number(n) => n.to_string(),
                            serde_yaml::Value::Bool(b) => b.to_string(),
                            _ => continue,
                        };
                        cmd = cmd.replace(&format!("{{{{{}}}}}", key), &value_str);
                    }
                }

                // Fill in defaults for remaining placeholders
                for required in &pattern.required_inputs {
                    let placeholder = format!("{{{{{}}}}}", required);
                    if cmd.contains(&placeholder) {
                        cmd = cmd.replace(&placeholder, &format!("$INPUT_{}", required.to_uppercase().replace('-', "_")));
                    }
                }

                commands.push(cmd);
            }

            // Extract required env from inputs
            let required_env: Vec<_> = inputs
                .map(|i| {
                    i.keys()
                        .map(|k| format!("INPUT_{}", k.to_uppercase().replace('-', "_")))
                        .collect()
                })
                .unwrap_or_default();

            Ok(InterpretedAction {
                original: action_ref.to_string(),
                purpose: pattern.purpose.to_string(),
                prerequisites: pattern.prerequisites.iter().map(|s| s.to_string()).collect(),
                produces: pattern.produces.iter().map(|s| s.to_string()).collect(),
                cacheable: pattern.cacheable,
                commands,
                required_env,
                kg_terms: vec![format!("uses:{}", action_name)],
                confidence: 0.9,
            })
        } else {
            // Unknown action - return low confidence interpretation
            Ok(InterpretedAction {
                original: action_ref.to_string(),
                purpose: format!("Execute action: {}", action_name),
                prerequisites: Vec::new(),
                produces: Vec::new(),
                cacheable: false,
                commands: vec![format!("# Unknown action: {}", action_ref)],
                required_env: Vec::new(),
                kg_terms: vec![format!("uses:{}", action_name)],
                confidence: 0.3,
            })
        }
    }

    /// Translate a shell command
    pub fn translate_command(&self, command: &str, shell: &str) -> RunnerResult<InterpretedAction> {
        // Parse the command to understand what it does
        let first_word = command
            .trim()
            .split_whitespace()
            .next()
            .unwrap_or("shell");

        let (purpose, prerequisites, produces, cacheable) = match first_word {
            "npm" | "yarn" | "pnpm" => {
                let subcommand = command.split_whitespace().nth(1).unwrap_or("");
                match subcommand {
                    "install" | "ci" => (
                        "Install Node.js dependencies".to_string(),
                        vec!["node".to_string(), "npm".to_string()],
                        vec!["node_modules".to_string()],
                        true,
                    ),
                    "test" => (
                        "Run Node.js tests".to_string(),
                        vec!["node".to_string(), "npm".to_string(), "node_modules".to_string()],
                        vec!["test results".to_string()],
                        false,
                    ),
                    "build" => (
                        "Build Node.js project".to_string(),
                        vec!["node".to_string(), "npm".to_string(), "node_modules".to_string()],
                        vec!["build output".to_string()],
                        false,
                    ),
                    _ => (
                        format!("Run npm {}", subcommand),
                        vec!["node".to_string(), "npm".to_string()],
                        Vec::new(),
                        false,
                    ),
                }
            }
            "cargo" => {
                let subcommand = command.split_whitespace().nth(1).unwrap_or("");
                match subcommand {
                    "build" => (
                        "Build Rust project".to_string(),
                        vec!["rustc".to_string(), "cargo".to_string()],
                        vec!["target directory".to_string()],
                        true,
                    ),
                    "test" => (
                        "Run Rust tests".to_string(),
                        vec!["rustc".to_string(), "cargo".to_string()],
                        vec!["test results".to_string()],
                        false,
                    ),
                    "fmt" => (
                        "Format Rust code".to_string(),
                        vec!["rustfmt".to_string()],
                        Vec::new(),
                        false,
                    ),
                    "clippy" => (
                        "Run Rust linter".to_string(),
                        vec!["clippy".to_string()],
                        vec!["lint results".to_string()],
                        false,
                    ),
                    _ => (
                        format!("Run cargo {}", subcommand),
                        vec!["cargo".to_string()],
                        Vec::new(),
                        false,
                    ),
                }
            }
            "pip" => (
                "Install Python packages".to_string(),
                vec!["python".to_string(), "pip".to_string()],
                vec!["installed packages".to_string()],
                true,
            ),
            "go" => {
                let subcommand = command.split_whitespace().nth(1).unwrap_or("");
                (
                    format!("Run go {}", subcommand),
                    vec!["go".to_string()],
                    Vec::new(),
                    subcommand == "build",
                )
            }
            "docker" => (
                "Docker operation".to_string(),
                vec!["docker".to_string()],
                Vec::new(),
                false,
            ),
            "make" => (
                "Run make target".to_string(),
                vec!["make".to_string()],
                Vec::new(),
                false,
            ),
            "git" => (
                "Git operation".to_string(),
                vec!["git".to_string()],
                Vec::new(),
                false,
            ),
            _ => (
                format!("Execute: {}", first_word),
                vec![first_word.to_string()],
                Vec::new(),
                false,
            ),
        };

        // Wrap command in shell
        let shell_cmd = match shell {
            "bash" => format!("bash -c '{}'", command.replace('\'', "'\\''")),
            "sh" => format!("sh -c '{}'", command.replace('\'', "'\\''")),
            "pwsh" | "powershell" => format!("pwsh -Command '{}'", command),
            "python" => format!("python -c '{}'", command),
            _ => command.to_string(),
        };

        Ok(InterpretedAction {
            original: command.to_string(),
            purpose,
            prerequisites,
            produces,
            cacheable,
            commands: vec![shell_cmd],
            required_env: Vec::new(),
            kg_terms: vec![format!("run:{}", first_word)],
            confidence: 0.8,
        })
    }
}

impl Default for CommandTranslator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_translate_checkout() {
        let translator = CommandTranslator::new();
        let result = translator.translate_action("actions/checkout@v4", None).unwrap();

        assert_eq!(result.purpose, "Checkout repository code");
        assert!(!result.commands.is_empty());
        assert!(result.prerequisites.contains(&"git".to_string()));
    }

    #[test]
    fn test_translate_setup_node() {
        let translator = CommandTranslator::new();
        let mut inputs = AHashMap::new();
        inputs.insert("node-version".to_string(), serde_yaml::Value::String("20".to_string()));

        let result = translator
            .translate_action("actions/setup-node@v4", Some(&inputs))
            .unwrap();

        assert!(result.purpose.contains("Node.js"));
        assert!(result.produces.iter().any(|p| p.contains("node")));
    }

    #[test]
    fn test_translate_npm_command() {
        let translator = CommandTranslator::new();
        let result = translator.translate_command("npm ci", "bash").unwrap();

        assert!(result.purpose.contains("dependencies"));
        assert!(result.cacheable);
    }

    #[test]
    fn test_translate_cargo_command() {
        let translator = CommandTranslator::new();
        let result = translator.translate_command("cargo build --release", "bash").unwrap();

        assert!(result.purpose.contains("Build"));
        assert!(result.prerequisites.contains(&"cargo".to_string()));
    }

    #[test]
    fn test_unknown_action() {
        let translator = CommandTranslator::new();
        let result = translator.translate_action("unknown/action@v1", None).unwrap();

        assert!(result.confidence < 0.5);
        assert!(result.commands[0].contains("Unknown"));
    }
}
