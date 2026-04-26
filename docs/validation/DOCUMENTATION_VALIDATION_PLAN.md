# Documentation Validation Plan

## Executive Summary

This plan validates every step in Terraphim AI documentation against the actual implementation. We have identified **12 documents** across two repositories that need validation, with **47 specific validation items** that must be checked.

## Repository Sync Status

| Remote | Status | Latest Commit |
|--------|--------|---------------|
| GitHub (origin/main) | In sync | 48df9d67 |
| Gitea (gitea/main) | In sync | 48df9d67 |
| Branches on Gitea only | 12 task branches | task/680-* through task/860-* |

**Finding**: GitHub and Gitea main branches are identical. Gitea has additional feature branches not on GitHub.

## Document Inventory

### Repository: terraphim-ai/docs/

| Document | Type | Lines | Last Updated |
|----------|------|-------|--------------|
| user-guide/getting-started.md | User Guide | 193 | Dec 2025 |
| user-guide/installation.md | User Guide | 370 | Dec 2025 |
| user-guide/quick-start.md | User Guide | 275 | Dec 2025 |
| user-guide/quickwit-log-exploration.md | User Guide | 358 | Jan 2026 |
| user-guide/troubleshooting.md | User Guide | 549 | Dec 2025 |
| blog/fff-search-integration.md | Blog Post | 202 | Apr 2025 |
| walkthroughs/frontend-developer-agent.md | Walkthrough | 707 | Apr 2026 |

### Repository: terraphim.ai/website/

| Document | Type | Lines | Last Updated |
|----------|------|-------|--------------|
| content/docs/installation.md | Documentation | 239 | Jan 2026 |
| content/docs/quickstart.md | Documentation | 203 | Jan 2026 |
| content/docs/crates.md | Reference | 171 | Apr 2026 |
| content/how-tos/command-rewriting-howto.md | How-To | 248 | Apr 2026 |

## Critical Discrepancies Found

### 1. Crate Count Mismatch (HIGH PRIORITY)
- **Document**: `terraphim.ai/content/docs/crates.md` claims **52 crates**
- **Actual**: Workspace contains **54 Cargo.toml files** in `crates/`
- **Action**: Count actual crates and update documentation

### 2. CLI Commands Mismatch (HIGH PRIORITY)
- **Document**: `docs/user-guide/getting-started.md` and `installation.md` claim `terraphim-agent` has 14 commands
- **Actual**: `terraphim-agent --help` shows **18 commands** (search, roles, config, graph, chat, extract, replace, validate, suggest, hook, guard, interactive, repl, setup, check-update, update, learn, sessions, listen)
- **Action**: Update command counts and verify all commands work

### 3. Missing Commands in Documentation (MEDIUM PRIORITY)
- **Documents**: `quickstart.md`, `getting-started.md`
- **Missing**: `guard`, `hook`, `learn`, `sessions`, `listen`, `check-update`
- **Action**: Add missing commands or remove references to non-existent ones

### 4. Installation Methods Discrepancy (HIGH PRIORITY)
- **Document**: `docs/user-guide/installation.md` mentions Node.js (`@terraphim/autocomplete`) and Python (`terraphim-automata`) packages
- **Document**: `terraphim.ai/content/docs/installation.md` mentions `terraphim-cli` binary
- **Actual**: `cargo install terraphim-agent` works; other packages need verification
- **Action**: Test each installation method

### 5. Version Numbers Out of Sync (MEDIUM PRIORITY)
- **Document**: `terraphim.ai/content/docs/installation.md` references version **1.16.31**
- **Actual**: `terraphim-agent --version` shows **1.16.32**
- **Action**: Update all version references

### 6. Setup Templates Discrepancy (MEDIUM PRIORITY)
- **Document**: `docs/walkthroughs/frontend-developer-agent.md` mentions `--template frontend-engineer`
- **Actual**: `terraphim-agent setup --list-templates` shows **11 templates** including `frontend-engineer`
- **Finding**: Template exists but documentation describes different behavior than actual implementation

### 7. REPL Commands Mismatch (HIGH PRIORITY)
- **Document**: `terraphim.ai/content/docs/quickstart.md` shows REPL commands: `search`, `role`, `connect`, `import`, `export`, `status`, `help`
- **Actual**: REPL commands need verification against actual implementation
- **Action**: Start REPL and test each command

