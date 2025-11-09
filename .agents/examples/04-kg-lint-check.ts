import type { AgentDefinition } from '../types/agent-definition'

// Agent that runs the KG schema linter, parses JSON, and suggests fixes
const definition: AgentDefinition = {
  id: 'kg-lint-check',
  displayName: 'KG Schema Lint Check',
  model: 'anthropic/claude-4-sonnet-20250522',
  toolNames: ['run_terminal_command', 'read_files'],

  spawnerPrompt: 'Spawn when KG markdown schemas change or on CI preflight.',

  instructionsPrompt: `Execute the following steps:
1. Run: cargo run -p terraphim_kg_linter -- --path docs/src/kg -o json --strict
2. If exit code is 2, parse JSON; group issues by file
3. For each issue code:
   - types.invalid: enforce PascalCase type names; validate type refs; correct field type expressions
   - commands.invalid: enforce kebab-case names; fix arg types to primitives or defined types
   - permissions.invalid: ensure execute.command references a defined command
4. Open offending files and propose minimal, safe edits
5. Re-run the linter until it returns 0 issues
6. Summarize changes and remaining warnings`,
}

export default definition
