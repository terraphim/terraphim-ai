//! Lab marks and trim candidates as plain-data diagnostics.
//!
//! The marks and the trim plan come from terraphim-editor's own Lab engine,
//! [`terraphim_lab`], so the editor and every LSP client flag exactly the
//! same spans. This module only positions them in the full text (the
//! engine's offsets are UTF-16 into the body; the annotation block is never
//! given to it) and maps them to [`Diagnostic`]s:
//!
//! | Mark kind | Code | Severity | Fix |
//! |---|---|---|---|
//! | typo | `lab-typo` | Information | "Apply fix: X" |
//! | punctuation | `lab-punctuation` | Information | "Apply fix: X" |
//! | weak sentence | `lab-weak-sentence` | Hint | |
//! | long sentence | `lab-long-sentence` | Hint | |
//! | convoluted sentence | `lab-convoluted-sentence` | Hint | |
//! | off-tone | `lab-off-tone` | Hint | |
//! | hedge | `lab-hedge` | Hint | |
//! | filler | `lab-filler` | Hint | |
//! | trim candidate | `trim-candidate` | Hint, `Unnecessary` | |
//!
//! The message is the engine's reason. A fix replaces exactly the marked
//! range with the engine's proposal and nothing else: no `a`/`an` fix-up is
//! added, because the engine's proposals are already complete replacements
//! for their range.

use serde::{Deserialize, Serialize};
use terraphim_lab::{Cut, LabConfig, LabMark, MarkKind, TrimLevel, mark_many, trim_plan};

use crate::block::split_annotation_block;
use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticTag, Severity};
use crate::engine::TextEdit;
use crate::offset::{TextOffset, TextRange};

pub use terraphim_lab::LabAction;

/// A replacement the Lab proposes for a marked range.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LabFix {
    /// The code-action title, `Apply fix: X`.
    pub title: String,
    /// The edit: the marked range replaced by the proposal.
    pub edit: TextEdit,
}

/// One Lab mark positioned in the text.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LabFinding {
    /// The mark as a diagnostic.
    pub diagnostic: Diagnostic,
    /// The action that produced it.
    pub action: LabAction,
    /// The proposed fix, for typos and punctuation.
    pub fix: Option<LabFix>,
}

/// Trim candidates at one level, positioned in the text.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrimPreview {
    /// The level previewed.
    pub level: TrimLevel,
    /// One [`DiagnosticCode::TrimCandidate`] hint per faded span, tagged
    /// [`DiagnosticTag::Unnecessary`]. A cut lying inside another faded cut
    /// is left out: the enclosing one already fades it.
    pub diagnostics: Vec<Diagnostic>,
    /// The engine's status card line, `535 → 480 words · −10%`.
    pub status: String,
}

/// The diagnostic code of a mark kind.
pub fn code_for_mark(kind: MarkKind) -> DiagnosticCode {
    match kind {
        MarkKind::Typo => DiagnosticCode::LabTypo,
        MarkKind::Punctuation => DiagnosticCode::LabPunctuation,
        MarkKind::WeakSentence => DiagnosticCode::LabWeakSentence,
        MarkKind::LongSentence => DiagnosticCode::LabLongSentence,
        MarkKind::ConvolutedSentence => DiagnosticCode::LabConvolutedSentence,
        MarkKind::OffTone => DiagnosticCode::LabOffTone,
        MarkKind::Hedge => DiagnosticCode::LabHedge,
        MarkKind::Filler => DiagnosticCode::LabFiller,
    }
}

/// The severity of a mark kind: Information for the kinds with a fix,
/// Hint for the rest.
pub fn severity_for_mark(kind: MarkKind) -> Severity {
    match kind {
        MarkKind::Typo | MarkKind::Punctuation => Severity::Information,
        _ => Severity::Hint,
    }
}

/// Run `actions` over the body of `text` (the annotation block excluded)
/// and return the marks as findings, in document order.
///
/// ```
/// use terraphim_lsp_core::{DiagnosticCode, LabAction, LabConfig, lab_findings};
///
/// let config = LabConfig::with_defaults().unwrap();
/// let text = "We recieve it.";
/// let found = lab_findings(text, &config, &[LabAction::TyposAndPunctuation]);
/// assert_eq!(found[0].diagnostic.code, DiagnosticCode::LabTypo);
/// assert_eq!(&text[found[0].diagnostic.range.bytes()], "recieve");
/// assert_eq!(found[0].fix.as_ref().unwrap().title, "Apply fix: receive");
/// ```
pub fn lab_findings(text: &str, config: &LabConfig, actions: &[LabAction]) -> Vec<LabFinding> {
    let body = &text[..split_annotation_block(text).body_end.byte];
    let marks = mark_many(body, config, actions);
    let ranges = ranges_from_utf16(body, marks.iter().map(|m| (m.start, m.end)));
    marks
        .into_iter()
        .zip(ranges)
        .map(|(mark, range)| finding(mark, range))
        .collect()
}

