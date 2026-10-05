//! Pure knowledge-graph analysis core for Terraphim editors.
//!
//! This crate is the engine behind `terraphim_lsp` and terraphim-editor's
//! alternative control. It has no LSP, async runtime or I/O dependencies and
//! builds for `wasm32-unknown-unknown`, so the editor can run it in-process
//! while the LSP server wraps the same code for Zed, VS Code and others.
//!
//! In a knowledge graph every synonym of a concept shares the concept's id,
//! so the synonyms *are* the alternatives for a matched word. The crate
//! provides:
//!
//! - [`KgEngine::analyse`]: the KG terms in a document, each with its concept
//!   id, normalised concept name (`nterm`) and byte plus UTF-16 range.
//! - [`KgEngine::alternatives_at`] / [`KgEngine::alternatives_for`]: the other
//!   terms of the matched concept (current form excluded) as ready-to-apply
//!   [`Replacement`]s that keep the original's [`Capitalisation`] and fix a
//!   preceding `a`/`an` in the same edit.
//! - [`split_annotation_block`]: locating the trailing
//!   `terraphim-alternatives` block so it is never analysed; a malformed
//!   block yields exactly one [`Diagnostic`].
//!
//! Matching and the concept -> synonyms index come from
//! [`terraphim_automata`] ([`terraphim_automata::CompiledMatcher`] and
//! [`terraphim_automata::ConceptIndex`]); this crate adds positions,
//! casing, articles and the block boundary on top.
//!
//! Every type that crosses the API is plain data with serde derives, so it
//! can pass through `wasm-bindgen` as JSON. Offsets are relative to the text
//! passed in and carry both byte and UTF-16 positions ([`TextOffset`]).
//!
//! # Example
//!
//! ```
//! use terraphim_lsp_core::{KgEngine, apply_edits};
//!
//! let thesaurus = r#"{"name": "demo", "data": {
//!     "decision": {"id": 1, "nterm": "decision"},
//!     "choice":   {"id": 1, "nterm": "decision"},
//!     "judgment": {"id": 1, "nterm": "decision"}
//! }}"#;
//! let engine = KgEngine::from_json(thesaurus)?;
//!
//! let text = "Choice matters.";
//! let analysis = engine.analyse(text);
//! assert_eq!(analysis.matches[0].nterm, "decision");
//!
//! let set = engine.alternatives_for(text, &analysis.matches[0]).unwrap();
//! let offered: Vec<&str> = set.replacements.iter().map(|r| r.text.as_str()).collect();
//! assert_eq!(offered, ["Decision", "Judgment"]);
//! assert_eq!(apply_edits(text, &set.replacements[1].edits), "Judgment matters.");
//! # Ok::<(), terraphim_lsp_core::CoreError>(())
//! ```

mod article;
mod block;
mod case;
mod diagnostic;
mod engine;
mod offset;

pub use article::{Article, article_for};
pub use block::{AnnotationBlock, BlockSplit, FENCE_INFO, split_annotation_block};
pub use case::Capitalisation;
pub use diagnostic::{Diagnostic, DiagnosticCode, Severity};
pub use engine::{
    AlternativeSet, Analysis, CoreError, KgEngine, Replacement, TermMatch, TextEdit, apply_edits,
};
pub use offset::{LineIndex, LinePosition, TextOffset, TextRange, utf16_len};

/// Re-exported so callers can build a thesaurus without a direct dependency.
pub use terraphim_types::Thesaurus;
