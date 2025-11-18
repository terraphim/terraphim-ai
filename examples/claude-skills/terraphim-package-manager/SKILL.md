---
name: terraphim-package-manager
description: Automatically replace package manager commands (npm, yarn, pnpm) with bun using Terraphim's knowledge graph system. Use this when the user mentions package managers, installation commands, or build scripts that use npm/yarn/pnpm.
---

# Terraphim Package Manager Replacement

This skill uses Terraphim's knowledge graph to automatically replace package manager commands with preferred alternatives (e.g., npm → bun).

## When to Use This Skill

Activate this skill when:
- User mentions npm, yarn, or pnpm commands
- User asks to install dependencies
- User shares package.json scripts
- User provides shell scripts with package manager commands
- User discusses build/test/dev commands

## How It Works

The skill uses `terraphim-tui replace` command which:
1. Loads a knowledge graph from markdown files
2. Uses Aho-Corasick automata for fast pattern matching
3. Replaces all package manager references with bun
4. Preserves the rest of the text unchanged

## Capabilities

**Supported Replacements:**
- `npm` → `bun`
- `yarn` → `bun`
- `pnpm` → `bun`
- `npm install` → `bun install`
- `yarn install` → `bun install`
- `pnpm install` → `bun install`

**Features:**
- Case-insensitive matching (NPM = npm)
- Longest match first (npm install before npm)
- Sub-100ms execution time
- Non-overlapping replacements

## Instructions

### Step 1: Check if terraphim-tui is available

Before using this skill, verify terraphim-tui is built:

```bash
ls -lh target/release/terraphim-tui
```

If not found, build it:

```bash
cargo build --release -p terraphim_tui
```

### Step 2: Identify package manager commands

Look for patterns like:
- "npm install"
- "yarn build"
- "pnpm test"
- package.json scripts with npm/yarn/pnpm

### Step 3: Replace using terraphim-tui

Use the replace command:

```bash
terraphim-tui replace "TEXT_TO_REPLACE" 2>/dev/null
```

**Important**: Suppress stderr with `2>/dev/null` to avoid log messages.

### Step 4: Present the results

Show the user:
1. Original text
2. Replaced text
3. Brief explanation of what changed

## Examples

### Example 1: Simple Command Replacement

**User asks:** "How do I install dependencies?"

**Before replacement:**
```bash
npm install
```

**After replacement:**
```bash
bun install
```

**Your response:**
```
To install dependencies with bun:

\`\`\`bash
bun install
\`\`\`

I've replaced `npm install` with `bun install` using Terraphim's knowledge graph.
```

### Example 2: Package.json Scripts

**User shares:**
```json
{
  "scripts": {
    "install": "npm install",
    "build": "yarn build",
    "test": "pnpm test"
  }
}
```

**After replacement:**
```json
{
  "scripts": {
    "install": "bun install",
    "build": "bun build",
    "test": "bun test"
  }
}
```

### Example 3: Shell Script

**User provides:**
```bash
#!/bin/bash
npm install
npm run build
npm test
```

**After replacement:**
```bash
#!/bin/bash
bun install
bun run build
bun test
```

## Best Practices

1. **Always explain the replacement**: Don't silently change commands without telling the user
2. **Show before and after**: Let users see what was changed
3. **Respect user preferences**: If user explicitly wants npm, don't override
4. **Handle errors gracefully**: If terraphim-tui fails, provide the original text
5. **Suppress logs**: Always use `2>/dev/null` to avoid log clutter

## Error Handling

If terraphim-tui is not available:
1. Inform the user
2. Provide manual replacement guidance
3. Suggest building terraphim-tui

Example error response:
```
I don't have terraphim-tui available to perform automatic replacement.
To use bun instead of npm, manually replace:
- npm → bun
- npm install → bun install
- npm run → bun run
```

## Integration with Knowledge Graph

This skill leverages Terraphim's knowledge graph system defined in:
- `docs/src/kg/bun.md`: Package manager synonyms
- `docs/src/kg/bun_install.md`: Install command synonyms

These markdown files define semantic relationships that power the replacement logic.

## Performance Notes

- Pattern matching: ~10-50ms
- Knowledge graph loading: ~100-200ms (cached after first run)
- Total execution: typically under 100ms

## Related Commands

```bash
# Replace text
terraphim-tui replace "TEXT" 2>/dev/null

# Replace with role
terraphim-tui replace "TEXT" --role "Terraphim Engineer" 2>/dev/null

# Replace with output format
terraphim-tui replace "TEXT" --format markdown 2>/dev/null
```

## Limitations

- Requires terraphim-tui to be built
- Works best with specific command patterns
- May be aggressive with longer text containing many technical terms
- Requires knowledge graph files in docs/src/kg/

## See Also

- Claude Code Hook: `examples/claude-code-hooks/README.md`
- Knowledge Graph: `docs/src/kg/PACKAGE_MANAGER_REPLACEMENT.md`
- Terraphim TUI: `crates/terraphim_tui/README.md`
