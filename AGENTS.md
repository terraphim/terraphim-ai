# Terraphim AI - Agent Development Guide

## Documentation Organization

All project documentation is organized in the `.docs/` folder:
- **Individual File Summaries**: `.docs/summary-<normalized-path>.md` - Detailed summaries of each working file
- **Comprehensive Overview**: `.docs/summary.md` - Consolidated project overview and architecture analysis
- **Agent Instructions**: `.docs/agents_instructions.json` - Machine-readable agent configuration and workflows

## Mandatory /init Command Steps

When user executes `/init` command, you MUST perform these two steps in order:

### Step 1: Summarize Working Files
Can you summarize the working files? Save each file's summary in `.docs/summary-<normalized-path>.md`

- Identify all relevant working files in the project
- Create individual summaries for each file
- Save summaries using the pattern: `.docs/summary-<normalized-path>.md`
- Include file purpose, key functionality, and important details
- Normalize file paths (replace slashes with hyphens, remove special characters)

### Step 2: Create Comprehensive Summary
Can you summarize your context files ".docs/summary-*.md" and save the result in `.docs/summary.md`
- Read all individual summary files created in Step 1
- Synthesize into a comprehensive project overview
- Include architecture, security, testing, and business value analysis
- Save the consolidated summary as `.docs/summary.md`
- Update any relevant documentation references

Both steps are MANDATORY for every `/init` command execution.

## Build/Lint/Test Commands

### Rust Backend
```bash
# Build all workspace crates
cargo build --workspace

# Run single test
cargo test -p <crate_name> <test_name>

# Run tests with features
cargo test --features openrouter
cargo test --features mcp-rust-sdk

# Format and lint
cargo fmt
cargo clippy
```

### Frontend (Svelte)
```bash
cd desktop
yarn install
yarn run dev          # Development server
yarn run build        # Production build
yarn run check        # Type checking
yarn test             # Unit tests
yarn e2e              # End-to-end tests
```

## Code Style Guidelines

### Rust
- Use `tokio` for async runtime with `async fn` syntax
- Snake_case for variables/functions, PascalCase for types
- Use `Result<T, E>` with `?` operator for error handling
- Prefer `thiserror`/`anyhow` for custom error types
- Use `dyn` keyword for trait objects (e.g., `Arc<dyn StateManager>`)
- Remove unused imports regularly
- Feature gates: `#[cfg(feature = "openrouter")]`

### Frontend
- Svelte with TypeScript, Vite build tool
- Bulma CSS framework (no Tailwind)
- Use `yarn` package manager
- Component naming: PascalCase
- File naming: kebab-case

### General
- Never use `sleep` before `curl` (Cursor rule)
- Commit only relevant changes with clear technical descriptions
- All commits must pass pre-commit checks (format, lint, compilation)
- Use structured concurrency with scoped tasks
- Implement graceful degradation for network failures
- Never use `timeout` in command line; this command does not exist on macOS
- Never use mocks in tests
- Use IDE diagnostics to find and fix errors
- Always check test coverage after implementation
- Keep track of all tasks in Gitea issues using `gitea-robot` and `tea` CLI tools (see Gitea PageRank Workflow below)
- Commit every change and keep Gitea issues updated with progress using `tea comment IDX`
- Use `tmux` to spin off background tasks, read their output, and drive interaction
- Use `tmux` instead of `sleep` to continue working on a project and then read log output

## Documentation Management

### File Summaries
- Create individual summaries for each working file in `.docs/summary-<normalized-path>.md`
- Include file purpose, key functionality, and important details
- Normalize file paths (replace slashes with hyphens, remove special characters)

### Comprehensive Documentation
- Maintain consolidated overview in `.docs/summary.md`
- Include architecture, security, testing, and business value analysis
- Update documentation references when making changes

### Agent Instructions
- Use `.docs/agents_instructions.json` as primary reference for project patterns
- Contains machine-readable instructions for AI agents
- Includes project context, critical lessons, and established practices

````markdown
## UBS Quick Reference for AI Agents

UBS stands for "Ultimate Bug Scanner": **The AI Coding Agent's Secret Weapon: Flagging Likely Bugs for Fixing Early On**

**Install:** `curl -sSL https://raw.githubusercontent.com/Dicklesworthstone/ultimate_bug_scanner/master/install.sh | bash`

