# Terraphim Package Manager Skill

This Claude Skill uses Terraphim's knowledge graph to automatically replace package manager commands (npm/yarn/pnpm) with bun.

## What is a Claude Skill?

Claude Skills are modular capabilities that extend Claude's functionality. They package instructions, metadata, and optional resources that Claude automatically uses when relevant to user requests.

**Key Benefits:**
- **Progressive Disclosure**: Skills load in layers (metadata → instructions → resources)
- **Automatic Activation**: Claude knows when to use the skill based on context
- **Reusable**: Create once, use across conversations
- **Composable**: Combine with other skills for complex workflows

## Skill vs Hook: Which to Use?

### Use Claude Skill When:
- ✅ You want Claude to proactively help with package manager replacements
- ✅ You want conversational context (Claude explains changes)
- ✅ You're working interactively with Claude
- ✅ You want Claude to learn when to apply replacements

### Use Claude Code Hook When:
- ✅ You want automatic, silent replacements on every prompt
- ✅ You want consistent enforcement without explanation
- ✅ You're using Claude Code CLI specifically
- ✅ You want the hook to run before Claude sees your input

**Summary**: Skills are conversational and context-aware; Hooks are automatic and transparent.

## Quick Start

### 1. Install the Skill

**For Claude Code:**
```bash
# Skills go in ~/.claude/skills/ directory
mkdir -p ~/.claude/skills/terraphim-package-manager
cp -r examples/claude-skills/terraphim-package-manager/* ~/.claude/skills/terraphim-package-manager/
```

**For Claude.ai:**
1. Create a zip file:
   ```bash
   cd examples/claude-skills
   zip -r terraphim-package-manager.zip terraphim-package-manager/
   ```
2. Upload to Claude.ai via Settings → Skills
3. Enable the skill

**For Claude API:**
Skills are specified via the `skill_id` parameter in API calls.

### 2. Build terraphim-tui

The skill requires the terraphim-tui binary:

```bash
cargo build --release -p terraphim_tui
```

### 3. Test the Skill

**Option A: Use the helper script**
```bash
cd ~/.claude/skills/terraphim-package-manager
./replace.sh "npm install dependencies"
# Output: bun install dependencies
```

**Option B: Direct command**
```bash
terraphim-tui replace "npm install" 2>/dev/null
# Output: bun install
```

### 4. Use with Claude

Start a conversation with Claude and mention package managers:

```
User: How do I install dependencies for my project?

Claude: To install dependencies with bun:

\`\`\`bash
bun install
\`\`\`

I've replaced `npm install` with `bun install` using Terraphim's knowledge graph system.
```

## Skill Structure

```
terraphim-package-manager/
├── SKILL.md              # Main skill definition with YAML frontmatter
├── replace.sh            # Helper script for replacements
├── README.md             # This file
└── examples/             # (Optional) Example files
    ├── package.json      # Example package.json
    └── script.sh         # Example shell script
```

### SKILL.md Format

The skill follows Claude's standard format:

```yaml
---
name: terraphim-package-manager
description: Automatically replace package manager commands (npm, yarn, pnpm) with bun
---

# Skill Title

## Instructions
[How Claude should use this skill]

## Examples
[Concrete examples for Claude]
```

**Key Fields:**
- `name`: Skill identifier (lowercase, hyphens, max 64 chars)
- `description`: What the skill does and when to trigger it (max 1024 chars)

## How It Works

### Progressive Loading

Skills load in three levels:

**Level 1 - Metadata (Always Loaded)**
```yaml
name: terraphim-package-manager
description: Replace npm/yarn/pnpm with bun
```
~100 tokens. Claude knows the skill exists.

**Level 2 - Instructions (Triggered Loading)**
The main SKILL.md body loads when Claude detects:
- User mentions npm, yarn, or pnpm
- User asks about installation
- User shares package.json scripts

Under 5k tokens with instructions and examples.

**Level 3 - Resources (On-Demand)**
The `replace.sh` script executes only when Claude needs to perform a replacement. Script output enters context, but the script itself doesn't consume tokens.

### Execution Flow

```
User Message
    ↓
Claude detects package manager mention
    ↓
Skill metadata matches → Load Level 2 instructions
    ↓
Claude decides to replace text
    ↓
Execute replace.sh script → Get output
    ↓
Claude presents results with explanation
```

## Examples

### Example 1: Interactive Replacement

**User:** "Write a script to install and build my app"

**Claude:**
```
Here's a script using bun:

\`\`\`bash
#!/bin/bash
bun install
bun run build
\`\`\`

I've used bun instead of npm/yarn for faster performance.
```

### Example 2: Package.json Conversion

**User:** "Convert my package.json to use bun"

**Before:**
```json
{
  "scripts": {
    "install": "npm install",
    "dev": "npm run dev",
    "build": "yarn build"
  }
}
```

**After (Claude provides):**
```json
{
  "scripts": {
    "install": "bun install",
    "dev": "bun run dev",
    "build": "bun build"
  }
}
```

### Example 3: Documentation Update

**User:** "Update the README to use bun"

**Before:**
```markdown
## Installation

Run `npm install` to install dependencies.
```

**After (Claude provides):**
```markdown
## Installation

Run `bun install` to install dependencies.
```

## Configuration

### Environment Variables

