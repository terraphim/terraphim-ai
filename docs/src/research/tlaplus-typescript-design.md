# Design Document: TypeScript Bindings for TLA+

**Status**: Draft
**Author**: Terraphim AI
**Date**: 2026-03-14
**Phase**: 2 (Disciplined Design)
**Research**: [tlaplus-typescript-bindings.md](./tlaplus-typescript-bindings.md)

## Design Overview

A TypeScript library (`@terraphim/tlaplus`) providing native TLA+ tooling: parsing via tree-sitter-tlaplus, typed AST, expression evaluation, formatting, and TLC model checking bridge. Published as an npm package with CLI tool.

## Architecture

```
@terraphim/tlaplus
├── src/
│   ├── parser/          -- tree-sitter wrapper, CST-to-AST transform
│   │   ├── index.ts     -- TlaParser class
│   │   └── cst-to-ast.ts -- CST node visitors -> typed AST
│   ├── ast/             -- TypeScript type definitions for TLA+ AST
│   │   ├── types.ts     -- Node types (Module, OpDef, Expression, etc.)
│   │   ├── visitors.ts  -- Visitor pattern for AST traversal
│   │   └── builders.ts  -- AST node factory functions
│   ├── eval/            -- Expression evaluator
│   │   ├── evaluator.ts -- Evaluate constant TLA+ expressions
│   │   ├── operators.ts -- Built-in operator implementations
│   │   └── values.ts    -- TLA+ value types (sets, functions, records)
│   ├── format/          -- Pretty-printer
│   │   ├── formatter.ts -- AST-to-string formatting
│   │   └── rules.ts     -- Formatting rules (indentation, line width)
│   ├── bridge/          -- TLC model checking bridge
│   │   ├── tlc.ts       -- Spawn Java TLC process, parse output
│   │   └── types.ts     -- TLC result types (states, errors, coverage)
│   ├── cli/             -- Command-line interface
│   │   └── index.ts     -- parse|format|validate|check subcommands
│   └── index.ts         -- Public API exports
├── test/
│   ├── fixtures/        -- TLA+ spec fixtures for testing
│   ├── parser.test.ts
│   ├── ast.test.ts
│   ├── eval.test.ts
│   ├── format.test.ts
│   ├── bridge.test.ts
│   └── cli.test.ts
├── package.json
├── tsconfig.json
├── vitest.config.ts
└── README.md
```

## Issue Decomposition for Symphony

### Issue 1: Scaffold TypeScript project

**Title**: Scaffold TypeScript project with build tooling and CI
**Labels**: `priority/P1-high`, `type/infrastructure`
**Blocks**: #3, #4, #5, #6, #7, #8

**Description**:

Create the initial TypeScript project structure for `@terraphim/tlaplus`.

Requirements:
- `package.json` with name `@terraphim/tlaplus`, type `module`, exports map
- `tsconfig.json` targeting ES2022, NodeNext module resolution, strict mode
- `vitest.config.ts` with coverage reporting
- `.eslintrc.json` with TypeScript eslint
- `.gitignore` for node_modules, dist, coverage
- `src/index.ts` exporting a placeholder version string
- `test/smoke.test.ts` verifying the placeholder export
- Install dev dependencies: `typescript`, `vitest`, `eslint`, `@typescript-eslint/*`, `tsup` (bundler)
- Install runtime dependencies: `tree-sitter`, `@tlaplus/tree-sitter-tlaplus`
- `tsup.config.ts` for building CJS + ESM bundles
- README.md with project description and basic usage

Acceptance criteria:
- `npm install` succeeds
- `npm run build` produces `dist/` with CJS and ESM
- `npm test` runs and passes
- `npm run lint` passes

---

### Issue 2: Create TLA+ AST type definitions

**Title**: Define TypeScript types for TLA+ abstract syntax tree
**Labels**: `priority/P1-high`, `type/feature`
**Blocks**: #3, #4, #5

**Description**:

Define comprehensive TypeScript types for the TLA+ AST in `src/ast/types.ts`.

The type hierarchy should cover:

