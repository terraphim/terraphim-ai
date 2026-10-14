//! Byte and UTF-16 offsets, and line/column positions.
//!
//! The KG matcher works in UTF-8 **byte** offsets. Browser text APIs and the
//! Language Server Protocol (with its default position encoding) count
//! **UTF-16 code units**. Every position this crate returns carries both, so
//! neither consumer has to convert: the editor uses [`TextOffset::utf16`]
//! directly against a JavaScript string, and the LSP server uses
//! [`LineIndex`] to turn offsets into `line`/`character` pairs.
//!
//! All offsets are relative to the text that was passed in.

use serde::{Deserialize, Serialize};

/// A position in a text, as a UTF-8 byte offset and a UTF-16 code-unit offset.
///
/// Both fields always describe the same character boundary.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize,
)]
pub struct TextOffset {
    /// UTF-8 byte offset (valid for slicing a Rust `&str`).
    pub byte: usize,
    /// UTF-16 code-unit offset (valid for a JavaScript string or an LSP
    /// position using the default UTF-16 encoding).
    pub utf16: usize,
}

impl TextOffset {
    /// The offset of byte position `byte` in `text`.
    ///
    /// `byte` is clamped to the text length and moved back to the start of
    /// the character it falls inside, so the result is always a boundary.
    pub fn from_byte(text: &str, byte: usize) -> Self {
        let byte = floor_char_boundary(text, byte);
        Self {
            byte,
            utf16: utf16_len(&text[..byte]),
        }
    }

    /// The offset of UTF-16 position `utf16` in `text`.
    ///
    /// A position past the end clamps to the end; a position between the two
    /// halves of a surrogate pair moves back to the start of that character.
    pub fn from_utf16(text: &str, utf16: usize) -> Self {
        let mut units = 0;
        for (byte, ch) in text.char_indices() {
            let next = units + ch.len_utf16();
            if next > utf16 {
                return Self { byte, utf16: units };
            }
            units = next;
        }
        Self {
            byte: text.len(),
            utf16: units,
        }
    }
}

/// A half-open range `[start, end)` in a text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct TextRange {
    /// First position inside the range.
    pub start: TextOffset,
    /// First position after the range.
    pub end: TextOffset,
}

impl TextRange {
    /// The range of bytes `start..end` in `text` (see [`TextOffset::from_byte`]).
    pub fn from_bytes(text: &str, start: usize, end: usize) -> Self {
        let start = TextOffset::from_byte(text, start);
        Self {
            start,
            end: Utf16Cursor::starting_at(text, start).offset(end),
        }
    }

    /// The range of UTF-16 code units `start..end` in `text` (see
    /// [`TextOffset::from_utf16`]).
    pub fn from_utf16(text: &str, start: usize, end: usize) -> Self {
        Self {
            start: TextOffset::from_utf16(text, start),
            end: TextOffset::from_utf16(text, end),
        }
    }

    /// The byte range, for slicing the text the range was computed on.
    pub fn bytes(&self) -> std::ops::Range<usize> {
        self.start.byte..self.end.byte
    }

    /// Whether `byte` lies in the range, counting both ends as inside, so a
    /// cursor just after a word still finds it.
    pub fn touches_byte(&self, byte: usize) -> bool {
        self.start.byte <= byte && byte <= self.end.byte
    }
}

/// Number of UTF-16 code units in `text`.
pub fn utf16_len(text: &str) -> usize {
    text.chars().map(char::len_utf16).sum()
}

/// Largest character boundary in `text` that is `<= byte`.
pub(crate) fn floor_char_boundary(text: &str, byte: usize) -> usize {
    if byte >= text.len() {
        return text.len();
    }
    (0..=byte)
        .rev()
        .find(|&candidate| text.is_char_boundary(candidate))
        .unwrap_or(0)
}

/// Converts a nondecreasing sequence of byte offsets to [`TextOffset`]s in a
/// single forward pass, instead of rescanning from the start for each one.
#[derive(Debug, Clone)]
pub(crate) struct Utf16Cursor<'a> {
    text: &'a str,
    at: TextOffset,
}