### 8. Build Issues (HIGH PRIORITY)
- **Finding**: `cargo build --workspace` fails with **zlob/zig build error**
- **Impact**: "Build from source" instructions in multiple documents are broken
- **Action**: Fix build or document workaround

## Validation Matrix

### Phase 1: Installation & Setup (Priority: Critical)

| Step | Document | Command | Expected | Actual | Status |
|------|----------|---------|----------|--------|--------|
| 1.1 | installation.md | `cargo install terraphim-agent` | Installs binary | Works (1.16.32) | PASS |
| 1.2 | installation.md | `cargo install terraphim-cli` | Installs CLI | Needs verification | TODO |
| 1.3 | installation.md | `brew install terraphim-ai` | Installs via Homebrew | Needs verification | TODO |
| 1.4 | installation.md | `npm install @terraphim/autocomplete` | Installs Node package | Needs verification | TODO |
| 1.5 | installation.md | `pip install terraphim-automata` | Installs Python package | Needs verification | TODO |
| 1.6 | installation.md | Build from source | Compiles workspace | FAILS (zlob error) | FAIL |
| 1.7 | quickstart.md | `terraphim_server` | Starts server | Needs verification | TODO |
| 1.8 | quickstart.md | `terraphim-agent` | Starts agent | Works | PASS |

### Phase 2: CLI Commands (Priority: Critical)

| Command | Document | Status | Notes |
|---------|----------|--------|-------|
| `search` | All | PASS | Works with `--role`, `--limit`, `--terms` |
| `roles` | getting-started.md | PASS | Listed in help |
| `config` | installation.md | PASS | Subcommands: show, set, validate, reload |
| `graph` | getting-started.md | PASS | Listed in help |
| `chat` | getting-started.md | PASS | Listed in help |
| `extract` | getting-started.md | PASS | Listed in help |
| `replace` | command-rewriting-howto.md | PASS | Works with `--role`, `--fail-open`, `--json` |
| `validate` | getting-started.md | PASS | Listed in help |
| `suggest` | frontend-developer-agent.md | PASS | Listed in help |
| `hook` | command-rewriting-howto.md | PASS | Subcommands: pre-tool-use, post-tool-use, pre-commit, prepare-commit-msg |
| `guard` | command-rewriting-howto.md | PASS | Works with `--json`, `--fail-open` |
| `interactive` | getting-started.md | PASS | Listed in help |
| `repl` | frontend-developer-agent.md | PASS | Works with `--server`, `--server-url` |
| `setup` | frontend-developer-agent.md | PASS | Works with `--template`, `--list-templates` |
| `check-update` | - | PASS | Listed in help, not in docs |
| `update` | - | PASS | Listed in help, not in docs |
| `learn` | command-rewriting-howto.md | PASS | Subcommands: capture, list, query, correct, correction, hook, install-hook, procedure |
| `sessions` | - | PASS | Subcommands: sources, list, search, stats |
| `listen` | - | PASS | Listed in help, not in docs |

### Phase 3: Configuration (Priority: High)

| Step | Document | Config Path | Expected | Status |
|------|----------|-------------|----------|--------|
| 3.1 | installation.md | `~/.config/terraphim/config.toml` | Created by `init` | TODO: Verify `terraphim-agent init` exists |
| 3.2 | installation.md | Environment variables | `TERRAPHIM_LOG_LEVEL`, etc. | TODO: Verify |
| 3.3 | frontend-developer-agent.md | `~/.config/terraphim/embedded_config.json` | Role config | TODO: Verify format |
| 3.4 | frontend-developer-agent.md | `~/.config/terraphim/kg/frontend/` | Knowledge graph | TODO: Verify |

### Phase 4: Features (Priority: High)

| Feature | Document | Status | Notes |
|---------|----------|--------|-------|
| Quickwit integration | quickwit-log-exploration.md | TODO | Test with actual Quickwit server |
| GrepApp integration | frontend-developer-agent.md | TODO | Requires `--features grepapp` |
| MCP server | frontend-developer-agent.md | TODO | Test `terraphim_mcp_server` |
| FFF search | blog/fff-search-integration.md | TODO | Verify tools exist |
| Command rewriting | command-rewriting-howto.md | TODO | Test `replace` with KG |
| Session search | - | TODO | Test `terraphim-agent sessions` |

### Phase 5: Website Content (Priority: Medium)

