#!/usr/bin/env bash
# Setup script for tlaplus-ts Gitea repository and issues
# Run on bigbox after sourcing credentials:
#   source ~/op_env  # or export GITEA_TOKEN=...
#   bash scripts/setup-tlaplus-ts-gitea.sh

set -euo pipefail

GITEA_URL="${GITEA_URL:-https://git.terraphim.cloud}"
OWNER="terraphim"
REPO="tlaplus-ts"

if [ -z "${GITEA_TOKEN:-}" ]; then
  echo "ERROR: GITEA_TOKEN not set. Run: export GITEA_TOKEN=\$(op read 'op://TerraphimPlatform/gitea-test-token/credential')"
  exit 1
fi

API="${GITEA_URL}/api/v1"
AUTH="Authorization: token ${GITEA_TOKEN}"
CT="Content-Type: application/json"

echo "=== Creating repository ${OWNER}/${REPO} ==="
curl -s -X POST "${API}/orgs/${OWNER}/repos" \
  -H "${AUTH}" -H "${CT}" \
  -d "{
    \"name\": \"${REPO}\",
    \"description\": \"TypeScript bindings for TLA+ formal specifications\",
    \"private\": false,
    \"auto_init\": true,
    \"default_branch\": \"main\"
  }" | jq -r '.full_name // .message'

echo ""
echo "=== Creating labels ==="
for label_data in \
  '{"name":"priority/P1-high","color":"#e11d48"}' \
  '{"name":"priority/P2-medium","color":"#f59e0b"}' \
  '{"name":"priority/P3-low","color":"#22c55e"}' \
  '{"name":"type/infrastructure","color":"#6366f1"}' \
  '{"name":"type/feature","color":"#3b82f6"}' \
  '{"name":"type/documentation","color":"#8b5cf6"}' \
  '{"name":"status/todo","color":"#94a3b8"}' \
  '{"name":"status/in-progress","color":"#f97316"}'; do
  name=$(echo "$label_data" | jq -r '.name')
  curl -s -X POST "${API}/repos/${OWNER}/${REPO}/labels" \
    -H "${AUTH}" -H "${CT}" \
    -d "$label_data" | jq -r ".name // .message" || true
done

echo ""
echo "=== Creating issues ==="

# Issue 1: Scaffold
ISSUE1=$(curl -s -X POST "${API}/repos/${OWNER}/${REPO}/issues" \
  -H "${AUTH}" -H "${CT}" \
  -d '{
    "title": "Scaffold TypeScript project with build tooling and CI",
    "body": "Create the initial TypeScript project structure for `@terraphim/tlaplus`.\n\n## Requirements\n\n- `package.json` with name `@terraphim/tlaplus`, type `module`, exports map\n- `tsconfig.json` targeting ES2022, NodeNext module resolution, strict mode\n- `vitest.config.ts` with coverage reporting\n- `.eslintrc.json` with TypeScript eslint\n- `.gitignore` for node_modules, dist, coverage\n- `src/index.ts` exporting a placeholder version string\n- `test/smoke.test.ts` verifying the placeholder export\n- Install dev deps: `typescript`, `vitest`, `eslint`, `@typescript-eslint/*`, `tsup`\n- Install runtime deps: `tree-sitter`, `@tlaplus/tree-sitter-tlaplus`\n- `tsup.config.ts` for building CJS + ESM bundles\n- `README.md` with project description\n\n## Acceptance Criteria\n\n- [x] `npm install` succeeds\n- [x] `npm run build` produces `dist/` with CJS and ESM\n- [x] `npm test` runs and passes\n- [x] `npm run lint` passes",
    "labels": [1, 5]
  }' | jq -r '.number')
echo "Created issue #${ISSUE1}: Scaffold"