impl<'a> Utf16Cursor<'a> {
    /// A cursor at the start of `text`.
    pub(crate) fn new(text: &'a str) -> Self {
        Self {
            text,
            at: TextOffset::default(),
        }
    }

    fn starting_at(text: &'a str, at: TextOffset) -> Self {
        Self { text, at }
    }

    /// The offset of `byte`. Offsets before the previous call restart the
    /// scan from the beginning, so the result is always correct; only the
    /// forward case is fast.
    pub(crate) fn offset(&mut self, byte: usize) -> TextOffset {
        let byte = floor_char_boundary(self.text, byte);
        if byte < self.at.byte {
            self.at = TextOffset::default();
        }
        self.at = TextOffset {
            byte,
            utf16: self.at.utf16 + utf16_len(&self.text[self.at.byte..byte]),
        };
        self.at
    }

    /// The range of bytes `start..end`.
    pub(crate) fn range(&mut self, start: usize, end: usize) -> TextRange {
        TextRange {
            start: self.offset(start),
            end: self.offset(end),
        }
    }
}

/// A zero-based line and UTF-16 column, the shape of an LSP `Position`.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize,
)]
pub struct LinePosition {
    /// Zero-based line number. Lines end at `\n` or `\r\n`; the line break
    /// is never part of the line's content.
    pub line: u32,
    /// Zero-based UTF-16 code-unit column within the line.
    pub character: u32,
}

/// Line-start table for converting between byte offsets and
/// [`LinePosition`]s in one text.
///
/// Build it once per document version and reuse it for every conversion.
#[derive(Debug, Clone)]
pub struct LineIndex<'a> {
    text: &'a str,
    line_starts: Vec<usize>,
}

impl<'a> LineIndex<'a> {
    /// Index the line starts of `text`.
    pub fn new(text: &'a str) -> Self {
        let line_starts = std::iter::once(0)
            .chain(text.match_indices('\n').map(|(index, _)| index + 1))
            .collect();
        Self { text, line_starts }
    }

    /// The line/column of byte offset `byte` (clamped to a character
    /// boundary inside the text).
    pub fn position(&self, byte: usize) -> LinePosition {
        let byte = floor_char_boundary(self.text, byte);
        let line = self.line_starts.partition_point(|&start| start <= byte) - 1;
        let line_start = self.line_starts[line];
        // An offset inside a line break (between `\r` and `\n`) maps to the
        // end of the line's content, the same place `byte_offset` clamps to.
        let byte = byte.min(self.content_end(line));
        LinePosition {
            line: saturating_u32(line),
            character: saturating_u32(utf16_len(&self.text[line_start..byte])),
        }
    }

    /// The byte offset of `position`.
    ///
    /// Following the LSP rules, a column past the end of the line resolves to
    /// the end of the line (before its `\n` or `\r\n`), and a line past the last one
    /// resolves to the end of the text. A column between the halves of a
    /// surrogate pair moves back to the start of that character.
    pub fn byte_offset(&self, position: LinePosition) -> usize {
        let line = position.line as usize;
        let Some(&line_start) = self.line_starts.get(line) else {
            return self.text.len();
        };
        let line_text = &self.text[line_start..self.content_end(line)];
        line_start + TextOffset::from_utf16(line_text, position.character as usize).byte
    }

    /// End of line `line`'s content: before its `\n`, or before the `\r` of
    /// a `\r\n` line end. `line` must be a valid line index.
    fn content_end(&self, line: usize) -> usize {
        match self.line_starts.get(line + 1) {
            Some(&next) => {
                let newline = next - 1;
                if self.text[..newline].ends_with('\r') {
                    newline - 1
                } else {
                    newline
                }
            }
            None => self.text.len(),
        }
    }

    /// The [`TextOffset`] of `position` (see [`LineIndex::byte_offset`]).
    pub fn offset(&self, position: LinePosition) -> TextOffset {
        TextOffset::from_byte(self.text, self.byte_offset(position))
    }
}

