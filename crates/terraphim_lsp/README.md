# terraphim_lsp

Language Server Protocol (LSP) implementation for Terraphim knowledge graphs.

## Overview

`terraphim_lsp` provides editor support for Terraphim knowledge-graph markdown
documents. It analyses open documents against a knowledge-graph thesaurus and
offers:

- **`textDocument/hover`** - On a thesaurus term, its concept and
  description. On a term or a span stored in the annotation block, the
  alternatives with their provenance (R-4.4): the stored ones in block order,
  labelled `original`, `human`, `ai` or `ai, <model>`, then the KG synonyms
  not already stored, labelled `kg`; the active one is marked `active`. A
  stored sentence or paragraph span with no KG term shows its stored list.
- **`textDocument/completion`** - Suggest knowledge-graph terms at the cursor.
- **Diagnostics**, one model per client so nothing is shown twice: clients
  that advertise `textDocument.diagnostic` pull them (`textDocument/diagnostic`
  is advertised only to them) and get `workspace/diagnostic/refresh` when
  results change without an edit (Lab runs and clears, settings and
  thesaurus changes), only if they advertise it (the spec key
  `workspace.diagnostics.refreshSupport`, read from the raw `initialize`
  because lsp-types drops it; see `handshake.rs`); other clients get
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
  fixed in the same `WorkspaceEdit`. None is offered when its edit (the term
  or the article) would change text inside a stored span or ghost, which
  would leave that anchor stale; the hover still lists the synonyms. On a
  span stored in the annotation block, "Replace with X (provenance)"
  (`refactor.terraphim.alternative`, R-2.2) for each inactive stored
  alternative: a versioned `WorkspaceEdit` that swaps the body text (with the
  `a`/`an` fix-up) and records the new active alternative in the block,
  exactly as terraphim-editor does. A malformed block, or one with an anchor
  that cannot be placed, offers none. On a Lab typo or punctuation mark,
  "Apply fix: X" (`quickfix`) replacing exactly the marked range. On any
  open document, command-only trim actions (`refactor.terraphim.trim`, no
  `edit`) for clients that cannot add palette commands, such as Zed:
  "Trim: Original", "Trim: Slight trim ~10%", "Trim: Tighten more ~20%",
  "Trim: Even sharper ~30%" and "Trim: Cut in half ~50%" run
  `terraphim.trim.preview` with `{uri, level, version}` ("Original" ends the
  preview); "Trim: Make the cuts" runs `terraphim.trim.make_cuts` and "Trim:
  Walk through" runs `terraphim.trim.next` from the cursor; with the cursor
  inside a faded cut, "Keep: «excerpt»" runs `terraphim.trim.keep` (innermost
  cut first). The menu is stateless (Zed caches code actions until the cursor
  moves): every level and review action is always listed and reads the trim
  state when it runs, so "Make the cuts" is a command, not a cached edit that
  would ignore a later Keep.
  On a non-empty selection that is not wholly ghosted, "Ghost it"; with the
  cursor in a ghost or a selection overlapping one, "Revive"
  (`refactor.terraphim.ghost`, R-5.1 and R-5.2). Each carries a versioned
  `WorkspaceEdit` that rewrites only the trailing annotation block, creating
  it on a plain document, with the editor's own merge (overlapping or
  touching ghosts become one), trim and split rules. Spans, alternatives and
  overflow are kept, and a malformed block offers neither. The new ghost
  renders through the existing `ghosted` hints once the edit is applied.
  A kind of its own rather than `refactor.rewrite`, because the body text
  never changes.
  Also command-only, the Lab actions (`refactor.terraphim.lab`): "Lab: Fix
  punctuation and typos", "Lab: Mark the weakest sentences", "Lab: Mark
  sentences that run long", "Lab: Mark convoluted sentences", "Lab: Mark words
  that don't fit the tone" and "Lab: Mark hedges and filler" run
  `terraphim.lab.mark` with `{uri, version, action}`; "Lab: Clear marks" runs
  `terraphim.lab.clear`. They are stateless like the trim menu, offered on any
  open document while the Lab engine is available, and the marks reach the
  client through the usual diagnostic push or refresh.
  The overflow (`refactor.terraphim.overflow`, R-6.3 and R-6.4): on a
  non-empty selection "Stash in Overflow" moves it out of the body to the end
  of the overflow kept in the annotation block, and at the cursor "Pull
  back: «excerpt»" copies one overflow paragraph (a run of non-blank lines;
  the first five are listed) into the body, as the editor does: the overflow
  keeps it, so it can be used again. In the
  middle of a line the paragraph goes in as it is; at the start or end of a
  line, or on a blank line, it becomes a paragraph of its own, with one blank
  line (in the document's line ending) between it and the text on each side. Both are
  versioned workspace edits (the body change and the block rewrite, merged
  when they touch), through the editor's own span model, so spans after the
  edit move with their text. A stash that touches a span, a pull back inside
  one, a malformed block and anchors that no longer match offer nothing;
  a ghost inside a stashed selection goes with it. The stashed text is the
  selection exactly. Overflow is never exported and survives every other
  rewrite of the block.
  "Export clean Markdown" (`source.terraphim.export`, R-9.3) is a
  command-only action running `terraphim.export`; it is offered only when
  the export would differ from the document and would not be refused.
- **Trim review** (R-8.4, R-8.5) - after a preview or a Keep the server
  shows the status card with `window/showMessageRequest`, without waiting
  for it: `Slight trim: 479 → 433 words · −10%. Faded words would go. Keep
  one with the Keep action.` with the buttons `Make the cuts`, `Walk
  through` and `Done`. "Make the cuts" sends the engine's `make_cuts` edit
  (deletions plus join tidy-ups) through `workspace/applyEdit`, versioned and
  rechecked against the version the card was shown for; when the client
  reports exactly that text back, the preview ends. "Walk through" selects
  the next faded span with `window/showDocument` and shows the card again;
  "Done" or dismissing it leaves the fades. Hovering a faded span shows the
  card numbers and why the span would go. Clients without
  `window.showMessage.messageActionItem` get the card as a plain message;
  without `workspace.applyEdit`, `terraphim.trim.make_cuts` returns the
  edit; without `window.showDocument.support`, `terraphim.trim.next` returns
  the range only.
- **`textDocument/inlayHint`** - `[i/n]` after each KG term with
  alternatives: the current form is synonym `i` of `n`. Off by default.
- **`textDocument/foldingRange`** (R-9.1) - the trailing annotation block
  folds as one `region`, from its opening fence to the closing fence, so the
  raw JSON can be collapsed. No range for a document without a block.
- **Re-anchoring on save** (R-9.2) - on `textDocument/didSave` the spans and
  ghosts of the annotation block are found again in the body (the editor's
  own re-anchoring) and their stored context is refreshed. When anything
  moved, one versioned `workspace/applyEdit` rewrites only the block; when
  nothing moved, or the block is malformed, nothing is sent, so saving again
  is quiet and the edit cannot loop. A span or ghost that can no longer be
  placed leaves the block alone and is reported with `window/logMessage`
  (rewriting would delete it). The edit leaves the buffer modified: the next
  save writes it. Zed does not send `willSaveWaitUntil` (its client
  capabilities declare `didSave` only), so `didSave` is the hook; the Lab
  refresh that follows waits for the client's change, so Lab marks survive.
  Turn it off with `reanchorOnSave`.
- **`workspace/executeCommand`** - see below.

Edits are versioned (`documentChanges` with the document version) for
clients that accept them, so a stale edit is rejected rather than applied.

## Lifecycle

The server exits as soon as it receives `exit`, whether or not the client
keeps stdin open: with code 0 after a `shutdown` request and code 1 without
one (LSP 3.17). End of input also ends it.

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
  "reanchorOnSave": true,
  "lab": { "actions": [], "trigger": "save" }
}
```

| Setting | Default | Effect |
|---|---|---|
| `thesaurus` | none | path of the thesaurus JSON file; overrides `--thesaurus` and `TERRAPHIM_THESAURUS`; a value from `initializationOptions` survives later settings without one; reloaded when it changes (see [Thesaurus](#thesaurus)) |
| `inlayHints` | `false` | `[i/n]` inlay hints (the capability is always advertised) |
| `ghostDiagnostics` | `true` | faded hints over ghosted text |
| `unknownTerms` | `false` | a Warning (`Unknown term: X`) on every occurrence of a word that is not part of any thesaurus match; off by default because against a real thesaurus nearly every ordinary word is unknown |
| `reanchorOnSave` | `true` | re-anchor the annotation block against the body on save, with one `workspace/applyEdit` when something moved (see above) |
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
| `terraphim.alternative.add_and_apply` | the same explicit arguments as `alternative.add` | requests `workspace/applyEdit` once and returns `{ applied: true, uri, version }` only after the expected document change is observed |
| `terraphim.lab.mark` | `uri`, `action`, `version?` | `{ action, marks }`; the action sticks to the document until cleared |
| `terraphim.lab.clear` | `uri` | `null` |
| `terraphim.trim.preview` | `uri`, `level` (`original`/`slight`/`tighten`/`sharper`/`half`), `version?` | `{ level, candidates, status, words_before, words_after, percent }`, `status` being the Lab status card (`61 → 55 words · −10%`); the card is then shown |
| `terraphim.trim.clear` | `uri` | `null` |
| `terraphim.trim.keep` | `uri`, `cut`, `version?` | like `terraphim.trim.preview`, with the cut un-faded; the card is shown again |
| `terraphim.trim.make_cuts` | `uri`, `version?` | `{ edits, words_after }`; the edit goes out with `workspace/applyEdit`, or is returned as `edit` |
| `terraphim.trim.next` | `uri`, `position`, `version?` | `{ range }` of the next faded span after `position`, wrapping; `null` with nothing faded |
| `terraphim.export` | `uri`, `version?` | `{ markdown, uri, written }`: the clean Markdown (ghosts cut, no annotation block, no overflow). With `workspace/applyEdit` and `create` resource operations, the client is asked to create `<stem>.clean.md` beside the document (replacing an earlier export) and to show it; otherwise only the text is returned. `uri` is `null` for a non-`file:` document |

`terraphim.export` refuses (`InvalidParams`) a malformed block and a ghost
whose text can no longer be found, rather than export words meant to be cut.

`terraphim.alternative.add` refuses (`InvalidParams`) a malformed block, a
range outside the body, an alternative equal to the current text or already
present, a range overlapping another span, and a block whose stored anchors
can no longer be placed (rewriting it would drop them).

For a generic LSP command picker that displays command results without applying
returned edits, use `terraphim.alternative.add_and_apply`. It requires
`workspace.applyEdit` and versioned `documentChanges`; the original
`terraphim.alternative.add` returned-edit contract is unchanged. Supply the
actual open document URI and UTF-16 range explicitly. For a document beginning
with `Risk`, this example adds `Concern` to its saved alternatives:

```json
{
  "uri": "file:///absolute/path/brief.md",
  "range": {
    "start": { "line": 0, "character": 0 },
    "end": { "line": 0, "character": 4 }
  },
  "text": "Concern",
  "kind": "word"
}
```

The command does not infer the editor selection or save the file itself. Save
the edited Markdown, then use `terraphim.export_knowledge_graph` to export its
registered synonym graph. A rejected edit, an acknowledgement timeout, an
unobserved edit or a concurrent change is reported as an error; inspect the
document before retrying an uncertain result. A successful command result is
an acknowledgement, not a second edit for the client to apply.

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
terraphim_lsp = "1.22.0-rc.4"
```

Or build the standalone binary:

```bash
cargo build --manifest-path crates/terraphim_lsp/Cargo.toml -p terraphim_lsp --bin terraphim-lsp
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
cargo test --manifest-path crates/terraphim_lsp/Cargo.toml -p terraphim_lsp

# Run linter
cargo clippy --manifest-path crates/terraphim_lsp/Cargo.toml -p terraphim_lsp --all-targets
```

## License

Apache-2.0

## Public package source

The LSP and shared analysis core use a bounded workspace rooted at
`crates/terraphim_lsp/Cargo.toml`. Its dependencies come from crates.io; it does
not inherit private registry dependencies or patches from the repository root.
The packaged tests include their own fixtures. See [LICENSE](LICENSE) for the
Apache-2.0 licence notice.