# Issue 2: AST types
ISSUE2=$(curl -s -X POST "${API}/repos/${OWNER}/${REPO}/issues" \
  -H "${AUTH}" -H "${CT}" \
  -d '{
    "title": "Define TypeScript types for TLA+ abstract syntax tree",
    "body": "Define comprehensive TypeScript types for the TLA+ AST.\n\n## Files to Create\n\n- `src/ast/types.ts` -- All node types as discriminated unions with `kind` field\n- `src/ast/visitors.ts` -- Visitor interface and base walker\n- `src/ast/builders.ts` -- Factory functions for creating AST nodes\n\n## Type Hierarchy\n\n### Top-level\n- `TlaModule` -- name, extends, declarations, instances\n\n### Declarations\n- `OpDef` -- operator definitions with params and body\n- `VarDecl` -- variable declarations\n- `ConstDecl` -- constant declarations\n- `Assumption`, `Theorem`, `InstanceDecl`\n\n### Expressions (discriminated union)\n- `LiteralExpr` (numbers, strings, booleans)\n- `NameExpr` (identifiers)\n- `OpCallExpr` (operator application)\n- `SetExpr`, `SetFilterExpr`, `SetMapExpr`\n- `FunctionExpr`, `FunctionCallExpr`\n- `RecordExpr`, `TupleExpr`\n- `IfThenElseExpr`, `LetInExpr`\n- `QuantifierExpr` (forall, exists)\n- `ChooseExpr`, `CaseExpr`, `ExceptExpr`\n- `PrefixOpExpr`, `InfixOpExpr`, `PostfixOpExpr`\n- `ActionExpr`, `TemporalExpr`\n\n### PlusCal\n- `PlusCalAlgorithm` with variables, procedures, body\n\n## Acceptance Criteria\n\n- [x] All node types are discriminated unions with `kind` field\n- [x] Each node type has JSDoc documentation\n- [x] Visitor interface covers all node types\n- [x] Builder functions create valid nodes\n- [x] Unit tests verify type narrowing works correctly",
    "labels": [1, 4]
  }' | jq -r '.number')
echo "Created issue #${ISSUE2}: AST types"

# Issue 3: Parser
ISSUE3=$(curl -s -X POST "${API}/repos/${OWNER}/${REPO}/issues" \
  -H "${AUTH}" -H "${CT}" \
  -d '{
    "title": "Implement tree-sitter parser wrapper with CST-to-AST transform",
    "body": "Create `src/parser/index.ts` and `src/parser/cst-to-ast.ts` that wrap tree-sitter-tlaplus and transform the concrete syntax tree into our typed AST.\n\n## API\n\n```typescript\nexport class TlaParser {\n  constructor();\n  parse(source: string): TlaModule;\n  parseExpression(source: string): Expression;\n  getErrors(source: string): ParseError[];\n}\n```\n\n## Implementation\n\n1. Initialise tree-sitter with the TLA+ grammar\n2. Parse source string to get tree-sitter Tree\n3. Walk the CST recursively, mapping node types to AST types\n4. Handle PlusCal blocks (embedded in comments)\n5. Collect parse errors with line/column information\n\n## Reference\n\nStudy `js/eval.js` in Spectacle (github.com/will62794/spectacle) for CST traversal patterns.\n\n## Key tree-sitter node types\n\n- `source_file` -> Module\n- `operator_definition` -> OpDef\n- `variable_declaration` -> VarDecl\n- `constant_declaration` -> ConstDecl\n- `bounded_quantification` -> QuantifierExpr\n- `finite_set_literal` -> SetExpr\n- `function_literal` -> FunctionExpr\n- `record_literal` -> RecordExpr\n- `tuple_literal` -> TupleExpr\n- `if_then_else` -> IfThenElseExpr\n- `let_in` -> LetInExpr\n\nTest with TLA+ specs from tlaplus/Examples (DieHard, TwoPhaseCommit).\n\n## Acceptance Criteria\n\n- [x] Parses simple specs into correct AST\n- [x] Reports parse errors with location info\n- [x] Handles multi-module specs (EXTENDS)\n- [x] Handles PlusCal blocks\n- [x] 90%+ test coverage for parser module",
    "labels": [1, 4]
  }' | jq -r '.number')
