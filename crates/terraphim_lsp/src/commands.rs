//! `workspace/executeCommand` commands and their arguments.
//!
//! Every command takes a single JSON object as its only argument. Commands
//! that act on a document accept an optional `version`: when given and the
//! server holds a different version of the document, the request fails with
//! `ContentModified` (-32801) instead of acting on text the client no longer
//! shows.
//!
//! | Command | Arguments | Result |
//! |---|---|---|
//! | [`ADD_ALTERNATIVE`] | `uri`, `range`, `text`, `kind?`, `version?` | a `WorkspaceEdit` rewriting the annotation block |
//! | [`LAB_MARK`] | `uri`, `action`, `version?` | `{ action, marks }`; the marks are published as diagnostics |
//! | [`LAB_CLEAR`] | `uri` | `null`; Lab marks removed |
//! | [`TRIM_PREVIEW`] | `uri`, `level`, `version?` | `{ level, candidates, status }`; candidates published as faded hints |
//! | [`TRIM_CLEAR`] | `uri` | `null`; trim hints removed |
//!
//! `action` is one of `typos_and_punctuation`, `weakest_sentences`,
//! `long_sentences`, `convoluted_sentences`, `off_tone`,
//! `hedges_and_filler`; `level` one of `original` (same as clearing),
//! `slight`, `tighten`, `sharper`, `half`.
//!
//! Requested Lab actions and the trim level stick to the document until
//! cleared. Their results are dropped on every edit (stale ranges are worse
//! than none) and recomputed on save when `lab.trigger` is `save` (the
//! default), or on the next command when it is `command`.

use serde::Deserialize;
use serde_json::Value;
use tower_lsp::jsonrpc::{Error, ErrorCode};
use tower_lsp::lsp_types::{Range, Url};

use terraphim_lsp_core::{LabAction, SpanKind, TrimLevel};

/// Add a human-written alternative for a range to the annotation block.
///
/// The result is a `WorkspaceEdit` (versioned when the client accepts
/// `documentChanges`, like the code actions) that replaces the trailing
/// block and nothing else. The server does not apply it: the client applies
/// the returned edit, so a client that never asked for it is never edited.
pub const ADD_ALTERNATIVE: &str = "terraphim.alternative.add";

/// Run one Lab action on a document and publish its marks as diagnostics
/// (one code per mark kind; typo and punctuation marks get "Apply fix: X"
/// quick fixes).
pub const LAB_MARK: &str = "terraphim.lab.mark";

/// Remove a document's Lab marks and forget the actions requested for it.
pub const LAB_CLEAR: &str = "terraphim.lab.clear";

/// Publish the spans a trim level would cut as faded (`Unnecessary`) hints.
pub const TRIM_PREVIEW: &str = "terraphim.trim.preview";

/// Remove a document's trim preview.
pub const TRIM_CLEAR: &str = "terraphim.trim.clear";

/// Every command the server executes, as advertised in
/// `executeCommandProvider.commands`.
pub const ALL: &[&str] = &[
    ADD_ALTERNATIVE,
    LAB_MARK,
    LAB_CLEAR,
    TRIM_PREVIEW,
    TRIM_CLEAR,
];

/// Arguments of [`ADD_ALTERNATIVE`].
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AddAlternativeArgs {
    /// The document.
    pub uri: Url,
    /// The body range the alternative is for.
    pub range: Range,
    /// The alternative text.
    pub text: String,
    /// Span granularity (`word`, `sentence` or `paragraph`); inferred from
    /// the range's text when omitted.
    #[serde(default)]
    pub kind: Option<SpanKind>,
    /// The document version the range refers to.
    #[serde(default)]
    pub version: Option<i32>,
}

/// Arguments of [`LAB_MARK`].
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LabMarkArgs {
    /// The document.
    pub uri: Url,
    /// The action to run.
    pub action: LabAction,
    /// The document version the client expects.
    #[serde(default)]
    pub version: Option<i32>,
}

/// Arguments of [`TRIM_PREVIEW`].
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrimPreviewArgs {
    /// The document.
    pub uri: Url,
    /// The level to preview.
    pub level: TrimLevel,
    /// The document version the client expects.
    #[serde(default)]
    pub version: Option<i32>,
}

/// Arguments of [`LAB_CLEAR`] and [`TRIM_CLEAR`].
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DocumentArgs {
    /// The document.
    pub uri: Url,
}

/// Parse a command's single object argument.
pub fn single_argument<T: serde::de::DeserializeOwned>(
    command: &str,
    arguments: Vec<Value>,
) -> Result<T, Error> {
    let [argument]: [Value; 1] = arguments
        .try_into()
        .map_err(|_| invalid_params(format!("{command} takes exactly one object argument")))?;
    serde_json::from_value(argument)
        .map_err(|error| invalid_params(format!("{command}: invalid arguments: {error}")))
}

/// An `InvalidParams` error with `message`.
pub fn invalid_params(message: impl Into<String>) -> Error {
    Error {
        code: ErrorCode::InvalidParams,
        message: message.into().into(),
        data: None,
    }
}

/// A `ContentModified` error: the client's version is not the server's.
pub fn content_modified(expected: i32, held: i32) -> Error {
    Error {
        code: ErrorCode::ContentModified,
        message: format!("document is at version {held}, not {expected}").into(),
        data: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_one_object_argument() {
        let args: AddAlternativeArgs = single_argument(
            ADD_ALTERNATIVE,
            vec![json!({
                "uri": "file:///a.md",
                "range": {"start": {"line": 0, "character": 1}, "end": {"line": 0, "character": 4}},
                "text": "x",
                "kind": "sentence"
            })],
        )
        .unwrap();
        assert_eq!(args.kind, Some(SpanKind::Sentence));
        assert_eq!(args.version, None);
    }

    #[test]
    fn rejects_wrong_arity_and_shapes() {
        let none = single_argument::<AddAlternativeArgs>(ADD_ALTERNATIVE, vec![]);
        assert_eq!(none.unwrap_err().code, ErrorCode::InvalidParams);
        let bad = single_argument::<AddAlternativeArgs>(ADD_ALTERNATIVE, vec![json!({"uri": 3})]);
        assert_eq!(bad.unwrap_err().code, ErrorCode::InvalidParams);
    }
}