```bash
# Use specific terraphim-tui binary
export TERRAPHIM_TUI_BIN=/path/to/terraphim-tui

# Use specific role
export TERRAPHIM_ROLE="My Custom Role"
```

### Customizing Replacements

Edit the knowledge graph files:

**`docs/src/kg/bun.md`:**
```markdown
# Bun

Bun is a modern JavaScript runtime.

synonyms:: pnpm, npm, yarn
```

**`docs/src/kg/bun_install.md`:**
```markdown
# bun install

Fast package installation.

synonyms:: pnpm install, npm install, yarn install
```

Add more synonym files as needed.

## Combining with Other Skills

Skills can work together. For example:

**Workflow: Create + Replace + Format**
1. User: "Create a Node.js project structure"
2. Claude uses project-structure skill → Creates files
3. Claude uses terraphim-package-manager skill → Replaces npm with bun
4. Claude uses prettier skill → Formats the code

## Platform-Specific Notes

### Claude Code

- Skills in `~/.claude/skills/` or `.claude/skills/` (project-specific)
- Full network access
- Can execute scripts
- Best for development workflows

### Claude.ai

- Upload skills as zip files
- May have restricted network access
- Scripts might be limited
- Best for interactive conversations

### Claude API

- Specify `skill_id` in API requests
- No network access
- Cannot install packages
- Best for automated workflows

## Troubleshooting

### Skill Not Loading

1. **Check location**: Skills must be in correct directory
   - Claude Code: `~/.claude/skills/` or `.claude/skills/`
   - Claude.ai: Upload via Settings

2. **Verify YAML frontmatter**: Must be valid YAML with `name` and `description`
   ```bash
   # Test YAML validity
   python3 -c "import yaml; yaml.safe_load(open('SKILL.md').read().split('---')[1])"
   ```

3. **Check file permissions**: Ensure scripts are executable
   ```bash
   chmod +x replace.sh
   ```

### terraphim-tui Not Found

```bash
# Check if built
ls -lh target/release/terraphim-tui

# Build if needed
cargo build --release -p terraphim_tui

# Add to PATH
export PATH="$PATH:$(pwd)/target/release"
```

### Replacement Not Working

1. **Test directly**:
   ```bash
   ./replace.sh "npm install"
   ```

2. **Check knowledge graph**:
   ```bash
   ls docs/src/kg/bun*.md
   ```

3. **Verify synonyms**:
   ```bash
   grep "synonyms::" docs/src/kg/bun.md
   ```

## Security Considerations

**⚠️ Important**: Skills execute code with your permissions.

**Best Practices:**
- ✅ Only install skills from trusted sources
- ✅ Review all scripts before enabling
- ✅ Check for unexpected network calls
- ✅ Audit bundled dependencies
- ✅ Use version control for skill changes

**Red Flags:**
- ❌ Unmarked external URL fetches
- ❌ Unusual system operations
- ❌ Requests for elevated permissions
- ❌ Obfuscated code

## Performance

- **Metadata loading**: ~100 tokens (always)
- **Instructions loading**: ~5k tokens (when triggered)
- **Script execution**: ~10-50ms
- **Total overhead**: Minimal, skill only loads when relevant

## Comparison: Skill vs Hook vs Manual

| Feature | Skill | Hook | Manual |
|---------|-------|------|--------|
| Automatic | Context-aware | Always | No |
| Explanation | Yes | No | No |
| User control | High | Medium | Full |
| Setup complexity | Low | Medium | None |
| Claude integration | Native | External | None |
| Cross-platform | Yes | CLI only | Yes |

## Advanced Usage

### Multi-Step Workflows

Create a workflow skill that uses this skill:

```yaml
---
name: nodejs-to-bun-migration
description: Complete migration from Node.js with npm to Bun
---

# Node.js to Bun Migration

1. Use terraphim-package-manager skill to replace commands
2. Update Dockerfile
3. Update CI/CD configuration
4. Test the migration
```

### Custom Roles

Use different Terraphim roles for different projects:

```bash
# In project A
export TERRAPHIM_ROLE="Frontend Engineer"

# In project B
export TERRAPHIM_ROLE="Backend Engineer"
```

Each role can have different knowledge graphs and preferences.

### Integration with CI/CD

```yaml
# .github/workflows/convert-to-bun.yml
name: Convert to Bun
on: [push]
jobs:
  convert:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2
      - name: Build terraphim-tui
        run: cargo build --release -p terraphim_tui
      - name: Convert scripts
        run: |
          find . -name "*.sh" -exec \
            sh -c 'terraphim-tui replace "$(cat {})" > {}.new && mv {}.new {}' \;
```

## Related Resources

- **Hook Implementation**: `examples/claude-code-hooks/README.md`
- **Knowledge Graph Guide**: `docs/src/kg/PACKAGE_MANAGER_REPLACEMENT.md`
- **Terraphim TUI Docs**: `crates/terraphim_tui/README.md`
- **Claude Skills Docs**: https://docs.claude.com/en/docs/agents-and-tools/agent-skills/overview
- **Skills Cookbook**: https://github.com/anthropics/claude-cookbooks/tree/main/skills

## Contributing

To improve this skill:

1. Test with real projects
2. Add more examples
3. Improve error handling
4. Add support for other package managers
5. Create variants for different languages

## License

This skill is part of the Terraphim AI project and follows the same license (Apache-2.0).
