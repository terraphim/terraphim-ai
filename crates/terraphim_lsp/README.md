# terraphim_lsp

Language Server Protocol (LSP) implementation for Terraphim knowledge graphs.

## Overview

`terraphim_lsp` provides editor support for Terraphim knowledge-graph markdown
documents. It analyses open documents against a knowledge-graph thesaurus and
offers:

- **`textDocument/hover`** - Show concept descriptions when hovering over
  thesaurus terms.
- **`textDocument/completion`** - Suggest knowledge-graph terms at the cursor.
- **Diagnostics**, one model per client so nothing is shown twice: clients
  that advertise `textDocument.diagnostic` pull them (`textDocument/diagnostic`
  is advertised only to them) and get `workspace/diagnostic/refresh` when
  results change without an edit (Lab runs and clears, settings and
  thesaurus changes), if they accept it; other clients get
  `textDocument/publishDiagnostics` pushes only. Report a malformed trailing `terraphim-alternatives` annotation block
  (one diagnostic per document) and fade ghosted text (code `ghosted`,
  severity Hint, tag `Unnecessary`; Zed fades it with
  `unnecessary_code_fade`). With the opt-in `unknownTerms` setting, also warn
  on every occurrence of a word that matches no thesaurus term, each at its
  own range.
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

## Thesaurus

Hover, "Replace with X" actions, `[i/n]` inlay hints and completion need a
thesaurus: a JSON file in the format terraphim's thesaurus builders write
(`{"name": "...", "data": {"term": {"id": 1, "nterm": "concept"}}}`). The
path is taken from the first of these that is set (empty values are
ignored):

1. the `thesaurus` setting of the latest `workspace/didChangeConfiguration`
   (bare or under `terraphim`);
2. the `thesaurus` member of `initializationOptions` (bare or under
   `terraphim`); it is kept when a later `didChangeConfiguration` does not
   name a thesaurus, so Zed's `initialization_options` and `settings` can be
   combined;
3. the `--thesaurus <path>` (or `--thesaurus=<path>`) command-line flag;
4. the `TERRAPHIM_THESAURUS` environment variable.

A leading `~/` expands to the home directory; other relative paths are
relative to the server's working directory (editors usually start it at the
workspace root). With none set, the binary starts with an empty thesaurus
and logs a `window/logMessage` warning saying how to configure one.

The file is read and compiled on the blocking thread pool at `initialize`,
and again whenever `didChangeConfiguration` changes the path; every open
document is then republished and inlay hints are refreshed. Removing the
configured setting returns to the `initializationOptions` path, then the
launch path, then the constructor's thesaurus (for programmatic use). A missing or invalid file is logged and shown once as a
`window/showMessage` Warning; the server keeps running with an empty
thesaurus for that path and does not retry it until the path changes (or
the server restarts).

Selecting a thesaurus by Terraphim **role** is not supported yet: resolving
a role needs `terraphim_config` and its persistence stack (device settings,
remote or markdown knowledge graphs, async builders), which this server
does not depend on. Build the role's thesaurus with the Terraphim tools and
point `thesaurus` at the JSON file.

## Settings

Passed as `initializationOptions` or through
`workspace/didChangeConfiguration`, bare or under a `terraphim` key (when
both appear, the keys are merged and the nested value wins per top-level
key):

```json
{
  "thesaurus": "/path/to/thesaurus.json",
  "inlayHints": false,
  "ghostDiagnostics": true,
  "unknownTerms": false,
  "lab": { "actions": [], "trigger": "save" }
}
```

| Setting | Default | Effect |
|---|---|---|
| `thesaurus` | none | path of the thesaurus JSON file; overrides `--thesaurus` and `TERRAPHIM_THESAURUS`; a value from `initializationOptions` survives later settings without one; reloaded when it changes (see [Thesaurus](#thesaurus)) |
| `inlayHints` | `false` | `[i/n]` inlay hints (the capability is always advertised) |
| `ghostDiagnostics` | `true` | faded hints over ghosted text |
| `unknownTerms` | `false` | a Warning (`Unknown term: X`) on every occurrence of a word that is not part of any thesaurus match; off by default because against a real thesaurus nearly every ordinary word is unknown |
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
command with `command`. Switching the trigger from `command` to `save`
computes the configured actions at once; switching `save` to `command`
keeps the marks already shown (the next edit drops them as usual). Command-requested actions and trim previews stay
until their `*.clear` command. Ghost hints and unknown-term warnings follow
`ghostDiagnostics` and `unknownTerms`, and inlay hints are refreshed.

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
terraphim-lsp --thesaurus ~/kg/thesaurus.json
# or
TERRAPHIM_THESAURUS=~/kg/thesaurus.json terraphim-lsp
```

`terraphim-lsp --help` lists the flags. Unrecognised arguments (such as
`--stdio`) are ignored. In Zed, for example:

```json
{
  "lsp": {
    "terraphim-lsp": {
      "initialization_options": {
        "thesaurus": "/absolute/path/to/thesaurus.json",
        "inlayHints": true
      }
    }
  }
}
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

To load the thesaurus from a file the way the binary does, use
`TerraphimLspServer::with_launch_options` (or
`run_stdio_with_launch_options`) with a
`terraphim_lsp::thesaurus::LaunchOptions`; a client `thesaurus` setting
replaces a programmatic thesaurus in either case.

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
