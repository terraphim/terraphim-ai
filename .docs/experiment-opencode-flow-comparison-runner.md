# OpenCode + Terraphim Flow Comparison Experiment

**Date**: 2026-05-02
**Task**: Build accessible navigation component in Svelte
**Project**: `~/projects/frontend-test`

## Setup Verification

```bash
# Verify roles
terraphim-agent roles list
# Expected: AI Engineer (ai), Frontend Developer (fedev)

# Verify project
ls ~/projects/frontend-test/src/
# Expected: app.d.ts, app.html, lib, routes

# Verify OpenCode
opencode --version
# Expected: 1.14.31
```

## Three Flows

| Flow | Description | Terraphim | Haystack/FFF |
|------|------------|-----------|---------------|
| **A** | OpenCode + terraphim search | Yes | Ripgrep |
| **B** | OpenCode + FFF MCP tools | Yes | FFF (direct) |
| **C** | Control (no terraphim) | No | Default search |

## Task Prompt

**Use this EXACT prompt for all three flows:**

```
Add an accessible navigation component to this Svelte project. Requirements:
1. Responsive design with mobile hamburger menu
2. ARIA labels and roles
3. Keyboard navigation (Tab to move between items, Escape to close mobile menu)
4. Semantic HTML using <nav>, <ul>, <li>, <button>
5. Focus management: trap focus inside mobile menu when open
6. Dark mode support using CSS custom properties

After initial implementation, I will ask you to audit for WCAG compliance.
```

**Iterative refinement prompt** (after initial implementation):
```
Audit the navigation for WCAG 2.1 AA compliance. List any issues found.
```

## Flow A: OpenCode + Ripgrep Haystack

### Setup
```bash
cd ~/projects/frontend-test

# Ensure terraphim MCP is enabled in opencode
# ~/.config/opencode/opencode.json should have:
# "mcp": { "terraphim": { "type": "local", ... } }

opencode
```

### Run Experiment
1. In OpenCode, paste the Task Prompt
2. Let OpenCode implement the navigation
3. Note: OpenCode should use `terraphim search` or MCP tools
4. After completion, paste the Iterative Refinement Prompt
5. Capture the WCAG audit results

### Capture Results
```bash
# In a separate terminal, capture session
tmux new -s flow-a -d
tmux capture-pane -t flow-a -p > ~/flow-a-session.txt

# Or press Ctrl+Z to background and capture
```

## Flow B: OpenCode + FFF MCP Tools

### Setup
```bash
cd ~/projects/frontend-test

# Ensure FFF tools are available (terraphim_mcp_server)
opencode
```

### Run Experiment
1. In OpenCode, paste the Task Prompt
2. Note: OpenCode should use `terraphim_find_files`, `terraphim_grep` directly
3. After completion, paste the Iterative Refinement Prompt
4. Capture the WCAG audit results

## Flow C: Control (No Terraphim)

### Setup
```bash
cd ~/projects/frontend-test

# Disable terraphim MCP in opencode
# Comment out terraphim entry in ~/.config/opencode/opencode.json

opencode
```

### Run Experiment
1. In OpenCode, paste the Task Prompt
2. OpenCode uses default search only (no terraphim tools)
3. After completion, paste the Iterative Refinement Prompt
4. Capture the WCAG audit results

## Metrics to Capture

| Metric | Flow A | Flow B | Flow C |
|--------|--------|--------|--------|
| Time to initial solution | | | |
| Total session time | | | |
| Iterations needed | | | |
| Tool calls count | | | |
| terraphim tool calls | | | |
| Lines of code | | | |
| WCAG violations (initial) | | | |
| WCAG violations (final) | | | |
| User rating (1-5) | | | |

## Session Capture Template

Create a file for each flow:

**`~/experiment/flow-a-results.md`**:
```markdown
# Flow A: OpenCode + Terraphim (Ripgrep Haystack)

## Setup
- Date: YYYY-MM-DD
- OpenCode version: X.X.X
- terraphim-agent version: X.X.X

## Session Log
[paste OpenCode session transcript]

## Metrics
- Time to solution:
- Iterations:
- Tool calls:
- WCAG violations (initial):
- WCAG violations (final):

## Code Quality Assessment
- Accessibility:
- TypeScript correctness:
- Svelte idioms:
- CSS quality:

## Notes
[observations]
```

## Running the Experiment

### Quick Start

```bash
# 1. Create experiment directory
mkdir -p ~/experiment

# 2. Backup opencode config
cp ~/.config/opencode/opencode.json ~/.config/opencode/opencode.json.bak

# 3. Run Flow A (with terraphim)
# - Enable terraphim MCP
# - opencode
# - Paste prompt
# - Save session to ~/experiment/flow-a.md

# 4. Run Flow B (with FFF)
# - Same session, OpenCode should use FFF tools
# - Save session to ~/experiment/flow-b.md

# 5. Run Flow C (control)
# - Disable terraphim MCP
# - opencode
# - Paste prompt
# - Save session to ~/experiment/flow-c.md

# 6. Restore config
cp ~/.config/opencode/opencode.json.bak ~/.config/opencode/opencode.json
```

### Expected Observations

| Aspect | Flow A (Terraphim) | Flow B (FFF) | Flow C (Control) |
|--------|---------------------|--------------|------------------|
| Search approach | KG-boosted haystack | Direct MCP tools | Default |
| Pattern suggestions | Concept-aware | File-aware | Generic |
| WCAG compliance | Higher (knowledgeable) | Variable | Baseline |
| Development speed | Faster for known patterns | Fast file access | Baseline |

## Post-Experiment Analysis

After running all three flows:

1. Compare WCAG compliance scores
2. Compare code quality (accessibility, TypeScript, Svelte idioms)
3. Analyze tool usage patterns
4. Document which approach felt most helpful

## Troubleshooting

### "terraphim not found" in OpenCode
```bash
# Check terraphim_mcp_server is in PATH
which terraphim_mcp_server
# If not, copy it
cp ~/projects/terraphim/terraphim-ai/target/release/terraphim_mcp_server ~/.cargo/bin/
```

### "Role not found"
```bash
terraphim-agent config reload
terraphim-agent roles list
```

### GrepApp not working
GrepApp requires `--features grepapp` build. For this experiment, use Ripgrep only.