```typescript
// Top-level
interface TlaModule {
  kind: 'module';
  name: string;
  extends: string[];
  declarations: Declaration[];
  instances: InstanceDecl[];
}

// Declarations
type Declaration = OpDef | VarDecl | ConstDecl | Assumption | Theorem | InstanceDecl;

interface OpDef {
  kind: 'op_def';
  name: string;
  params: FormalParam[];
  body: Expression;
  isRecursive: boolean;
  isLocal: boolean;
}

interface VarDecl { kind: 'var_decl'; names: string[]; }
interface ConstDecl { kind: 'const_decl'; names: string[]; }

// Expressions (discriminated union)
type Expression =
  | LiteralExpr      // numbers, strings, booleans
  | NameExpr         // identifiers
  | OpCallExpr       // operator application
  | SetExpr          // {1, 2, 3}
  | SetFilterExpr    // {x \in S : P(x)}
  | SetMapExpr       // {f(x) : x \in S}
  | FunctionExpr     // [x \in S |-> e]
  | FunctionCallExpr // f[x]
  | RecordExpr       // [a |-> 1, b |-> 2]
  | TupleExpr        // <<1, 2, 3>>
  | IfThenElseExpr
  | LetInExpr
  | QuantifierExpr   // \A, \E
  | ChooseExpr       // CHOOSE
  | CaseExpr
  | ExceptExpr       // [f EXCEPT ![x] = y]
  | PrefixOpExpr     // UNCHANGED, ENABLED, ~, []
  | InfixOpExpr      // /\, \/, =>, +, -, \in, etc.
  | PostfixOpExpr    // '  (prime)
  | ActionExpr       // <<A>>_v, [A]_v
  | TemporalExpr     // []F, <>F, F ~> G
  | SubstitutionExpr;

// PlusCal (optional, in separate file)
interface PlusCalAlgorithm {
  kind: 'pluscal';
  name: string;
  variables: PcalVarDecl[];
  procedures: PcalProcedure[];
  body: PcalBody;
}
```

Also create:
- `src/ast/visitors.ts`: Visitor interface and base walker
- `src/ast/builders.ts`: Factory functions for creating AST nodes

Acceptance criteria:
- All node types are discriminated unions with `kind` field
- Each node type has JSDoc documentation
- Visitor interface covers all node types
- Builder functions create valid nodes
- Unit tests verify type narrowing works correctly

---

### Issue 3: Implement tree-sitter parser wrapper with CST-to-AST transform

**Title**: Implement TLA+ parser using tree-sitter-tlaplus with CST-to-AST transform
**Labels**: `priority/P1-high`, `type/feature`
**Blocked by**: #1, #2

**Description**:

Create `src/parser/index.ts` and `src/parser/cst-to-ast.ts` that wrap tree-sitter-tlaplus and transform the concrete syntax tree into our typed AST.

