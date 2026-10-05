//! Conversions between `terraphim_lsp_core` plain data and `lsp_types`.
//!
//! LSP positions count UTF-16 code units (the protocol's default encoding);
//! the core's [`LineIndex`] does the counting, so multi-byte and astral text
//! map to the right columns.

use terraphim_lsp_core::{
    Diagnostic as CoreDiagnostic, LineIndex, LinePosition, Severity, TextEdit as CoreTextEdit,
    TextRange,
};
use tower_lsp::lsp_types::{
    Diagnostic, DiagnosticSeverity, NumberOrString, Position, Range, TextEdit,
};

/// Source name attached to every diagnostic this server publishes.
pub(crate) const SOURCE: &str = "terraphim-lsp";

/// The byte offset of an LSP position.
pub(crate) fn byte_offset(index: &LineIndex<'_>, position: Position) -> usize {
    index.byte_offset(LinePosition {
        line: position.line,
        character: position.character,
    })
}

/// The LSP position of a byte offset.
pub(crate) fn position(index: &LineIndex<'_>, byte: usize) -> Position {
    let LinePosition { line, character } = index.position(byte);
    Position { line, character }
}

/// The LSP range of the byte range `start..end`.
pub(crate) fn byte_range(index: &LineIndex<'_>, start: usize, end: usize) -> Range {
    Range {
        start: position(index, start),
        end: position(index, end),
    }
}

/// The LSP range of a core [`TextRange`].
pub(crate) fn range(index: &LineIndex<'_>, range: TextRange) -> Range {
    byte_range(index, range.start.byte, range.end.byte)
}

/// An LSP text edit from a core edit.
pub(crate) fn text_edit(index: &LineIndex<'_>, edit: &CoreTextEdit) -> TextEdit {
    TextEdit {
        range: range(index, edit.range),
        new_text: edit.new_text.clone(),
    }
}

/// An LSP diagnostic from a core diagnostic.
pub(crate) fn diagnostic(index: &LineIndex<'_>, diagnostic: &CoreDiagnostic) -> Diagnostic {
    Diagnostic {
        range: range(index, diagnostic.range),
        severity: Some(match diagnostic.severity {
            Severity::Error => DiagnosticSeverity::ERROR,
            Severity::Warning => DiagnosticSeverity::WARNING,
            Severity::Information => DiagnosticSeverity::INFORMATION,
            Severity::Hint => DiagnosticSeverity::HINT,
        }),
        code: Some(NumberOrString::String(diagnostic.code.as_str().to_string())),
        code_description: None,
        source: Some(SOURCE.to_string()),
        message: diagnostic.message.clone(),
        related_information: None,
        tags: None,
        data: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn positions_count_utf16_units() {
        // '😀' is 4 bytes but 2 UTF-16 units.
        let text = "😀 rust\nnext";
        let index = LineIndex::new(text);
        assert_eq!(
            position(&index, 5),
            Position {
                line: 0,
                character: 3
            }
        );
        assert_eq!(
            byte_offset(
                &index,
                Position {
                    line: 0,
                    character: 3
                }
            ),
            5
        );
        assert_eq!(
            byte_range(&index, 5, 14),
            Range {
                start: Position {
                    line: 0,
                    character: 3
                },
                end: Position {
                    line: 1,
                    character: 4
                },
            }
        );
    }

    #[test]
    fn out_of_range_positions_clamp() {
        let text = "abc\ndef";
        let index = LineIndex::new(text);
        let past_line_end = Position {
            line: 0,
            character: 100,
        };
        assert_eq!(byte_offset(&index, past_line_end), 3);
        let past_last_line = Position {
            line: 5,
            character: 0,
        };
        assert_eq!(byte_offset(&index, past_last_line), text.len());
    }
}
