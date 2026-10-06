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
//!   block yields exactly one [`Diagnostic`], and a well-formed one's ghosts
//!   become [`DiagnosticTag::Unnecessary`] hints
//!   ([`AnnotationBlock::ghost_diagnostics`]).
//! - [`KgEngine::synonym_positions`]: the `[i/n]` position of each matched
//!   form among its concept's terms, for inlay hints.
//! - [`lab_findings`] / [`trim_preview`]: terraphim-editor's Lab marks
//!   (from [`terraphim_lab`]) as diagnostics with one code per mark kind and
//!   "Apply fix: X" fixes, and trim candidates at a level as faded hints.
//! - [`add_alternative`]: record a human-written alternative for a body
//!   range, as one edit that rewrites the trailing block with the editor's
//!   own writer.
//!
//! Matching and the concept -> synonyms index come from
//! [`terraphim_automata`] ([`terraphim_automata::CompiledMatcher`] and
//! [`terraphim_automata::ConceptIndex`]). The annotation-block parser and
//! the `a`/`an` rules are terraphim-editor's own, from
//! [`terraphim_alternatives`], so the editor and LSP clients cannot drift
//! apart. This crate adds positions, casing and edits on top.
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

mod alternative;
mod block;
mod case;
mod diagnostic;
mod engine;
mod lab;
mod offset;

pub use alternative::{
    AddAlternativeError, AlternativeAdded, add_alternative, add_alternative_utf16,
};
pub use block::{AnnotationBlock, BlockSplit, FENCE_INFO, GHOSTED_MESSAGE, split_annotation_block};
pub use case::Capitalisation;
pub use diagnostic::{Diagnostic, DiagnosticCode, DiagnosticTag, Severity};
pub use engine::{
    AlternativeSet, Analysis, CoreError, KgEngine, Replacement, SynonymPosition, TermMatch,
    TextEdit, apply_edits,
};
pub use lab::{
    LabAction, LabFinding, LabFix, TrimPreview, code_for_mark, lab_findings, severity_for_mark,
    trim_preview,
};
pub use offset::{LineIndex, LinePosition, TextOffset, TextRange, utf16_len};
/// Span granularity for [`add_alternative`], re-exported from
/// `terraphim_alternatives`.
pub use terraphim_alternatives::SpanKind;
/// The editor's a/an rules, re-exported from `terraphim_alternatives`.
pub use terraphim_alternatives::{Article, article_for};
/// The Lab engine's configuration, mark kinds and trim levels, re-exported
/// from `terraphim_lab`.
pub use terraphim_lab::{LabConfig, LabError, MarkKind, TrimLevel};

/// Re-exported so callers can build a thesaurus without a direct dependency.
pub use terraphim_types::Thesaurus;