**Golden Rule:** `ubs <changed-files>` before every commit. Exit 0 = safe. Exit >0 = fix & re-run.

**Commands:**
```bash
ubs file.ts file2.py                    # Specific files (< 1s) — USE THIS
ubs $(git diff --name-only --cached)    # Staged files — before commit
ubs --only=js,python src/               # Language filter (3-5x faster)
ubs --ci --fail-on-warning .            # CI mode — before PR
ubs --help                              # Full command reference
ubs sessions --entries 1                # Tail the latest install session log
ubs .                                   # Whole project (ignores things like .venv and node_modules automatically)
```

**Output Format:**
```
⚠️  Category (N errors)
    file.ts:42:5 – Issue description
    💡 Suggested fix
Exit code: 1
```
Parse: `file:line:col` → location | 💡 → how to fix | Exit 0/1 → pass/fail

**Fix Workflow:**
1. Read finding → category + fix suggestion
2. Navigate `file:line:col` → view context
3. Verify real issue (not false positive)
4. Fix root cause (not symptom)
5. Re-run `ubs <file>` → exit 0
6. Commit

**Speed Critical:** Scope to changed files. `ubs src/file.ts` (< 1s) vs `ubs .` (30s). Never full scan for small edits.

**Bug Severity:**
- **Critical** (always fix): Null safety, XSS/injection, async/await, memory leaks
- **Important** (production): Type narrowing, division-by-zero, resource leaks
- **Contextual** (judgment): TODO/FIXME, console logs

**Anti-Patterns:**
- ❌ Ignore findings → ✅ Investigate each
- ❌ Full scan per edit → ✅ Scope to file
- ❌ Fix symptom (`if (x) { x.y }`) → ✅ Root cause (`x?.y`)
````
Use 'bd' for task tracking

<!-- GITEA_PAGERANK_WORKFLOW_START -->

## Gitea PageRank Workflow: Task Management and Agent Coordination

### Single Source of Truth

Gitea at `https://git.terraphim.cloud` is the authoritative system for all task management. ALL agents (human and AI) MUST use gitea-robot and tea for issue tracking.

### Environment

```bash
source ~/.profile  # Loads GITEA_URL and GITEA_TOKEN
```

Required env vars (set in `~/.profile`):
- `GITEA_URL=https://git.terraphim.cloud`
- `GITEA_TOKEN` -- API token for authentication

### Tools

- **gitea-robot** (`/home/alex/go/bin/gitea-robot`): PageRank-based issue prioritisation
- **tea** (`/home/alex/go/bin/tea`): Gitea CLI for issues, comments, PRs

### Task Discovery and Prioritisation

```bash
# Get issues ranked by dependency impact (highest PageRank first)
gitea-robot ready --owner terraphim --repo terraphim-ai

# Full triage with blocked/unblocked status
gitea-robot triage --owner terraphim --repo terraphim-ai

# View dependency graph
gitea-robot graph --owner terraphim --repo terraphim-ai

# Add dependency: issue X is blocked by issue Y
gitea-robot add-dep --owner terraphim --repo terraphim-ai --issue X --blocks Y
```

PageRank scores reflect how many downstream issues each task unblocks. Higher score = fix first.

### Issue Lifecycle

```bash
# List open issues
tea issues list --repo terraphim/terraphim-ai --state open

# Create issue
tea issues create --title "..." --repo terraphim/terraphim-ai

# Comment on issue (progress updates, implementation notes)
tea comment IDX "progress update" --repo terraphim/terraphim-ai

# Close issue
tea issues close IDX --repo terraphim/terraphim-ai
```

### Agent Workflow (Mandatory for All Agents)

1. **Start**: `source ~/.profile` (load Gitea env vars)
2. **Pick work**: `gitea-robot ready --owner terraphim --repo terraphim-ai` -- choose highest PageRank unblocked issue
3. **Branch**: `git checkout -b task/IDX-short-title`
4. **Implement**: TDD, commit with `Refs #IDX`
5. **Update Gitea**: `tea comment IDX "Implementation complete. Summary." --repo terraphim/terraphim-ai`
6. **PR**: Push branch, create PR on GitHub referencing `Refs terraphim/terraphim-ai#IDX (Gitea)`
7. **Close**: `tea issues close IDX --repo terraphim/terraphim-ai` after merge

