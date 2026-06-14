# terraphim_lsp

Language Server Protocol (LSP) implementation for Terraphim knowledge graphs.

## Overview

`terraphim_lsp` provides editor support for Terraphim knowledge-graph markdown
documents. It analyses open documents against a knowledge-graph thesaurus and
offers:

- **`textDocument/hover`** - Show concept descriptions when hovering over
  thesaurus terms.
- **`textDocument/completion`** - Suggest knowledge-graph terms at the cursor.
- **`textDocument/diagnostic`** - Warn about terms in the document that are not
  present in the thesaurus.

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
        └── diagnostic ─► diagnostics.rs ───► Diagnostic[]
```

Open documents are tracked in memory. On every `did_open` and `did_change` the
document is re-analysed and diagnostics are published to the client.

## Testing

```bash
# Run unit and integration tests
cargo test -p terraphim_lsp

# Run linter
cargo clippy -p terraphim_lsp --all-targets
```

## License

Apache-2.0