Reference: Study `js/eval.js` in [Spectacle](https://github.com/will62794/spectacle) for how it traverses the tree-sitter CST.

```typescript
// src/parser/index.ts
export class TlaParser {
  constructor();
  parse(source: string): TlaModule;
  parseExpression(source: string): Expression;
  getErrors(source: string): ParseError[];
}
```

Implementation steps:
1. Initialise tree-sitter with the TLA+ grammar
2. Parse source string to get tree-sitter `Tree`
3. Walk the CST recursively, mapping node types to AST types
4. Handle PlusCal blocks (embedded in comments)
5. Collect parse errors with line/column information

Key tree-sitter node types to handle (from grammar.js):
- `source_file` -> Module
- `operator_definition` -> OpDef
- `variable_declaration` -> VarDecl
- `constant_declaration` -> ConstDecl
- `bounded_quantification` -> QuantifierExpr
- `finite_set_literal` -> SetExpr
- `set_filter` -> SetFilterExpr
- `function_literal` -> FunctionExpr
- `record_literal` -> RecordExpr
- `tuple_literal` -> TupleExpr
- `if_then_else` -> IfThenElseExpr
- `let_in` -> LetInExpr

Test with TLA+ specs from [tlaplus/Examples](https://github.com/tlaplus/Examples).

Acceptance criteria:
- Parses simple specs (DieHard, TwoPhaseCommit) into correct AST
- Reports parse errors with location information
- Handles multi-module specs (EXTENDS)
- Handles PlusCal blocks
- 90%+ test coverage for parser module

---

### Issue 4: Implement basic expression evaluator

**Title**: Implement TLA+ expression evaluator for constant expressions
**Labels**: `priority/P2-medium`, `type/feature`
**Blocked by**: #2, #3

**Description**:

Create `src/eval/evaluator.ts` that evaluates constant TLA+ expressions (no temporal operators, no state variables).

Reference: Study `js/eval.js` in [Spectacle](https://github.com/will62794/spectacle) for evaluation patterns.

Value types (`src/eval/values.ts`):
```typescript
type TlaValue =
  | { kind: 'bool'; value: boolean }
  | { kind: 'int'; value: number }
  | { kind: 'string'; value: string }
  | { kind: 'set'; elements: TlaValue[] }
  | { kind: 'tuple'; elements: TlaValue[] }
  | { kind: 'record'; fields: Map<string, TlaValue> }
  | { kind: 'function'; domain: TlaValue; mapping: Map<string, TlaValue> }
  | { kind: 'model_value'; name: string };
```

Operators to implement (`src/eval/operators.ts`):
- **Logic**: `/\`, `\/`, `~`, `=>`, `<=>`, `TRUE`, `FALSE`
- **Arithmetic**: `+`, `-`, `*`, `\div`, `%`, `..` (range)
- **Comparison**: `=`, `/=`, `<`, `>`, `<=`, `>=`
- **Sets**: `\in`, `\notin`, `\union`, `\intersect`, `\`, `SUBSET`, `UNION`, `Cardinality`
- **Functions**: function application `f[x]`, DOMAIN
- **Sequences**: `Append`, `Head`, `Tail`, `Len`, `SubSeq`
- **Records**: field access `r.field`
- **Quantifiers**: `\A`, `\E` (over finite domains)
- **CHOOSE**: `CHOOSE x \in S : P(x)`
- **IF-THEN-ELSE**
- **LET-IN**
- **CASE**

```typescript
export class TlaEvaluator {
  evaluate(expr: Expression, context?: Map<string, TlaValue>): TlaValue;
}
```

Acceptance criteria:
- Evaluates all listed operators correctly
- Provides clear error messages for unsupported operations
- Context/environment support for variable bindings
- Tests against known TLA+ expression results
- Performance: evaluate 1000 simple expressions in < 100ms

---

### Issue 5: Implement TLA+ formatter (pretty-printer)

**Title**: Implement TLA+ source code formatter
**Labels**: `priority/P2-medium`, `type/feature`
**Blocked by**: #3

**Description**:

Create `src/format/formatter.ts` that takes a parsed TLA+ AST and produces formatted source code.

Reference: [tlaplus-formatter](https://github.com/tlaplus/tlaplus-formatter) for formatting conventions.

```typescript
export interface FormatOptions {
  lineWidth: number;      // default: 80
  indentWidth: number;    // default: 2
  alignConjuncts: boolean; // default: true
}

export class TlaFormatter {
  constructor(options?: Partial<FormatOptions>);
  format(module: TlaModule): string;
  formatExpression(expr: Expression): string;
}
```

Formatting rules:
- Align conjuncts (`/\`) and disjuncts (`\/`) vertically
- Indent nested expressions consistently
- Preserve comments (attach to nearest AST node)
- Handle long lines by breaking at operators
- Follow tlaplus-formatter conventions where sensible
- Idempotent: `format(format(x)) === format(x)`

Acceptance criteria:
- Formats standard TLA+ specs (DieHard, etc.) readably
- Idempotent formatting
- Preserves semantic meaning (parse(format(parse(x))) === parse(x))
- Configurable line width and indent
- Comment preservation
- Tests with before/after fixtures

---

### Issue 6: Implement TLC model checking bridge

**Title**: Implement TLC CLI bridge for model checking from TypeScript
**Labels**: `priority/P2-medium`, `type/feature`
**Blocked by**: #1

**Description**:

Create `src/bridge/tlc.ts` that spawns a Java TLC process and parses its output.

```typescript
export interface TlcOptions {
  javaPath?: string;       // default: 'java'
  tla2toolsPath?: string;  // default: search common locations
  workers?: number;        // default: auto
  maxDepth?: number;
  checkDeadlock?: boolean; // default: true
}

export interface TlcResult {
  success: boolean;
  states: { distinct: number; found: number; left: number };
  errors: TlcError[];
  coverage: TlcCoverage[];
  duration: number;        // milliseconds
  rawOutput: string;
}

export interface TlcError {
  type: 'invariant_violation' | 'deadlock' | 'assertion_failure' | 'parse_error';
  message: string;
  trace?: TlcState[];      // counter-example trace
}

export class TlcBridge {
  constructor(options?: TlcOptions);
  check(specPath: string, configPath?: string): Promise<TlcResult>;
  simulate(specPath: string, depth: number): Promise<TlcResult>;
  isAvailable(): Promise<boolean>;
}
```

Implementation:
1. Locate `tla2tools.jar` (check `TLA2TOOLS_PATH` env, common install locations)
2. Spawn `java -cp tla2tools.jar tlc2.TLC` as child process
3. Parse stdout for state counts, errors, and coverage
4. Parse counter-example traces into structured data
5. Handle timeouts and process cleanup

Acceptance criteria:
- `isAvailable()` correctly detects Java + tla2tools.jar
- Parses TLC output into structured TlcResult
- Handles invariant violations with counter-example traces
- Handles deadlock detection
- Timeout and cancellation support
- Tests with known TLA+ specs that have violations

---

### Issue 7: Create CLI tool

**Title**: Create tlaplus-ts CLI with parse, format, validate, check subcommands
**Labels**: `priority/P2-medium`, `type/feature`
**Blocked by**: #3, #4, #5, #6

**Description**:

Create `src/cli/index.ts` as the CLI entry point.

```
tlaplus-ts parse <file.tla>      -- Parse and print AST as JSON
tlaplus-ts format <file.tla>     -- Format in place (or --stdout)
tlaplus-ts validate <file.tla>   -- Parse + evaluate constant expressions
tlaplus-ts check <file.tla>      -- Run TLC model checking (requires Java)
tlaplus-ts version               -- Print version
```

Implementation:
- Use a lightweight CLI argument parser (no heavy framework)
- Exit codes: 0 success, 1 errors found, 2 usage error
- `--json` flag for machine-readable output
- `--quiet` flag for suppressing non-error output
- Coloured terminal output (with `--no-color` flag)

Add `bin` field to `package.json`:
```json
{
  "bin": {
    "tlaplus-ts": "./dist/cli/index.js"
  }
}
```

Acceptance criteria:
- All four subcommands work correctly
- `--help` shows usage for each subcommand
- Exit codes are correct
- JSON output mode works
- Works with stdin pipe: `cat spec.tla | tlaplus-ts parse`

---

### Issue 8: Documentation, examples, and npm publish preparation

**Title**: Add documentation, examples, and prepare for npm publishing
**Labels**: `priority/P3-low`, `type/documentation`
**Blocked by**: #7

**Description**:

Finalise the package for npm publishing.

README.md sections:
- Installation (`npm install @terraphim/tlaplus`)
- Quick start (parse a spec, evaluate an expression, format, run TLC)
- API reference (TlaParser, TlaEvaluator, TlaFormatter, TlcBridge)
- CLI usage
- Browser usage (WASM notes)
- Contributing

Examples:
- `examples/parse-spec.ts` -- Parse and inspect a TLA+ spec
- `examples/evaluate.ts` -- Evaluate TLA+ expressions
- `examples/format.ts` -- Format a spec
- `examples/model-check.ts` -- Run TLC via bridge
- `examples/specs/DieHard.tla` -- Classic TLA+ example

Package preparation:
- Verify `package.json` exports map is correct
- Verify TypeScript declaration files are generated
- Add `files` field to package.json (include only dist/, README, LICENSE)
- Add LICENSE file (Apache-2.0)
- Verify `npm pack` produces correct tarball

Acceptance criteria:
- README is comprehensive and accurate
- All examples run successfully
- `npm pack` produces correct package
- TypeScript declaration files are included
- All tests pass
- Package size is reasonable (< 2MB excluding WASM)

## Dependency Graph

```
Issue 1 (scaffold) ──┬──> Issue 3 (parser) ──┬──> Issue 4 (evaluator) ──┐
                     │                        │                          │
Issue 2 (AST types) ─┤                        ├──> Issue 5 (formatter) ──┤
                     │                        │                          │
                     └──> Issue 6 (TLC bridge)┘──> Issue 7 (CLI) ────────┤
                                                                         │
                                                                         └──> Issue 8 (docs)
```

**Symphony dispatch waves**:
- Wave A: Issue 1, Issue 2 (parallel, no dependencies)
- Wave B: Issue 3, Issue 6 (parallel, depend on A)
- Wave C: Issue 4, Issue 5 (parallel, depend on B)
- Wave D: Issue 7 (depends on C)
- Wave E: Issue 8 (depends on D)