fn saturating_u32(value: usize) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ascii_offsets_agree() {
        let offset = TextOffset::from_byte("hello world", 6);
        assert_eq!(offset, TextOffset { byte: 6, utf16: 6 });
    }

    #[test]
    fn multi_byte_and_astral_offsets() {
        // 'é' is 2 bytes / 1 unit; '😀' is 4 bytes / 2 units.
        let text = "é😀x";
        assert_eq!(
            TextOffset::from_byte(text, 2),
            TextOffset { byte: 2, utf16: 1 }
        );
        assert_eq!(
            TextOffset::from_byte(text, 6),
            TextOffset { byte: 6, utf16: 3 }
        );
        assert_eq!(
            TextOffset::from_utf16(text, 3),
            TextOffset { byte: 6, utf16: 3 }
        );
    }

    #[test]
    fn offsets_inside_a_character_move_back() {
        let text = "é😀x";
        assert_eq!(
            TextOffset::from_byte(text, 1),
            TextOffset { byte: 0, utf16: 0 }
        );
        assert_eq!(
            TextOffset::from_byte(text, 4),
            TextOffset { byte: 2, utf16: 1 }
        );
        // Between the surrogate halves of '😀'.
        assert_eq!(
            TextOffset::from_utf16(text, 2),
            TextOffset { byte: 2, utf16: 1 }
        );
    }

    #[test]
    fn offsets_past_the_end_clamp() {
        let text = "a😀";
        assert_eq!(
            TextOffset::from_byte(text, 99),
            TextOffset { byte: 5, utf16: 3 }
        );
        assert_eq!(
            TextOffset::from_utf16(text, 99),
            TextOffset { byte: 5, utf16: 3 }
        );
    }

    #[test]
    fn cursor_matches_direct_conversion_even_backwards() {
        let text = "a😀b€c";
        let mut cursor = Utf16Cursor::new(text);
        for byte in [0, 1, 5, 6, 9, 10, 2, 0, 10] {
            assert_eq!(
                cursor.offset(byte),
                TextOffset::from_byte(text, byte),
                "{byte}"
            );
        }
    }

    #[test]
    fn range_from_bytes() {
        let text = "😀 rust";
        let range = TextRange::from_bytes(text, 5, 9);
        assert_eq!(range.start, TextOffset { byte: 5, utf16: 3 });
        assert_eq!(range.end, TextOffset { byte: 9, utf16: 7 });
        assert_eq!(&text[range.bytes()], "rust");
    }

    #[test]
    fn line_index_round_trips() {
        let text = "line one\nli😀ne two\r\nthird";
        let index = LineIndex::new(text);
        for byte in [0, 3, 8, 9, 11, 15, 21, 23, text.len()] {
            let position = index.position(byte);
            assert_eq!(index.byte_offset(position), byte, "{byte}: {position:?}");
        }
        assert_eq!(
            index.position(15),
            LinePosition {
                line: 1,
                character: 4
            }
        );
    }

    #[test]
    fn crlf_line_ends_are_not_line_content() {
        let text = "ab\r\ncd\r\n";
        let index = LineIndex::new(text);
        let past_line_end = LinePosition {
            line: 0,
            character: 100,
        };
        // Clamps before the `\r`, not between `\r` and `\n`.
        assert_eq!(index.byte_offset(past_line_end), 2);
        assert_eq!(
            index.byte_offset(LinePosition {
                line: 1,
                character: 2
            }),
            6
        );
        // Offsets inside the line break map to the end of the content.
        for byte in [2, 3] {
            assert_eq!(
                index.position(byte),
                LinePosition {
                    line: 0,
                    character: 2
                },
                "{byte}"
            );
        }
        assert_eq!(
            index.position(4),
            LinePosition {
                line: 1,
                character: 0
            }
        );
        // The empty last line after the final CRLF.
        assert_eq!(
            index.position(text.len()),
            LinePosition {
                line: 2,
                character: 0
            }
        );
    }

    #[test]
    fn line_index_clamps_like_lsp() {
        let text = "abc\ndef";
        let index = LineIndex::new(text);
        let past_line_end = LinePosition {
            line: 0,
            character: 100,
        };
        assert_eq!(index.byte_offset(past_line_end), 3);
        let past_last_line = LinePosition {
            line: 5,
            character: 0,
        };
        assert_eq!(index.byte_offset(past_last_line), text.len());
    }
}
