# Package Manager Replacement with Bun

This guide shows how to use Terraphim's knowledge graph and replace functionality to automatically convert package manager commands (pnpm, npm, yarn) to Bun.

## Overview

The knowledge graph in `docs/src/kg/bun.md` defines synonyms for package managers that all map to "Bun". The terraphim-tui's `/replace` command uses this knowledge graph to perform text replacements.

## Knowledge Graph Entry

File: `docs/src/kg/bun.md`

```markdown
# Bun

Bun is a modern JavaScript runtime and package manager.

(synonyms are defined in bun.md and bun-install.md files)
```

This creates a thesaurus mapping:
- `pnpm install` → `bun install`
- `npm install` → `bun install`
- `yarn install` → `bun install`
- `pnpm` → `bun`
- `npm` → `bun`
- `yarn` → `bun`

## Usage with TUI REPL

### Start the Server

```bash
# Terminal 1: Start terraphim server
cargo run --release -- --config context_engineer_config.json
```

The server will build a knowledge graph from all markdown files in `docs/src/kg/`.

### Start TUI REPL

```bash
# Terminal 2: Start TUI REPL in offline mode
cargo run --release -p terraphim_tui --bin terraphim-tui --features repl,repl-mcp -- repl
```

### Use Replace Command

```bash
# In the REPL prompt:
/replace "pnpm install dependencies"
# Output: bun install dependencies

/replace "npm install && yarn build"
# Output: bun install && bun build

/replace "PNPM INSTALL"
# Output: bun install (case insensitive)
```

### Replace Command Options

The `/replace` command supports different output formats:

```bash
# Plain text replacement (default)
/replace "npm install"
# Output: bun install

# Markdown links
/replace "npm install" --format markdown
# Output: [bun install](url)

# Wiki links
/replace "npm install" --format wiki
# Output: [[bun install]]

# HTML links
/replace "npm install" --format html
# Output: <a href="url">bun install</a>
```

## Usage from CLI

### Direct Text Replacement

```bash
# Pipe text through replace command
echo "npm install" | cargo run --release -p terraphim_tui --bin terraphim-tui -- \
  replace --text "npm install"
```

### Process Files

```bash
# Replace in a file
cargo run --release -p terraphim_tui --bin terraphim-tui -- \
  replace --file package.json
```

## How It Works

### 1. Knowledge Graph Building

When terraphim starts, it:
1. Scans `docs/src/kg/*.md` files
2. Extracts `synonyms::` lines using Logseq builder
3. Creates a thesaurus mapping synonyms to normalized terms
4. Builds Aho-Corasick automata for fast pattern matching

### 2. Replace Process

When you run `/replace`:
1. Loads the thesaurus for the current role
2. Uses Aho-Corasick to find all synonym matches in text
3. Replaces matches with normalized term based on link type
4. Returns the transformed text

### 3. Pattern Matching

The replacement is:
- **Case insensitive**: "pnpm" and "PNPM" both match
- **Leftmost longest**: Matches longest pattern first ("pnpm install" over "pnpm")
- **Non-overlapping**: Each position matched only once

## Examples

### Example 1: Convert Scripts

```bash
# Input script
#!/bin/bash
pnpm install
pnpm build
pnpm test

# After /replace
#!/bin/bash
bun install
bun build
bun test
```

### Example 2: Convert package.json Scripts

```json
{
  "scripts": {
    "install": "pnpm install",
    "build": "npm run build",
    "test": "yarn test"
  }
}
```

After replacement:
```json
{
  "scripts": {
    "install": "bun install",
    "build": "bun run build",
    "test": "bun test"
  }
}
```

### Example 3: Convert Documentation

```markdown
# Installation

Run `npm install` to install dependencies.

For development, use `yarn dev`.
```

After replacement:
```markdown
# Installation

Run `bun install` to install dependencies.

For development, use `bun dev`.
```

## Adding More Synonyms

To add more package manager commands, edit `docs/src/kg/bun.md`:

```markdown
# Bun

Bun is a modern JavaScript runtime and package manager.

(synonyms are defined in bun.md and bun-install.md files)
```

Then restart the terraphim server to rebuild the knowledge graph.

## Testing

### Manual Testing

```bash
# Test basic replacement
/replace "npm install"
# Expected: bun install

# Test multiple replacements
/replace "npm install && yarn build && pnpm test"
# Expected: bun install && bun build && bun test

# Test case insensitivity
/replace "NPM INSTALL"
# Expected: bun install
```

### Automated Testing

Run the test script:

```bash
./test_bun_replacement.sh
```

## Troubleshooting

### Knowledge Graph Not Loading

```bash
# Check if bun.md exists
ls docs/src/kg/bun.md

# Verify config points to correct path
grep "knowledge_graph_local" context_engineer_config.json

# Check server logs for KG building
# Should see: "Building knowledge graph from docs/src/kg"
```

### Replace Not Working

```bash
# Verify role is selected
# In REPL:
/role list
/role select "Context Engineer"

# Try again:
/replace "npm install"
```

### Case Sensitivity Issues

The Aho-Corasick matcher is case insensitive by default. If you need exact case matching, you would need to modify the matcher configuration in `crates/terraphim_automata/src/matcher.rs`.

## Performance

- **Pattern matching**: ~10-50ms for typical text (uses Aho-Corasick)
- **Knowledge graph loading**: ~100-500ms on startup
- **Memory usage**: ~5-10MB for thesaurus automata

For large files (>1MB), consider processing in chunks.

## Advanced Usage

### Multiple Knowledge Graphs

You can have multiple KG files for different domains:

```bash
docs/src/kg/
├── bun.md              # Package managers
├── typescript.md       # TypeScript synonyms
├── databases.md        # Database synonyms
└── frameworks.md       # Framework synonyms
```

All will be loaded and used for replacement.

### Custom Normalized Terms

In `bun.md`, you can customize the normalized term:

```markdown
# bun install

Fast package installation with Bun.

(synonyms defined in bun-install.md)
```

This would replace all synonyms with "Bun Install" instead of "bun".

## See Also

- [Knowledge Graph System](./knowledge-graph-system.md)
- [Thesaurus Documentation](./thesaurus.md)
- [TUI REPL Commands](../../tui.md)
- [Automata Matcher](../../components/terraphim-automata.md)