echo "Created issue #${ISSUE3}: Parser"

# Issue 4: Evaluator
ISSUE4=$(curl -s -X POST "${API}/repos/${OWNER}/${REPO}/issues" \
  -H "${AUTH}" -H "${CT}" \
  -d '{
    "title": "Implement basic TLA+ expression evaluator",
    "body": "Create `src/eval/evaluator.ts` that evaluates constant TLA+ expressions.\n\n## Value Types\n\n```typescript\ntype TlaValue =\n  | { kind: \"bool\"; value: boolean }\n  | { kind: \"int\"; value: number }\n  | { kind: \"string\"; value: string }\n  | { kind: \"set\"; elements: TlaValue[] }\n  | { kind: \"tuple\"; elements: TlaValue[] }\n  | { kind: \"record\"; fields: Map<string, TlaValue> }\n  | { kind: \"function\"; domain: TlaValue; mapping: Map<string, TlaValue> }\n  | { kind: \"model_value\"; name: string };\n```\n\n## Operators to Implement\n\n- Logic: `/\\`, `\\/`, `~`, `=>`, `<=>`\n- Arithmetic: `+`, `-`, `*`, `\\div`, `%`, `..` (range)\n- Comparison: `=`, `/=`, `<`, `>`, `<=`, `>=`\n- Sets: `\\in`, `\\notin`, `\\union`, `\\intersect`, `\\`, `SUBSET`, `UNION`, `Cardinality`\n- Functions: application `f[x]`, `DOMAIN`\n- Sequences: `Append`, `Head`, `Tail`, `Len`, `SubSeq`\n- Records: field access `r.field`\n- Quantifiers: `\\A`, `\\E` (over finite domains)\n- `CHOOSE`, `IF-THEN-ELSE`, `LET-IN`, `CASE`\n\n## Reference\n\nStudy `js/eval.js` in Spectacle (github.com/will62794/spectacle).\n\n## Acceptance Criteria\n\n- [x] Evaluates all listed operators correctly\n- [x] Clear error messages for unsupported operations\n- [x] Context/environment support for variable bindings\n- [x] Tests against known TLA+ expression results\n- [x] Performance: 1000 simple expressions in < 100ms",
    "labels": [2, 4]
  }' | jq -r '.number')
echo "Created issue #${ISSUE4}: Evaluator"

# Issue 5: Formatter
ISSUE5=$(curl -s -X POST "${API}/repos/${OWNER}/${REPO}/issues" \
  -H "${AUTH}" -H "${CT}" \
  -d '{
    "title": "Implement TLA+ source code formatter",
    "body": "Create `src/format/formatter.ts` that takes a parsed TLA+ AST and produces formatted source code.\n\n## API\n\n```typescript\nexport interface FormatOptions {\n  lineWidth: number;       // default: 80\n  indentWidth: number;     // default: 2\n  alignConjuncts: boolean; // default: true\n}\n\nexport class TlaFormatter {\n  constructor(options?: Partial<FormatOptions>);\n  format(module: TlaModule): string;\n  formatExpression(expr: Expression): string;\n}\n```\n\n## Formatting Rules\n\n- Align conjuncts and disjuncts vertically\n- Indent nested expressions consistently\n- Preserve comments (attach to nearest AST node)\n- Handle long lines by breaking at operators\n- Idempotent: `format(format(x)) === format(x)`\n\nReference: tlaplus-formatter (github.com/tlaplus/tlaplus-formatter) for conventions.\n\n## Acceptance Criteria\n\n- [x] Formats standard TLA+ specs readably\n- [x] Idempotent formatting\n- [x] Preserves semantic meaning\n- [x] Configurable line width and indent\n- [x] Comment preservation\n- [x] Tests with before/after fixtures",
    "labels": [2, 4]
  }' | jq -r '.number')
echo "Created issue #${ISSUE5}: Formatter"

