# Terraphim Integration with Claude: Complete Guide

This guide explains how to integrate Terraphim's knowledge graph capabilities with Claude through two different approaches: **Hooks** and **Skills**.

## Table of Contents

- [Overview](#overview)
- [Approach Comparison](#approach-comparison)
- [Claude Code Hooks](#claude-code-hooks)
- [Claude Skills](#claude-skills)
- [Which Approach to Use](#which-approach-to-use)
- [Getting Started](#getting-started)
- [Advanced Integration](#advanced-integration)

## Overview

Terraphim provides knowledge graph-based text replacement capabilities through its `terraphim-tui` command-line tool. This can be integrated with Claude in two ways:

1. **Hooks**: Automatic, transparent interception of user input
2. **Skills**: Context-aware, conversational assistance

Both approaches use the same underlying technology:
- **Knowledge Graph**: Semantic relationships defined in markdown files
- **Aho-Corasick Automata**: Fast pattern matching (O(n + m))
- **Terraphim-TUI**: Command-line interface for replacements

## Approach Comparison

| Feature | Claude Code Hooks | Claude Skills |
|---------|-------------------|---------------|
| **Activation** | Automatic on every prompt | Context-aware when relevant |
| **User Visibility** | Transparent (optional notification) | Conversational with explanation |
| **Platform** | Claude Code CLI only | All Claude platforms |
| **Setup Location** | Hook script in settings | Skill in ~/.claude/skills/ |
| **User Control** | Environment variables | Natural language direction |
| **Execution Timing** | Before Claude sees input | During Claude's response |
| **Best For** | Consistent enforcement | Interactive collaboration |
| **Explanation** | None (or minimal log) | Full context and reasoning |
| **Complexity** | Medium (bash scripting) | Low (markdown + optional scripts) |
| **Cross-Platform** | No (CLI only) | Yes (all surfaces) |

## Claude Code Hooks

### What Are Hooks?

Hooks are shell commands that execute in response to events like user prompt submission. They modify input before Claude sees it.

### How Hooks Work

```
User Input → Hook Script → Modified Input → Claude
```

The hook intercepts the input, processes it, and returns modified text.

### Hook Architecture

```bash
#!/usr/bin/env bash
# Read user input
INPUT=$(cat)

# Process with terraphim-tui
REPLACED=$(terraphim-tui replace "$INPUT" 2>/dev/null)

# Return modified input
echo "$REPLACED"
```

### Hook Configuration

**File**: `~/.config/claude-code/settings.json`

```json
{
  "hooks": {
    "user-prompt-submit": {
      "command": "bash",
      "args": ["/path/to/terraphim-package-manager-hook.sh"],
      "enabled": true,
      "description": "Replace package manager commands with bun"
    }
  }
}
```

### Hook Modes

Hooks support three operational modes:

**Replace Mode (default)**
```bash
export HOOK_MODE=replace
```
Automatically replaces all package manager commands without notification.

**Suggest Mode**
```bash
export HOOK_MODE=suggest
```
Shows suggestions in stderr but keeps original input.

**Passive Mode**
```bash
export HOOK_MODE=passive
```
Only logs what would be replaced, doesn't modify input.

### When to Use Hooks

✅ **Use hooks when:**
- You want consistent, automatic enforcement
- You don't need explanations for changes
- You're using Claude Code CLI exclusively
- You want replacements to happen transparently
- You have clear, well-defined patterns to match

❌ **Don't use hooks when:**
- You want Claude to explain changes
- You need context-aware decisions
- You're using multiple Claude platforms
- You want interactive control
- Patterns are complex or context-dependent

### Hook Example

**Location**: `examples/claude-code-hooks/`

**Files**:
- `terraphim-package-manager-hook.sh` - Hook script
- `test-hook.sh` - Test suite
- `claude-settings-example.json` - Configuration example
- `README.md` - Comprehensive guide

**Documentation**: See `examples/claude-code-hooks/README.md`

### Real-World Examples

**1. Package Manager Replacement (bun)**

Knowledge Graph Files:
- `docs/src/kg/bun.md` - Maps npm/yarn/pnpm → bun
- `docs/src/kg/bun_install.md` - Maps installation commands → bun_install

Example Replacements:
```bash
$ terraphim-tui replace "npm install && yarn test"
bun_install && bun test
```

**2. Attribution Replacement (Claude → Terraphim)**

Knowledge Graph Files:
- `docs/src/kg/terraphim_ai.md` - Maps "Claude Code" → terraphim_ai
- `docs/src/kg/https___terraphim_ai.md` - Maps Claude URLs → Terraphim URLs
- `docs/src/kg/generated_with_terraphim.md` - Maps attribution text
- `docs/src/kg/noreply_terraphim.md` - Maps email addresses

Example Replacements:
```bash
$ terraphim-tui replace "🤖 Generated with [Claude Code](https://claude.com/claude-code) Co-Authored-By: Claude <noreply@anthropic.com>"
🤖 Generated with [terraphim_ai](https___terraphim_ai) Co-Authored-By: terraphim_ai <noreply_terraphim>
```

**Note on Output Format**: Replacement text uses filename stems, so spaces become underscores and special characters are converted (e.g., "https://terraphim.ai" → "https___terraphim_ai"). This is a fundamental design of the system where filenames serve as normalized replacement terms.

## Claude Skills

### What Are Skills?

Skills are modular capabilities that extend Claude's functionality through instructions, metadata, and optional resources.

### How Skills Work

```
User Message → Claude Detects Context → Loads Skill → Executes Logic → Responds with Explanation
```

Skills use progressive disclosure:
1. **Metadata** (always loaded, ~100 tokens)
2. **Instructions** (loaded when triggered, <5k tokens)
3. **Resources** (loaded on-demand, output only)

### Skill Architecture

**File**: `SKILL.md`

```yaml
---
name: terraphim-package-manager
description: Replace npm/yarn/pnpm with bun using knowledge graph
---

# Skill Title

## Instructions
[Step-by-step guide for Claude]

## Examples
[Concrete usage examples]
```

### Skill Structure

```
terraphim-package-manager/
├── SKILL.md              # Main skill definition
├── replace.sh            # Helper script (optional)
├── README.md             # Documentation
└── examples/             # Example files (optional)
```

### Skill Configuration

**Claude Code**: Place in `~/.claude/skills/` or `.claude/skills/` (project-specific)

```bash
mkdir -p ~/.claude/skills/terraphim-package-manager
cp -r examples/claude-skills/terraphim-package-manager/* \
      ~/.claude/skills/terraphim-package-manager/
```

**Claude.ai**: Upload as zip file via Settings → Skills

```bash
cd examples/claude-skills
zip -r terraphim-package-manager.zip terraphim-package-manager/
```

**Claude API**: Specify `skill_id` in API requests

### Skill Activation

Skills activate automatically when Claude detects relevant context:

**Triggers**:
- User mentions npm, yarn, or pnpm
- User asks about installation
- User shares package.json files
- User provides shell scripts with package managers

### When to Use Skills

✅ **Use skills when:**
- You want Claude to explain changes
- You need context-aware decisions
- You're working across multiple platforms
- You want interactive control
- You need Claude to learn when to apply

❌ **Don't use skills when:**
- You want silent, automatic replacements
- You don't need explanations
- You're only using Claude Code CLI
- You want pre-processing before Claude sees input
- Performance is critical (skills add token overhead)

### Skill Example

**Location**: `examples/claude-skills/terraphim-package-manager/`

**Files**:
- `SKILL.md` - Skill definition with YAML frontmatter
- `replace.sh` - Helper script for replacements
- `README.md` - Complete documentation
- `examples/` - Example package.json and scripts

**Documentation**: See `examples/claude-skills/terraphim-package-manager/README.md`

## Which Approach to Use

### Decision Matrix

| Your Need | Recommended Approach |
|-----------|---------------------|
| Automatic, silent replacements | **Hook** |
| Explanations and context | **Skill** |
| Claude Code CLI only | **Hook** |
| All Claude platforms | **Skill** |
| Pre-processing input | **Hook** |
| Interactive collaboration | **Skill** |
| Learning and adaptation | **Skill** |
| Consistent enforcement | **Hook** |
| Complex decision-making | **Skill** |
| Simple pattern matching | **Hook** |

### Use Both Approaches

You can use both simultaneously:

**Hook**: Automatically replace obvious patterns (npm → bun)
**Skill**: Help with complex scenarios (migration planning, documentation updates)

### Migration Path

**Phase 1**: Start with skill for learning
- Claude explains what would be replaced
- You learn the patterns
- You validate the approach

**Phase 2**: Add hook for automation
- Once patterns are validated, add hook
- Hook handles common cases automatically
- Skill handles edge cases

**Phase 3**: Optimize based on usage
- Keep hook for 95% of cases
- Use skill for remaining 5%
- Update knowledge graph based on experience

## Getting Started

### Prerequisites

Both approaches require:

1. **Terraphim-TUI built**:
   ```bash
   cargo build --release -p terraphim_tui
   ```

2. **Knowledge graph files**:
   - `docs/src/kg/bun.md` - Package manager synonyms
   - `docs/src/kg/bun_install.md` - Install command synonyms

3. **PATH configured** (optional):
   ```bash
   export PATH="$PATH:$(pwd)/target/release"
   ```

### Quick Start: Hooks

```bash
# 1. Copy hook script
cp examples/claude-code-hooks/terraphim-package-manager-hook.sh ~/

# 2. Make executable
chmod +x ~/terraphim-package-manager-hook.sh

# 3. Configure Claude Code
mkdir -p ~/.config/claude-code
cat > ~/.config/claude-code/settings.json << 'EOF'
{
  "hooks": {
    "user-prompt-submit": {
      "command": "bash",
      "args": ["/home/user/terraphim-package-manager-hook.sh"],
      "enabled": true
    }
  }
}
EOF

# 4. Test
echo "npm install" | ~/terraphim-package-manager-hook.sh
```

### Quick Start: Skills

```bash
# 1. Install skill
mkdir -p ~/.claude/skills/terraphim-package-manager
cp -r examples/claude-skills/terraphim-package-manager/* \
      ~/.claude/skills/terraphim-package-manager/

# 2. Make script executable
chmod +x ~/.claude/skills/terraphim-package-manager/replace.sh

# 3. Test
cd ~/.claude/skills/terraphim-package-manager
./replace.sh "npm install"

# 4. Use with Claude
# Start Claude and mention package managers
```

## Advanced Integration

### Combining Hooks and Skills

Use both for maximum flexibility:

```json
// Claude Code settings.json
{
  "hooks": {
    "user-prompt-submit": {
      "command": "bash",
      "args": ["/path/to/hook.sh"],
      "enabled": true
    }
  }
}
```

Plus:

```bash
# Skill for advanced scenarios
~/.claude/skills/terraphim-package-manager/
```

**Workflow**:
1. Hook handles simple replacements automatically
2. Claude uses skill for complex cases
3. User gets best of both worlds

### Custom Knowledge Graphs

Create domain-specific replacements:

**Frontend Developer**:
```markdown
# React
synonyms:: vue, angular, svelte
```

**Backend Developer**:
```markdown
# FastAPI
synonyms:: flask, django, express
```

**DevOps Engineer**:
```markdown
# Docker
synonyms:: podman, containerd
```

### Multi-Step Workflows

Create a workflow skill that uses the package manager skill:

```yaml
---
name: full-stack-migration
description: Complete migration from npm to bun for full-stack projects
---

# Full-Stack Migration Workflow

1. Use terraphim-package-manager skill for frontend
2. Use terraphim-package-manager skill for backend
3. Update Docker files
4. Update CI/CD configuration
5. Update documentation
6. Run tests
```

### Role-Based Configuration

Use different Terraphim roles for different contexts:

```bash
# Frontend work
export TERRAPHIM_ROLE="Frontend Engineer"

# Backend work
export TERRAPHIM_ROLE="Backend Engineer"

# DevOps work
export TERRAPHIM_ROLE="DevOps Engineer"
```

Each role can have its own knowledge graph and preferences.

### CI/CD Integration

Integrate both approaches in CI/CD:

**Hook in Pre-commit**:
```bash
# .git/hooks/pre-commit
#!/bin/bash
find . -name "*.sh" | while read file; do
    terraphim-tui replace "$(cat $file)" > $file.new
    mv $file.new $file
done
```

**Skill in GitHub Actions**:
```yaml
# .github/workflows/validate.yml
- name: Check package manager usage
  run: |
    # Use Claude API with skill to analyze and suggest improvements
    claude-api --skill terraphim-package-manager validate
```

## Troubleshooting

### Common Issues

**Hook not executing**:
1. Check hook script path in settings.json
2. Verify script is executable (`chmod +x`)
3. Test script directly: `echo "npm install" | ./hook.sh`

**Skill not loading**:
1. Verify location (`~/.claude/skills/` or `.claude/skills/`)
2. Check YAML frontmatter is valid
3. Ensure `name` and `description` fields exist

**terraphim-tui not found**:
1. Build: `cargo build --release -p terraphim_tui`
2. Add to PATH: `export PATH="$PATH:$(pwd)/target/release"`
3. Use absolute path in scripts

**Replacements not working**:
1. Test directly: `terraphim-tui replace "npm install" 2>/dev/null`
2. Check knowledge graph files exist: `ls docs/src/kg/bun*.md`
3. Verify synonyms are defined: `grep "synonyms::" docs/src/kg/bun.md`

### Performance Tuning

**Hook Performance**:
- Pattern matching: ~10-50ms
- Knowledge graph loading: ~100-200ms (cached)
- Total execution: <100ms typically

**Skill Performance**:
- Metadata loading: ~100 tokens (always)
- Instructions loading: ~5k tokens (when triggered)
- Script execution: ~10-50ms
- Total overhead: Minimal, only loads when relevant

## Best Practices

### For Hooks

1. ✅ Suppress stderr: `2>/dev/null`
2. ✅ Handle errors gracefully: `|| echo "$INPUT"`
3. ✅ Test before deploying
4. ✅ Use environment variables for configuration
5. ✅ Log in suggest/passive mode during testing

### For Skills

1. ✅ Write clear descriptions that trigger appropriately
2. ✅ Provide concrete examples in SKILL.md
3. ✅ Keep instructions focused and actionable
4. ✅ Test with actual conversations
5. ✅ Audit scripts for security

### For Both

1. ✅ Version control knowledge graph files
2. ✅ Document custom synonyms
3. ✅ Test with real-world examples
4. ✅ Monitor performance
5. ✅ Gather user feedback and iterate

## Resources

### Documentation

- **Hooks Guide**: `examples/claude-code-hooks/README.md`
- **Skills Guide**: `examples/claude-skills/terraphim-package-manager/README.md`
- **Knowledge Graph**: `docs/src/kg/PACKAGE_MANAGER_REPLACEMENT.md`
- **Terraphim TUI**: `crates/terraphim_tui/README.md`

### External Links

- **Claude Skills Docs**: https://docs.claude.com/en/docs/agents-and-tools/agent-skills/overview
- **Skills Cookbook**: https://github.com/anthropics/claude-cookbooks/tree/main/skills
- **Claude Code**: https://code.claude.com/

### Examples

- **Hook Tests**: `examples/claude-code-hooks/test-hook.sh`
- **Skill Examples**: `examples/claude-skills/terraphim-package-manager/examples/`
- **Knowledge Graph**: `docs/src/kg/`

## Contributing

Improvements welcome!

**Hook Improvements**:
- Add more modes (audit, report, interactive)
- Support more configuration options
- Add telemetry/metrics

**Skill Improvements**:
- Add more examples
- Improve error messages
- Support more package managers

**Knowledge Graph Improvements**:
- Add more domains (databases, frameworks, tools)
- Create role-specific graphs
- Add validation and testing

## License

This integration guide and examples are part of the Terraphim AI project and follow the same license (Apache-2.0).