/// The spans trimmed at `level` in the body of `text`, as faded hints.
///
/// ```
/// use terraphim_lsp_core::{DiagnosticTag, LabConfig, TrimLevel, trim_preview};
///
/// let config = LabConfig::with_defaults().unwrap();
/// let text = "The editor, which owns its DOM, is the only target here today.";
/// let preview = trim_preview(text, &config, TrimLevel::Sharper);
/// assert_eq!(&text[preview.diagnostics[0].range.bytes()], ", which owns its DOM,");
/// assert_eq!(preview.diagnostics[0].tags, [DiagnosticTag::Unnecessary]);
/// ```
pub fn trim_preview(text: &str, config: &LabConfig, level: TrimLevel) -> TrimPreview {
    let body = &text[..split_annotation_block(text).body_end.byte];
    let plan = trim_plan(body, config);
    let faded: Vec<&Cut> = plan.faded(level).collect();
    let mut outermost: Vec<&Cut> = faded
        .iter()
        .copied()
        .filter(|cut| {
            !faded.iter().any(|other| {
                !std::ptr::eq(*other, *cut)
                    && other.start <= cut.start
                    && cut.end <= other.end
                    && (other.start, other.end) != (cut.start, cut.end)
            })
        })
        .collect();
    outermost.dedup_by_key(|cut| (cut.start, cut.end));
    let ranges = ranges_from_utf16(body, outermost.iter().map(|cut| (cut.start, cut.end)));
    let diagnostics = outermost
        .iter()
        .zip(ranges)
        .map(|(cut, range)| Diagnostic {
            range,
            severity: Severity::Hint,
            code: DiagnosticCode::TrimCandidate,
            message: format!("{}: {}", level.label(), cut.reason),
            tags: vec![DiagnosticTag::Unnecessary],
        })
        .collect();
    TrimPreview {
        level,
        diagnostics,
        status: plan.status(level, &[]).card_text(),
    }
}

fn finding(mark: LabMark, range: TextRange) -> LabFinding {
    let fix = mark.proposal.as_ref().map(|proposal| LabFix {
        title: format!("Apply fix: {proposal}"),
        edit: TextEdit {
            range,
            new_text: proposal.clone(),
        },
    });
    LabFinding {
        diagnostic: Diagnostic {
            range,
            severity: severity_for_mark(mark.kind),
            code: code_for_mark(mark.kind),
            message: mark.reason,
            tags: Vec::new(),
        },
        action: mark.kind.action(),
        fix,
    }
}

/// Convert UTF-16 ranges of `body` to [`TextRange`]s in one pass over the
/// text, whatever order the ranges come in.
fn ranges_from_utf16(body: &str, ranges: impl Iterator<Item = (usize, usize)>) -> Vec<TextRange> {
    let ranges: Vec<(usize, usize)> = ranges.collect();
    let mut wanted: Vec<usize> = ranges.iter().flat_map(|&(s, e)| [s, e]).collect();
    wanted.sort_unstable();
    wanted.dedup();
    let mut offsets: Vec<TextOffset> = Vec::with_capacity(wanted.len());
    let mut chars = body.char_indices();
    let mut at = TextOffset::default();
    for &target in &wanted {
        while at.utf16 < target {
            let Some((byte, ch)) = chars.next() else {
                break;
            };
            debug_assert_eq!(byte, at.byte);
            at = TextOffset {
                byte: byte + ch.len_utf8(),
                utf16: at.utf16 + ch.len_utf16(),
            };
        }
        offsets.push(at);
    }
    let lookup = |utf16: usize| offsets[wanted.binary_search(&utf16).expect("collected")];
    ranges
        .into_iter()
        .map(|(start, end)| TextRange {
            start: lookup(start),
            end: lookup(end),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utf16_ranges_cover_astral_and_multibyte_text() {
        let body = "a \u{1F600} café b";
        // "café" starts after "a 😀 " = 1 + 1 + 2 + 1 = 5 UTF-16 units.
        let ranges = ranges_from_utf16(body, [(5, 9), (0, 1)].into_iter());
        assert_eq!(&body[ranges[0].bytes()], "café");
        assert_eq!(&body[ranges[1].bytes()], "a");
        assert_eq!(ranges[0].start.utf16, 5);
    }

    #[test]
    fn every_mark_kind_has_its_own_code() {
        let kinds = [
            MarkKind::Typo,
            MarkKind::Punctuation,
            MarkKind::WeakSentence,
            MarkKind::LongSentence,
            MarkKind::ConvolutedSentence,
            MarkKind::OffTone,
            MarkKind::Hedge,
            MarkKind::Filler,
        ];
        let codes: std::collections::HashSet<_> = kinds.into_iter().map(code_for_mark).collect();
        assert_eq!(codes.len(), kinds.len());
        assert_eq!(severity_for_mark(MarkKind::Typo), Severity::Information);
        assert_eq!(severity_for_mark(MarkKind::Hedge), Severity::Hint);
    }
}
