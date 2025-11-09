# Terraphim KG Schema Linter

This project includes a Rust CLI linter that validates markdown-based schemas for Knowledge Graph (KG) extensions used by AI agents.

The linter recognizes structured code fences inside markdown files for:
- Commands (\`kg-commands\` or \`kg:commands\`)
- Types (\`kg-types\` or \`kg:types\`)
- Permissions (\`kg-permissions\` or \`kg:permissions\`)

It also builds a thesaurus from \`synonyms::\` lines using \`terraphim_automata\` (Logseq builder) and verifies the autocomplete index to ensure KG search readiness (graph-embeddings integration).

## Install & Run

- Build: \`cargo build -p terraphim_kg_linter\`
- Lint default KG path: \`cargo run -p terraphim_kg_linter -- --path docs/src/kg\`
- JSON output: \`cargo run -p terraphim_kg_linter -- --path docs/src/kg -o json\`
- Strict mode (non-zero exit on issues): \`cargo run -p terraphim_kg_linter -- --strict\`

## Schema Blocks

### Commands

\`\`\`kg-commands
# Single command (YAML object) or list (- ...)
name: search
description: Search documents
args:
  - name: query
    type: string
    required: true
  - name: limit
    type: integer
    required: false
    default: 10
permissions:
  - can: read
    on: documents
\`\`\`

Rules:
- \`name\` must be kebab-case.
- \`args[].type\` references primitive (\`string|integer|number|boolean|object|array|ulid|url|path\`) or a user-defined type from \`kg-types\`.

### Types

\`\`\`kg-types
Document:
  id: string
  title: string
  body: string
  tags: string[]
  rank?: integer
\`\`\`

Rules:
- Type names must be PascalCase.
- Field types may use \`[]\` for arrays and optional \`?\` suffix (on the field name).
- Custom types can reference other types by name.

### Permissions

\`\`\`kg-permissions
roles:
  - name: agent
    allow:
      - action: execute
        command: search
      - action: read
        resource: documents
    deny:
      - action: delete
        resource: documents
\`\`\`

Rules:
- \`execute\` rules must reference an existing command name.
- Linter verifies command/permission linkage and basic structure.

## KG Integration

- \`synonyms::\` lines in \`docs/src/kg/**/*.md\` are indexed by \`terraphim_automata::Logseq::build\`.
- The linter builds a \`Thesaurus\` and an FST autocomplete index to validate readiness for graph-based search (embedding-like autocomplete) without network access.

## Example

Create \`docs/src/kg/schema-example.md\`:

\`\`\`markdown
# Example Schema

\`\`\`kg-types
Document:
  id: string
  title: string
  tags: string[]
\`\`\`

\`\`\`kg-commands
name: search
args:
  - name: query
    type: string
\`\`\`

\`\`\`kg-permissions
roles:
  - name: agent
    allow:
      - action: execute
        command: search
\`\`\`
\`\`\`

Then run:

\`\`\`
cargo run -p terraphim_kg_linter -- --path docs/src/kg --strict
\`\`\`

Expected output: \`No issues found\` and non-zero exit only when issues exist.

## For AI Agents

- Preferred command (machine-readable):
  - `cargo run -p terraphim_kg_linter -- --path docs/src/kg -o json --strict`
- Behavior:
  - Exit code 0 when no issues; 2 when issues are present
  - JSON schema: `{ scanned_files, issues: [{ path, severity, code, message }], stats }`
- Typical workflow:
  - Run linter → parse JSON → group by `code` → suggest minimal edits in offending file(s)
  - Apply fixes (e.g., kebab-case command names; PascalCase types; fix unknown types)
  - Re-run until zero issues

Issue codes and quick actions:
- `types.invalid` → fix type names (PascalCase), field names, or referenced type existence
- `types.duplicate` → deduplicate type definitions across files
- `commands.invalid` → enforce kebab-case command names; ensure args reference valid types
- `permissions.invalid` → ensure `execute` references an existing command; add or correct command

Programmatic example:

1) Execute the JSON command.
2) Parse and map issues to file edits.
3) Re-run the same command to verify a clean state.
