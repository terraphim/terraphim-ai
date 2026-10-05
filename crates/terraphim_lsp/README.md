# terraphim_lsp

Language Server Protocol (LSP) implementation for Terraphim knowledge graphs.

## Overview

`terraphim_lsp` provides editor support for Terraphim knowledge-graph markdown
documents. It analyses open documents against a knowledge-graph thesaurus and
offers:

- **`textDocument/hover`** - Show concept descriptions when hovering over
  thesaurus terms.
- **`textDocument/completion`** - Suggest knowledge-graph terms at the cursor.
- **`textDocument/diagnostic`** (pulled, and pushed as `publishDiagnostics`)
  - Warn about terms in the document that are not present in the thesaurus,
  report a malformed trailing `terraphim-alternatives` annotation block (one
  diagnostic per document), and fade ghosted text (code `ghosted`, severity
  Hint, tag `Unnecessary`; Zed fades it with `unnecessary_code_fade`).
- **Lab marks** - terraphim-editor's Lab engine (`terraphim_lab`): one
  diagnostic code per mark kind (`lab-typo`, `lab-punctuation`,
  `lab-weak-sentence`, `lab-long-sentence`, `lab-convoluted-sentence`,
  `lab-off-tone`, `lab-hedge`, `lab-filler`), Information for typos and
  punctuation, Hint otherwise, message = the engine's reason. Off by default.
- **Trim preview** - the spans a trim level would cut, as `trim-candidate`
  Hints tagged `Unnecessary`.
- **`textDocument/codeAction`** - On a KG term, one "Replace with X" action
  (`refactor.rewrite`) per other synonym of its concept. The current form is
  excluded, the original capitalisation is kept, and a preceding `a`/`an` is
  fixed in the same `WorkspaceEdit`. On a Lab typo or punctuation mark,
  "Apply fix: X" (`quickfix`) replacing exactly the marked range.
- **`textDocument/inlayHint`** - `[i/n]` after each KG term with
  alternatives: the current form is synonym `i` of `n`. Off by default.
- **`workspace/executeCommand`** - see below.

Edits are versioned (`documentChanges` with the document version) for
clients that accept them, so a stale edit is rejected rather than applied.

## Settings

Passed as `initializationOptions` or through
`workspace/didChangeConfiguration`, bare or under a `terraphim` key:

```json
{
  "inlayHints": false,
  "ghostDiagnostics": true,
  "lab": { "actions": [], "trigger": "save" }
}
```

| Setting | Default | Effect |
|---|---|---|
| `inlayHints` | `false` | `[i/n]` inlay hints (the capability is always advertised) |
| `ghostDiagnostics` | `true` | faded hints over ghosted text |
| `lab.actions` | `[]` | Lab actions run automatically: `typos_and_punctuation`, `weakest_sentences`, `long_sentences`, `convoluted_sentences`, `off_tone`, `hedges_and_filler` |
| `lab.trigger` | `"save"` | `save`: Lab marks and the trim preview are recomputed on open and save; `command`: only by the commands |

Lab marks are never computed per keystroke: each run parses the whole
Markdown document, and marks flickering while typing would be noise. An edit
drops the current Lab marks and trim hints (their ranges would be stale)
until the next save or command. Lab runs use the blocking thread pool and
hold no document lock; results computed for a version the client has
already replaced are dropped, and a command whose run was overtaken by an
edit fails with `ContentModified`. A per-document generation (bumped by
edits, Lab commands, clears and settings changes) keeps runs that finish
out of order from overwriting newer Lab or trim state; a superseded
command recomputes from the current state.

On `didChangeConfiguration`, marks of actions that are no longer configured
(and were not requested by command) disappear at once; newly configured
actions are computed immediately with the `save` trigger and on the next
command with `command`. Command-requested actions and trim previews stay
until their `*.clear` command. Ghost hints follow `ghostDiagnostics` and
inlay hints are refreshed.

## Commands

Each takes one JSON object. `version`, when given, must match the server's
version of the document, or the request fails with `ContentModified`.

| Command | Arguments | Result |
|---|---|---|
| `terraphim.alternative.add` | `uri`, `range`, `text`, `kind?` (`word`/`sentence`/`paragraph`), `version?` | a `WorkspaceEdit` that rewrites only the trailing annotation block, written by terraphim-editor's own writer; the client applies it |
| `terraphim.lab.mark` | `uri`, `action`, `version?` | `{ action, marks }`; the action sticks to the document until cleared |
| `terraphim.lab.clear` | `uri` | `null` |
| `terraphim.trim.preview` | `uri`, `level` (`original`/`slight`/`tighten`/`sharper`/`half`), `version?` | `{ level, candidates, status }`, `status` being the Lab status card (`61 → 55 words · −10%`) |
| `terraphim.trim.clear` | `uri` | `null` |

`terraphim.alternative.add` refuses (`InvalidParams`) a malformed block, a
range outside the body, an alternative equal to the current text or already
present, a range overlapping another span, and a block whose stored anchors
can no longer be placed (rewriting it would drop them).

The analysis lives in [`terraphim_lsp_core`](../terraphim_lsp_core), a pure
crate with no LSP or async dependencies that also builds for
`wasm32-unknown-unknown`; terraphim-editor runs the same code in-process.
It is re-exported as `terraphim_lsp::core`. The trailing annotation block is
never analysed. Positions are converted with UTF-16 columns, so multi-byte
and astral text map to the right characters.

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
terraphim_lsp = { path = "../terraphim_lsp" }
```

Or build the standalone binary:

```bash
cargo build -p terraphim_lsp --bin terraphim-lsp
```

## Usage

### Standalone binary over stdio

The `terraphim-lsp` binary speaks LSP over standard input/output and can be
configured in any LSP-compatible editor:

```bash
terraphim-lsp
```

### Programmatic use

```rust
use tower_lsp::LspService;
use terraphim_lsp::TerraphimLspServer;
use terraphim_types::{NormalizedTerm, NormalizedTermValue, Thesaurus};

#[tokio::main]
async fn main() {
    let mut thesaurus = Thesaurus::new("programming".to_string());
    thesaurus.insert(
        NormalizedTermValue::from("rust"),
        NormalizedTerm::with_auto_id(NormalizedTermValue::from("rust programming language")),
    );

    let (service, socket) =
        LspService::new(move |client| TerraphimLspServer::new(client, thesaurus.clone()));

    // service implements tower_lsp::LanguageServer; wire it to stdin/stdout or
    // a test harness.
}
```

## Architecture

```
Editor LSP request
        │
        ▼
  TerraphimLspServer
        │
        ├── hover ──────► kg_analysis ──────► Hover response
        ├── completion ─► completion.rs ────► CompletionItem[]
        ├── diagnostic ─► diagnostics.rs + Lab/trim ─► Diagnostic[]
        ├── codeAction ─► KgEngine::alternatives_at + Lab fixes ─► CodeAction[]
        ├── inlayHint ──► KgEngine::synonym_positions ─► InlayHint[]
        └── executeCommand ─► add_alternative / lab_findings / trim_preview
                │
                ▼
        terraphim_lsp_core (KgEngine: CompiledMatcher + ConceptIndex)
```

Open documents are tracked in memory. On every `did_open` and `did_change` the
document is re-analysed and diagnostics are published to the client; Lab
results are recomputed on open, save and command only.

## Testing

```bash
# Run unit and integration tests
cargo test -p terraphim_lsp

# Run linter
cargo clippy -p terraphim_lsp --all-targets
```

## License

Apache-2.0
