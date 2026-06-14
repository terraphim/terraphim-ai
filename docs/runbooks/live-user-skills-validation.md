# Live-User Skills Validation Runbook

## Purpose
Enable a real live user to validate every Terraphim Skill end-to-end across the supported CLI estate. This runbook covers environment preparation, token acquisition, validation execution, evidence capture, and sign-off.

## Prerequisites

Run the prerequisite check first:

```bash
bash scripts/skill-installer-prereqs.sh
```

The script checks:
1. Required CLI binaries: `tsm`, `claude`, `codex`, `pi`, `pi-rust`, `grok`
2. Optional binaries: `adf`, `adf-ctl` (only needed for agent-driven runs)
3. Required skill directories: `~/.claude/skills/`, `~/.codex/skills/`, `~/.pi/agent/skills/`, `~/.grok/skills/`, `~/.agents/skills/`
4. Token presence without printing its value: `TSM_TOKEN` env var or `~/.terraphim/token`
5. Marketplace reachability at `https://api.terraphim-skills.md`

Exit codes:
- `0` = all required prerequisites met
- `1` = some required prerequisites missing
- `2` = token missing specifically

## Step 1: Install `tsm`

If `tsm` is missing:

```bash
cargo install --path /path/to/terraphim-skills.md
# or from registry when published:
cargo install tsm
```

Verify:

```bash
tsm --version
```

## Step 2: Acquire Token

### Option A: Interactive login (recommended for first-time setup)

```bash
tsm login
```

This prints instructions to copy a token from `https://terraphim-skills.md/account`. The token is stored at `~/.terraphim/token` (chmod 600).

### Option B: Environment variable (for unattended runs)

Set `TSM_TOKEN` from 1Password or a secure vault. The validator reads the variable length but never the value:

```bash
# Set without echoing
read -s TSM_TOKEN
export TSM_TOKEN
```

The validation script and prereqs check only verify the length of the variable. The token value is never printed.

## Step 3: Install Required CLIs

Document where to obtain each CLI. All five are open-source:

| CLI | Install | Skills Dir |
|-----|---------|-----------|
| Claude Code | https://claude.com/product/claude-code | `~/.claude/skills/` |
| Codex | https://github.com/openai/codex | `~/.codex/skills/` |
| Pi (badlogic) | https://github.com/badlogic/pi-mono | `~/.pi/agent/skills/` |
| pi-rust | https://github.com/badlogic/pi-mono (Rust port) | `~/.pi/agent/skills/` and/or `~/.agents/skills/` |
| Grok | https://github.com/superagent-ai/grok | `~/.grok/skills/` |

The script `skill-installer-prereqs.sh` reports versions and directory presence.

## Step 4: Run Validation

### Sentinel smoke test (fast)

```bash
TERRAPHIM_SKILLS_SENTINEL_SKILLS="code-review,testing,security-audit" \
TERRAPHIM_SKILLS_MANAGER_BIN="$(which tsm)" \
  scripts/skill-installer-validation.sh
```

Output:
- JSON to stdout
- Markdown matrix to stderr

### Full catalogue run

```bash
scripts/skill-installer-validation.sh
```

This uses the canonical full skills catalogue (see `#2709`). Same JSON + Markdown output.

### Dry-run (no token, ci-native)

```bash
TERRAPHIM_SKILLS_DRY_RUN=1 scripts/skill-installer-validation.sh
```

## Step 5: Capture Evidence

Every run produces:
- JSON report (stdout) - machine-readable
- Markdown matrix (stderr) - human-readable
- Exit code (0 = all PASS, 1 = one or more FAIL, 2 = no token)

To capture and post to a Gitea issue:

```bash
scripts/skill-installer-validation.sh > report.json 2> matrix.md
gtr comment --owner terraphim --repo terraphim-ai --index 2707 --body "$(cat matrix.md)"
```

## Step 6: Sign Off

The live user confirms:
- [ ] All required CLIs installed
- [ ] Token present and working
- [ ] Sentinel smoke test PASS
- [ ] Full catalogue run completed
- [ ] Every FAIL has a follow-up issue or approved waiver
- [ ] Evidence posted to the epic `#2707`

## Rollback / Cleanup

Uninstalling all skills across CLIs:

```bash
tsm uninstall <skill>  # for each installed skill
# or per-CLI:
# Claude: rm -rf ~/.claude/skills/<skill>
# Codex: rm -rf ~/.codex/skills/<skill>
# Pi: rm -rf ~/.pi/agent/skills/<skill>
# Grok: rm -rf ~/.grok/skills/<skill>
```

Removing token:

```bash
rm ~/.terraphim/token
unset TSM_TOKEN
```

## Troubleshooting

| Symptom | Cause | Action |
|---------|-------|--------|
| `tsm: command not found` | `tsm` not installed | `cargo install tsm` |
| `TSM_TOKEN not set` | Token missing | `tsm login` or set `TSM_TOKEN` |
| `No skill.toml found` | Skill not installed via marketplace | Use `tsm install <skill>` first |
| `pi inspect` hangs | Interactive prompt | Use `pi list` (non-interactive) |
| `codex inspect` stdin error | Not a TTY | Use `ls ~/.codex/skills/<skill>` |
| `grok inspect --cwd` fails | Subcommand doesn't take `--cwd` | Use `grok --cwd <dir> inspect --json` |

## Security Notes

- Tokens are stored in `~/.terraphim/token` with chmod 600.
- The prereqs script never prints token values, only lengths.
- Validation script redacts command output in error messages.
- No secrets are committed to repos.
- Remote URLs in `.git/config` never include tokens.