# Issue 6: TLC Bridge
ISSUE6=$(curl -s -X POST "${API}/repos/${OWNER}/${REPO}/issues" \
  -H "${AUTH}" -H "${CT}" \
  -d '{
    "title": "Implement TLC CLI bridge for model checking from TypeScript",
    "body": "Create `src/bridge/tlc.ts` that spawns a Java TLC process and parses its output.\n\n## API\n\n```typescript\nexport interface TlcOptions {\n  javaPath?: string;\n  tla2toolsPath?: string;\n  workers?: number;\n  maxDepth?: number;\n  checkDeadlock?: boolean;\n}\n\nexport interface TlcResult {\n  success: boolean;\n  states: { distinct: number; found: number; left: number };\n  errors: TlcError[];\n  coverage: TlcCoverage[];\n  duration: number;\n  rawOutput: string;\n}\n\nexport class TlcBridge {\n  constructor(options?: TlcOptions);\n  check(specPath: string, configPath?: string): Promise<TlcResult>;\n  simulate(specPath: string, depth: number): Promise<TlcResult>;\n  isAvailable(): Promise<boolean>;\n}\n```\n\n## Implementation\n\n1. Locate `tla2tools.jar` (check TLA2TOOLS_PATH env, common install locations)\n2. Spawn `java -cp tla2tools.jar tlc2.TLC` as child process\n3. Parse stdout for state counts, errors, and coverage\n4. Parse counter-example traces into structured data\n5. Handle timeouts and process cleanup\n\n## Acceptance Criteria\n\n- [x] `isAvailable()` correctly detects Java + tla2tools.jar\n- [x] Parses TLC output into structured TlcResult\n- [x] Handles invariant violations with traces\n- [x] Timeout and cancellation support\n- [x] Tests with known TLA+ specs",
    "labels": [2, 4]
  }' | jq -r '.number')
echo "Created issue #${ISSUE6}: TLC Bridge"

# Issue 7: CLI
ISSUE7=$(curl -s -X POST "${API}/repos/${OWNER}/${REPO}/issues" \
  -H "${AUTH}" -H "${CT}" \
  -d '{
    "title": "Create tlaplus-ts CLI with parse, format, validate, check subcommands",
    "body": "Create `src/cli/index.ts` as the CLI entry point.\n\n## Subcommands\n\n```\ntlaplus-ts parse <file.tla>      -- Parse and print AST as JSON\ntlaplus-ts format <file.tla>     -- Format in place (or --stdout)\ntlaplus-ts validate <file.tla>   -- Parse + evaluate constant expressions\ntlaplus-ts check <file.tla>      -- Run TLC model checking (requires Java)\ntlaplus-ts version               -- Print version\n```\n\n## Implementation\n\n- Lightweight CLI argument parser (no heavy framework)\n- Exit codes: 0 success, 1 errors found, 2 usage error\n- `--json` flag for machine-readable output\n- `--quiet` flag for suppressing non-error output\n- Coloured terminal output (with `--no-color` flag)\n- `bin` field in package.json pointing to dist/cli/index.js\n- Support stdin pipe: `cat spec.tla | tlaplus-ts parse`\n\n## Acceptance Criteria\n\n- [x] All four subcommands work correctly\n- [x] `--help` shows usage for each subcommand\n- [x] Exit codes are correct\n- [x] JSON output mode works\n- [x] Works with stdin pipe",
    "labels": [2, 4]
  }' | jq -r '.number')
echo "Created issue #${ISSUE7}: CLI"