### Commit Message Convention

```
feat(module): short description Refs #IDX
```

### Model Rules for Bigbox Agents

Use ONLY subscription-based models:
- `kimi-for-coding/k2p5` -- Moonshot subscription (implementation)
- `opencode-go/minimax-m2.5` -- MiniMax subscription
- `/home/alex/.local/bin/claude --model sonnet` -- Anthropic subscription (verification)
- `codex` -- ChatGPT OAuth

**NEVER** use `opencode/` prefix (Zen pay-per-use).
**NEVER** use `github-copilot/` prefix on bigbox.

<!-- GITEA_PAGERANK_WORKFLOW_END -->

## Landing the Plane (Session Completion)

**When ending a work session**, you MUST complete ALL steps below. Work is NOT complete until `git push` succeeds.

**MANDATORY WORKFLOW:**

1. **File issues for remaining work** - Create issues for anything that needs follow-up
2. **Run quality gates** (if code changed) - Tests, linters, builds
3. **Update issue status** - Close finished work, update in-progress items
4. **PUSH TO REMOTE** - This is MANDATORY:
   ```bash
   git pull --rebase
   bd sync
   git push
   git status  # MUST show "up to date with origin"
   ```
5. **Clean up** - Clear stashes, prune remote branches
6. **Verify** - All changes committed AND pushed
7. **Hand off** - Provide context for next session

**CRITICAL RULES:**
- Work is NOT complete until `git push` succeeds
- NEVER stop before pushing - that leaves work stranded locally
- NEVER say "ready to push when you are" - YOU must push
- If push fails, resolve and retry until it succeeds

## Agent Issue Creation Convention

Agents MUST follow this pattern when reporting findings. Do NOT post to standing log sinks.

### Rule: new issue per finding, with deduplication

Before creating any issue, check if one already exists for the same pattern:

```bash
THEME_ID="<agent-specific-slug>-$(date +%Y%m%d)"
EXISTING=$(gtr list-issues --owner OWNER --repo REPO --state open 2>/dev/null \
  | python3 -c 'import sys,json; issues=json.load(sys.stdin); [print(str(i["number"])) for i in issues if "Theme-ID: <prefix>" in i.get("body","")]' \
  | head -1)

if [ -n "$EXISTING" ]; then
  # Recurrence: comment on the existing issue
  gtr comment --owner OWNER --repo REPO --index "$EXISTING" \
    --body "Recurrence $(date -u +%Y-%m-%dT%H:%M:%SZ): $FINDING_DETAILS"
else
  # First occurrence: create new issue
  gtr create-issue --owner OWNER --repo REPO \
    --title "[Category] Finding title $(date +%Y-%m-%d)" \
    --body "$FINDING_DETAILS

Theme-ID: $THEME_ID

@adf:follow-up-agent please action this finding."
fi
# If nothing found: exit 0 silently - no issue needed
```

### Theme-ID convention

Every agent-created issue MUST include `Theme-ID: <slug>` in the body. The slug must be:
- Kebab-case
- Specific enough not to false-match related-but-different issues

| Agent | Theme-ID prefix | Follow-up agent |
|---|---|---|
| security-sentinel | `security-<finding-type>` | `@adf:merge-coordinator` |
| drift-detector | `config-drift` | `@adf:meta-coordinator` |
| documentation-generator | `doc-gap` | `@adf:reviewer` |
| spec-validator | `spec-gap` | `@adf:implementation-swarm` |
| test-guardian | `test-failure` | `@adf:implementation-swarm` |
| compliance-watchdog | `compliance-<finding>` | `@adf:merge-coordinator` |

### What NOT to do

```bash
# WRONG: posting to standing sink
gtr comment --owner terraphim --repo terraphim-ai --index 113 --body "report..."

# WRONG: creating duplicate issue without dedup check
gtr create-issue --owner terraphim --repo terraphim-ai --title "Security issue" ...

# WRONG: exiting silently when there is a finding (no issue created)
echo "Found drift" && exit 0
```

### When to create issues vs stay silent

- **Finding detected, no open issue for it**: create new issue with Theme-ID
- **Finding detected, open issue exists**: add recurrence comment
- **No findings**: exit 0 silently - do NOT create a "all clear" issue