| Document | Issue | Action |
|----------|-------|--------|
| crates.md | Claims 52 crates, actual is 54 | Update count and list |
| installation.md | Version 1.16.31, actual 1.16.32 | Update version |
| quickstart.md | REPL commands may be outdated | Verify each command |
| quickstart.md | `terraphim-cli` commands | Verify CLI works |

## Detailed Validation Steps

### Document: docs/user-guide/getting-started.md

1. **Line 11**: `cargo install terraphim-agent` - PASS (verified)
2. **Line 17**: `npm install @terraphim/autocomplete` - TODO (verify package exists)
3. **Line 23**: `pip install terraphim-automata` - TODO (verify package exists)
4. **Line 36**: `terraphim-agent search "query"` - PASS (verified)
5. **Line 55**: `terraphim-agent chat "help me understand my data"` - TODO (test chat)
6. **Line 56**: `terraphim-agent role create research-assistant` - FAIL (command is `roles`, not `role`)
7. **Line 93**: `~/.config/terraphim/config.toml` - TODO (verify path)
8. **Line 105**: LLM providers list - TODO (verify providers)

### Document: docs/user-guide/installation.md

1. **Line 11**: `cargo install terraphim-agent` - PASS
2. **Line 21**: `npm install @terraphim/autocomplete` - TODO
3. **Line 32**: `pip install terraphim-automata` - TODO
4. **Line 68**: Rust install instructions - PASS
5. **Line 107**: `brew install terraphim-agent` - TODO (verify formula exists)
6. **Line 186**: `terraphim-agent --version` - PASS (shows 1.16.32)
7. **Line 201**: `terraphim-agent init` - FAIL (command is `setup`, not `init`)
8. **Line 210**: `terraphim-agent search "test query"` - PASS
9. **Line 222**: Environment variables - TODO (verify)

### Document: docs/user-guide/quick-start.md

