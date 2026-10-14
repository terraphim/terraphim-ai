//! Clean Markdown export (R-9.3).
//!
//! The export is the body with every ghost range removed (and the whitespace
//! around a cut tidied), without the annotation block and without overflow:
//! what the writer means to publish. It is the editor's own
//! `Document::export`, run after re-anchoring so a ghost whose stored offsets
//! went stale is still cut from the right place.

use terraphim_alternatives::parse;

use crate::diagnostic::DiagnosticCode;

/// Why authored synonym groups could not be shared as a single thesaurus.
#[derive(Debug, thiserror::Error)]
pub enum KnowledgeGraphExportError {
    /// Invalid annotations must be repaired before exporting.
    #[error("invalid annotation block: {0}")]
    InvalidBlock(String),
    /// Conflicting concepts cannot be flattened without changing meaning.
    #[error(transparent)]
    ConflictingConcept(#[from] terraphim_alternatives::SynonymExportError),
}

/// Export attached document alternatives using Terraphim synonym parser/types.
///
/// The shared role and document remain unchanged. Detached groups are excluded.
///
/// # Errors
/// Refuses malformed annotations and synonyms belonging to conflicting groups.
pub fn export_knowledge_graph(
    text: &str,
) -> Result<terraphim_types::Thesaurus, KnowledgeGraphExportError> {
    let document = parse(text)
        .map_err(|error| KnowledgeGraphExportError::InvalidBlock(error.kind.to_string()))?;
    Ok(document.export_synonym_thesaurus()?)
}

/// Why a document could not be exported. Nothing is produced rather than a
/// text that might still hold ghosted words.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ExportError {
    /// The annotation block is malformed, so the ghosts cannot be read.
    #[error("the annotation block is malformed ({}): {message}", code.as_str())]
    MalformedBlock {
        /// The block diagnostic's code.
        code: DiagnosticCode,
        /// The parser's message.
        message: String,
    },
    /// Some ghosts no longer match the body and cannot be found again, so
    /// the export might keep text meant to be dropped.
    #[error("{0} ghost(s) no longer match the body; resolve them before exporting")]
    UnresolvedGhosts(usize),
}

/// The clean Markdown of `text`: ghosts cut, annotation block and overflow
/// omitted.
///
/// ```
/// use terraphim_lsp_core::export_clean;
///
/// let text = "Keep this. Drop this.\n\n```terraphim-alternatives\n\
///     {\"version\": 1, \"spans\": [], \"overflow\": \"notes\", \"ghosts\": \
///     [{\"id\": \"g1\", \"anchor\": {\"start\": 11, \"end\": 21, \
///     \"text\": \"Drop this.\"}}]}\n```\n";
/// assert_eq!(export_clean(text)?, "Keep this.");
/// # Ok::<(), terraphim_lsp_core::ExportError>(())
/// ```
pub fn export_clean(text: &str) -> Result<String, ExportError> {
    let mut document = parse(text).map_err(|error| ExportError::MalformedBlock {
        code: DiagnosticCode::for_block_error(&error.kind),
        message: error.kind.to_string(),
    })?;
    let report = document.reanchor();
    if !report.unresolved_ghosts.is_empty() {
        return Err(ExportError::UnresolvedGhosts(
            report.unresolved_ghosts.len(),
        ));
    }
    Ok(document.export())
}
