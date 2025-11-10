use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use regex::Regex;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use walkdir::WalkDir;

#[derive(Error, Debug)]
pub enum LintError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("YAML parse error in {path:?}: {source}")]
    Yaml {
        #[source]
        source: serde_yaml::Error,
        path: PathBuf,
    },
    #[error("Invalid schema in {0}: {1}")]
    InvalidSchema(PathBuf, String),
    #[error("Automata error: {0}")]
    Automata(#[from] terraphim_automata::TerraphimAutomataError),
    #[error("Automata builder error: {0}")]
    AutomataBuilder(String),
}

pub type Result<T> = std::result::Result<T, LintError>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandArg {
    pub name: String,
    #[serde(rename = "type")]
    pub type_name: String,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub default: Option<serde_yaml::Value>,
    #[serde(default)]
    pub enum_values: Option<Vec<String>>,
    #[serde(default)]
    pub pattern: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandPermissionRef {
    pub can: String,
    #[serde(default)]
    pub on: Option<String>, // resource or scope
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandDef {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub args: Vec<CommandArg>,
    #[serde(default)]
    pub permissions: Vec<CommandPermissionRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypesBlock(
    #[serde(with = "serde_yaml::with::singleton_map_recursive")]
    pub  BTreeMap<String, BTreeMap<String, String>>,
);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PermissionRule {
    pub action: String,
    #[serde(default)]
    pub command: Option<String>,
    #[serde(default)]
    pub resource: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RolePermissions {
    pub name: String,
    #[serde(default)]
    pub allow: Vec<PermissionRule>,
    #[serde(default)]
    pub deny: Vec<PermissionRule>,
}

#[derive(Debug, Clone, Default)]
pub struct SchemaFragments {
    pub commands: Vec<CommandDef>,
    pub types: BTreeMap<String, BTreeMap<String, String>>, // TypeName -> field -> type expr
    pub roles: Vec<RolePermissions>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LintIssue {
    pub path: PathBuf,
    pub severity: Severity,
    pub code: &'static str,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone, Serialize)]
pub struct LintReport {
    pub scanned_files: usize,
    pub issues: Vec<LintIssue>,
    pub stats: ReportStats,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct ReportStats {
    pub command_count: usize,
    pub type_count: usize,
    pub role_count: usize,
    pub thesaurus_terms: usize,
}

/// Parser for extracting code fences from markdown
fn extract_fenced_blocks(contents: &str) -> Vec<(String, String)> {
    // Captures triple-backtick fences with an identifier like ```kg-commands
    // Group 1: fence id, Group 2: body
    let re = Regex::new(r"(?ms)```\s*([A-Za-z0-9_\-:]+)\s*\n(.*?)\n```\s*").unwrap();
    re.captures_iter(contents)
        .filter_map(|cap| {
            let id = cap.get(1)?.as_str().trim().to_string();
            let body = cap.get(2)?.as_str().to_string();
            Some((id, body))
        })
        .collect()
}

fn parse_yaml<T: for<'de> Deserialize<'de>>(body: &str, path: &Path) -> Result<T> {
    serde_yaml::from_str::<T>(body).map_err(|e| LintError::Yaml {
        source: e,
        path: path.to_path_buf(),
    })
}

pub fn load_schema_fragments_from_markdown(path: &Path) -> Result<SchemaFragments> {
    let contents = std::fs::read_to_string(path)?;
    let mut fragments = SchemaFragments::default();
    for (id, body) in extract_fenced_blocks(&contents) {
        match id.as_str() {
            "kg-commands" | "kg:commands" => {
                // Support a single command or a YAML array of commands
                if body.trim_start().starts_with('-') {
                    let defs: Vec<CommandDef> = parse_yaml(&body, path)?;
                    fragments.commands.extend(defs);
                } else {
                    let def: CommandDef = parse_yaml(&body, path)?;
                    fragments.commands.push(def);
                }
            }
            "kg-types" | "kg:types" => {
                let types_block: TypesBlock = parse_yaml(&body, path)?;
                for (k, v) in types_block.0.into_iter() {
                    fragments.types.insert(k, v);
                }
            }
            "kg-permissions" | "kg:permissions" => {
                if body.trim_start().starts_with("roles:") {
                    #[derive(Deserialize)]
                    struct RolesWrapper {
                        roles: Vec<RolePermissions>,
                    }
                    let w: RolesWrapper = parse_yaml(&body, path)?;
                    fragments.roles.extend(w.roles);
                } else if body.trim_start().starts_with('-') {
                    let roles: Vec<RolePermissions> = parse_yaml(&body, path)?;
                    fragments.roles.extend(roles);
                } else {
                    let role: RolePermissions = parse_yaml(&body, path)?;
                    fragments.roles.push(role);
                }
            }
            _ => {}
        }
    }
    Ok(fragments)
}

fn validate_types(types: &BTreeMap<String, BTreeMap<String, String>>) -> Vec<String> {
    let mut issues = Vec::new();
    let valid_primitives: BTreeSet<&'static str> = [
        "string", "integer", "number", "boolean", "object", "array", "ulid", "url", "path",
    ]
    .into_iter()
    .collect();
    let type_re =
        Regex::new(r"^(?P<base>[A-Za-z_][A-Za-z0-9_-]*)(?P<array>\[\])?(?P<opt>\?)?$").unwrap();
    let type_name_re = Regex::new(r"^[A-Z][A-Za-z0-9_]*$").unwrap();
    let field_name_re = Regex::new(r"^[a-zA-Z_][A-Za-z0-9_]*\??$").unwrap();
    for (tname, fields) in types.iter() {
        if !type_name_re.is_match(tname) {
            issues.push(format!(
                "Type '{tname}' should be PascalCase alphanumeric with underscores"
            ));
        }
        for (fname, ftype) in fields.iter() {
            if !field_name_re.is_match(fname) {
                issues.push(format!(
                    "Field '{tname}.{fname}' should be snake_case or camelCase and may end with '?'"
                ));
            }
            if let Some(c) = type_re.captures(ftype) {
                let base = c.name("base").unwrap().as_str();
                if !valid_primitives.contains(base) && !types.contains_key(base) {
                    issues.push(format!(
                        "Field '{tname}.{fname}' references unknown type '{base}'"
                    ));
                }
            } else {
                issues.push(format!(
                    "Field '{tname}.{fname}' has invalid type expression '{ftype}'"
                ));
            }
        }
    }
    issues
}

fn validate_commands(
    commands: &[CommandDef],
    types: &BTreeMap<String, BTreeMap<String, String>>,
) -> Vec<String> {
    let mut issues = Vec::new();
    let type_re =
        Regex::new(r"^(?P<base>[A-Za-z_][A-Za-z0-9_-]*)(?P<array>\[\])?(?P<opt>\?)?$").unwrap();
    let command_name_re = Regex::new(r"^[a-z][a-z0-9_-]*$").unwrap();
    for cmd in commands {
        if !command_name_re.is_match(&cmd.name) {
            issues.push(format!(
                "Command '{}' should be kebab-case (lowercase letters, digits, hyphens)",
                cmd.name
            ));
        }
        let mut seen = BTreeSet::new();
        for arg in &cmd.args {
            if !seen.insert(arg.name.clone()) {
                issues.push(format!(
                    "Command '{}' has duplicate arg name '{}'",
                    cmd.name, arg.name
                ));
            }
            if let Some(c) = type_re.captures(&arg.type_name) {
                let base = c.name("base").unwrap().as_str();
                let valid_primitives: BTreeSet<&'static str> = [
                    "string", "integer", "number", "boolean", "object", "array", "ulid", "url",
                    "path",
                ]
                .into_iter()
                .collect();
                if !valid_primitives.contains(base) && !types.contains_key(base) {
                    issues.push(format!(
                        "Command '{}' arg '{}' references unknown type '{}'",
                        cmd.name, arg.name, base
                    ));
                }
            } else {
                issues.push(format!(
                    "Command '{}' arg '{}' has invalid type '{}'",
                    cmd.name, arg.name, arg.type_name
                ));
            }
        }
    }
    issues
}

fn validate_permissions(commands: &[CommandDef], roles: &[RolePermissions]) -> Vec<String> {
    let mut issues = Vec::new();
    let command_names: BTreeSet<String> = commands.iter().map(|c| c.name.clone()).collect();
    for role in roles {
        for rule in role.allow.iter().chain(role.deny.iter()) {
            if rule.action == "execute" {
                if let Some(cmd) = &rule.command {
                    if !command_names.contains(cmd) {
                        issues.push(format!(
                            "Role '{}' references unknown command '{}' in {} list",
                            role.name,
                            cmd,
                            if role.allow.contains(rule) {
                                "allow"
                            } else {
                                "deny"
                            }
                        ));
                    }
                } else {
                    issues.push(format!(
                        "Role '{}' has 'execute' rule without 'command' field",
                        role.name
                    ));
                }
            }
        }
    }
    issues
}

/// Build thesaurus from a directory of markdown files using terraphim_automata Logseq builder.
async fn build_thesaurus_from_dir(name: &str, dir: &Path) -> Result<terraphim_types::Thesaurus> {
    use terraphim_automata::ThesaurusBuilder;
    let builder = terraphim_automata::Logseq::default();
    let thesaurus = builder
        .build(name.to_string(), dir.to_path_buf())
        .await
        .map_err(|e| LintError::AutomataBuilder(e.to_string()))?;
    Ok(thesaurus)
}

pub async fn lint_path(path: &Path) -> Result<LintReport> {
    let mut issues = Vec::<LintIssue>::new();
    let mut fragments = SchemaFragments::default();
    let mut scanned_files = 0usize;

    // Scan markdown files recursively
    for entry in WalkDir::new(path).into_iter().filter_map(|e| e.ok()) {
        let p = entry.path();
        if p.is_file() {
            if let Some(ext) = p.extension() {
                if ext == "md" {
                    scanned_files += 1;
                    match load_schema_fragments_from_markdown(p) {
                        Ok(fr) => {
                            fragments.commands.extend(fr.commands);
                            for (k, v) in fr.types {
                                if fragments.types.contains_key(&k) {
                                    issues.push(LintIssue {
                                        path: p.to_path_buf(),
                                        severity: Severity::Error,
                                        code: "types.duplicate",
                                        message: format!("Type '{}' defined multiple times", k),
                                    });
                                }
                                fragments.types.insert(k, v);
                            }
                            fragments.roles.extend(fr.roles);
                        }
                        Err(e) => {
                            issues.push(LintIssue {
                                path: p.to_path_buf(),
                                severity: Severity::Error,
                                code: "parse.failure",
                                message: format!("Failed to parse schema blocks: {e}"),
                            });
                        }
                    }
                }
            }
        }
    }

    // Validate schema relationships
    for msg in validate_types(&fragments.types) {
        issues.push(LintIssue {
            path: path.to_path_buf(),
            severity: Severity::Error,
            code: "types.invalid",
            message: msg,
        });
    }
    for msg in validate_commands(&fragments.commands, &fragments.types) {
        issues.push(LintIssue {
            path: path.to_path_buf(),
            severity: Severity::Error,
            code: "commands.invalid",
            message: msg,
        });
    }
    for msg in validate_permissions(&fragments.commands, &fragments.roles) {
        issues.push(LintIssue {
            path: path.to_path_buf(),
            severity: Severity::Error,
            code: "permissions.invalid",
            message: msg,
        });
    }

    // Build thesaurus using existing automata extractor to ensure KG is healthy
    // If the directory is a KG root (contains markdown files), this will populate terms.
    let thesaurus = build_thesaurus_from_dir("KG", path).await?;
    let thesaurus_terms = thesaurus.len();

    // Build autocomplete index as a proxy for KG search/embeddings readiness
    let _index = terraphim_automata::build_autocomplete_index(thesaurus, None)
        .map_err(LintError::Automata)?;

    let report = LintReport {
        scanned_files,
        stats: ReportStats {
            command_count: fragments.commands.len(),
            type_count: fragments.types.len(),
            role_count: fragments.roles.len(),
            thesaurus_terms,
        },
        issues,
    };
    Ok(report)
}