# Issue 8: Documentation
ISSUE8=$(curl -s -X POST "${API}/repos/${OWNER}/${REPO}/issues" \
  -H "${AUTH}" -H "${CT}" \
  -d '{
    "title": "Add documentation, examples, and prepare for npm publishing",
    "body": "Finalise the package for npm publishing.\n\n## README.md Sections\n\n- Installation\n- Quick start (parse, evaluate, format, check)\n- API reference (TlaParser, TlaEvaluator, TlaFormatter, TlcBridge)\n- CLI usage\n- Browser usage (WASM notes)\n- Contributing\n\n## Examples\n\n- `examples/parse-spec.ts`\n- `examples/evaluate.ts`\n- `examples/format.ts`\n- `examples/model-check.ts`\n- `examples/specs/DieHard.tla`\n\n## Package Preparation\n\n- Verify package.json exports map\n- Verify TypeScript declaration files generated\n- Add `files` field (dist/, README, LICENSE)\n- Add LICENSE file (Apache-2.0)\n- Verify `npm pack` produces correct tarball\n\n## Acceptance Criteria\n\n- [x] README is comprehensive and accurate\n- [x] All examples run successfully\n- [x] `npm pack` produces correct package\n- [x] TypeScript declarations included\n- [x] All tests pass\n- [x] Package size < 2MB",
    "labels": [3, 6]
  }' | jq -r '.number')
echo "Created issue #${ISSUE8}: Documentation"

echo ""
echo "=== Setting up issue dependencies ==="
# Issue 3 blocked by 1, 2
for blocker in ${ISSUE1} ${ISSUE2}; do
  curl -s -X POST "${API}/repos/${OWNER}/${REPO}/issues/${ISSUE3}/blocks" \
    -H "${AUTH}" -H "${CT}" \
    -d "{\"issue_index\": ${blocker}}" | jq -r '.message // "ok"' || true
done

# Issue 4 blocked by 2, 3
for blocker in ${ISSUE2} ${ISSUE3}; do
  curl -s -X POST "${API}/repos/${OWNER}/${REPO}/issues/${ISSUE4}/blocks" \
    -H "${AUTH}" -H "${CT}" \
    -d "{\"issue_index\": ${blocker}}" | jq -r '.message // "ok"' || true
done

# Issue 5 blocked by 3
curl -s -X POST "${API}/repos/${OWNER}/${REPO}/issues/${ISSUE5}/blocks" \
  -H "${AUTH}" -H "${CT}" \
  -d "{\"issue_index\": ${ISSUE3}}" | jq -r '.message // "ok"' || true

# Issue 6 blocked by 1
curl -s -X POST "${API}/repos/${OWNER}/${REPO}/issues/${ISSUE6}/blocks" \
  -H "${AUTH}" -H "${CT}" \
  -d "{\"issue_index\": ${ISSUE1}}" | jq -r '.message // "ok"' || true

# Issue 7 blocked by 3, 4, 5, 6
for blocker in ${ISSUE3} ${ISSUE4} ${ISSUE5} ${ISSUE6}; do
  curl -s -X POST "${API}/repos/${OWNER}/${REPO}/issues/${ISSUE7}/blocks" \
    -H "${AUTH}" -H "${CT}" \
    -d "{\"issue_index\": ${blocker}}" | jq -r '.message // "ok"' || true
done

# Issue 8 blocked by 7
curl -s -X POST "${API}/repos/${OWNER}/${REPO}/issues/${ISSUE8}/blocks" \
  -H "${AUTH}" -H "${CT}" \
  -d "{\"issue_index\": ${ISSUE7}}" | jq -r '.message // "ok"' || true

echo ""
echo "=== Setup complete ==="
echo "Repository: ${GITEA_URL}/${OWNER}/${REPO}"
echo "Issues: #${ISSUE1}, #${ISSUE2}, #${ISSUE3}, #${ISSUE4}, #${ISSUE5}, #${ISSUE6}, #${ISSUE7}, #${ISSUE8}"
echo ""
echo "Next steps:"
echo "  1. Copy WORKFLOW to bigbox: scp crates/terraphim_symphony/examples/WORKFLOW-tlaplus-ts.md bigbox:~/WORKFLOW-tlaplus-ts.md"
echo "  2. SSH to bigbox: ssh bigbox"
echo "  3. Run Symphony: cd ~/terraphim-ai/crates/terraphim_symphony && cargo run --release --bin symphony -- ~/WORKFLOW-tlaplus-ts.md"
