---
id: cb1b674aafa5499ca5bae7b6a4bb1cfa-1779732460792
command: rg -n 'fn from_toml|fn from_file|fn load' crates/terraphim_orchestrator/src/config.rs 2>/dev/null | head -20
exit_code: 1
source: Project
captured_at: 2026-05-25T18:07:40.792516305+00:00
working_dir: /home/alex/projects/terraphim/terraphim-ai
tags:
  - learning
  - exit-1
importance_total: 0.2900
importance_severity: 0.3000
importance_repetition: 0
importance_recency: 1.0000
importance_has_correction: false
---

## Command

`rg -n 'fn from_toml|fn from_file|fn load' crates/terraphim_orchestrator/src/config.rs 2>/dev/null | head -20`

## Error Output

```
1377:    pub fn from_toml(toml_str: &str) -> Result<Self, crate::error::OrchestratorError> {
1390:    pub fn from_file(
1469:    pub fn load_and_validate(
2900:    fn from_file_aggregates_pr_dispatch_from_includes() {
2986:    fn from_file_warns_when_pr_dispatch_in_include_has_no_projects() {

```