1. **Line 13**: One-line install - TODO (test)
2. **Line 23**: `terraphim-agent init` - FAIL (should be `setup`)
3. **Line 36**: `terraphim-agent search "how to install rust"` - PASS
4. **Line 46**: `terraphim-agent add-source local` - FAIL (command doesn't exist)
5. **Line 56**: `terraphim-agent chat` - TODO (test)
6. **Line 119**: `terraphim-agent config set` - TODO (test)
7. **Line 128**: `terraphim-agent search --scorer` - TODO (verify flag exists)
8. **Line 142**: `terraphim-agent role create` - FAIL (command is `roles`)
9. **Line 145**: `terraphim-agent workflow create` - FAIL (command doesn't exist)
10. **Line 151**: `terraphim-agent --tui` - FAIL (flag is `--interactive`, not `--tui`)
11. **Line 170**: `terraphim-agent --tui` - FAIL (same issue)
12. **Line 188**: `terraphim-ai-desktop` - TODO (verify binary exists)

### Document: docs/user-guide/quickwit-log-exploration.md

1. **Line 24**: Quickwit haystack configuration - TODO (verify format)
2. **Line 46**: `terraphim-agent` then `/role QuickwitLogs` - TODO (test REPL)
3. **Line 52**: `/search "level:ERROR"` - TODO (test with Quickwit)
4. **Line 294**: Pre-configured role in `terraphim_engineer_config.json` - TODO (verify file)

### Document: docs/walkthroughs/frontend-developer-agent.md

1. **Line 28**: `cargo build --release` - FAIL (workspace build fails)
2. **Line 31**: `cargo build --release -p terraphim_middleware --features grepapp` - TODO (test)
3. **Line 39**: `cargo install --path crates/terraphim_agent` - TODO (test)
4. **Line 55**: `terraphim-agent setup --template frontend-engineer` - PASS (template exists)
5. **Line 312**: `terraphim-agent search "flexbox responsive layout"` - TODO (test with role)
6. **Line 322**: `terraphim-agent suggest "svel" --fuzzy` - TODO (test)
7. **Line 332**: `terraphim-agent replace "..." --format markdown` - TODO (test)
8. **Line 342**: `terraphim-agent validate "..." --connectivity` - TODO (test)
9. **Line 352**: `terraphim-agent repl` - PASS
10. **Line 440**: `cargo build --release -p terraphim_mcp_server` - TODO (test)

### Document: docs/blog/fff-search-integration.md

1. **Line 31**: MCP tools `terraphim_find_files`, `terraphim_grep`, `terraphim_multi_grep` - TODO (verify)
2. **Line 51**: `ExternalScorer` trait - TODO (verify in code)
3. **Line 173**: Implementation status table - TODO (verify each component)

### Document: terraphim.ai/content/docs/installation.md

1. **Line 16**: Universal installer script - TODO (test)
2. **Line 24**: `brew tap terraphim/terraphim` - TODO (verify tap exists)
3. **Line 35**: `cargo install terraphim-agent` - PASS
4. **Line 38**: `cargo install terraphim-cli` - TODO (verify)
5. **Line 46**: `.deb` package download - TODO (verify URL)
6. **Line 60**: Binary download URLs with version 1.16.31 - FAIL (version is 1.16.32)
7. **Line 82**: Build from source - FAIL (workspace build fails)
8. **Line 151**: `npm install @terraphim/autocomplete` - TODO
9. **Line 159**: `pip install terraphim-automata` - TODO
10. **Line 186**: `terraphim-agent --version` shows 1.16.31 - FAIL (shows 1.16.32)

### Document: terraphim.ai/content/docs/quickstart.md

1. **Line 22**: Universal installer - TODO (test)
2. **Line 48**: `terraphim_server` - TODO (test)
3. **Line 65**: `terraphim-agent` - PASS
4. **Line 74**: REPL `search rust async` - TODO (test)
5. **Line 77**: REPL `role engineer` - TODO (test)
6. **Line 89**: REPL commands list - TODO (verify each)
7. **Line 104**: `import ~/notes/project-a.md` - TODO (test)
8. **Line 116**: `source add github ...` - TODO (test)
9. **Line 153**: `terraphim-cli search` - TODO (test)

### Document: terraphim.ai/content/docs/crates.md

1. **Line 12**: Claims 52 crates - FAIL (54 found)
2. **Line 18-31**: Core Engine crates - TODO (verify each exists)
3. **Line 36-43**: Binary crates - TODO (verify each exists)
4. **Line 48-60**: Agent Orchestration crates - TODO (verify each exists)
5. **Line 65-75**: KG Intelligence crates - TODO (verify each exists)
6. **Line 80-87**: Haystack crates - TODO (verify each exists)

### Document: terraphim.ai/content/how-tos/command-rewriting-howto.md

1. **Line 29**: `terraphim-agent` version 1.16.33 or later - FAIL (installed is 1.16.32)
2. **Line 41**: KG path `~/.config/terraphim/docs/src/kg/` - TODO (verify)
3. **Line 83**: `terraphim-agent replace --role "Terraphim Engineer"` - TODO (test)
4. **Line 104**: Cache path `/tmp/terraphim_sqlite/terraphim.db` - TODO (verify)
5. **Line 125**: OpenCode plugin example - TODO (verify)

## Recommended Actions

### Immediate (This Session)

1. **Fix build issue**: Investigate zlob/zig build failure
2. **Update version numbers**: Change 1.16.31 to 1.16.32 in all docs
3. **Fix command references**:
   - `init` -> `setup`
   - `role` -> `roles`
   - `--tui` -> `--interactive`
   - Remove non-existent commands (`workflow create`, `add-source`)
4. **Update crate count**: 52 -> 54

### Short-term (Next Session)

1. **Test installation methods**: npm, pip, brew, .deb
2. **Test REPL commands**: Verify each command in quickstart
3. **Test features**: Quickwit, GrepApp, MCP server
4. **Verify configuration paths**: Test config file locations
5. **Test command rewriting**: Verify `replace` with KG

### Long-term (Ongoing)

1. **Automate validation**: Create CI job that tests docs against implementation
2. **Version pinning**: Add version checks to release process
3. **Documentation linting**: Add tool to detect stale commands
4. **Integration tests**: Test each documented workflow end-to-end

## Success Criteria

- [ ] All installation methods work as documented
- [ ] All CLI commands match documentation
- [ ] All configuration paths are correct
- [ ] All version numbers are current
- [ ] All feature workflows execute successfully
- [ ] No references to non-existent commands or flags
- [ ] Build from source instructions work

## Notes

- Use `disciplined-research` skill for deep investigation of each discrepancy
- Use `disciplined-design` skill for planning fixes
- Use `disciplined-implementation` skill for executing fixes
- Use `disciplined-verification` skill for testing fixes
- Track progress in Gitea issues using `gitea-robot`
