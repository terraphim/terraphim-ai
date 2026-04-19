//! Ground-truth evaluation framework for automata-based classification accuracy.
//!
//! Computes precision, recall, and F1 per term and overall by comparing
//! `find_matches()` output against human-labeled ground truth documents.
//! Also detects systematic errors (terms that consistently produce false positives).

use std::collections::{HashMap, HashSet};
use std::path::Path;

use serde::{Deserialize, Serialize};
use terraphim_types::Thesaurus;

use crate::matcher::find_matches;

/// A single document with known ground-truth labels.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroundTruthDocument {
    /// Unique identifier for the document.
    pub id: String,
    /// The text content to run through the automata.
    pub text: String,
    /// The terms expected to be found by the automata.
    pub expected_terms: Vec<ExpectedMatch>,
}

/// A term expected to be found in a document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpectedMatch {
    /// The normalized term value expected to match.
    pub term: String,
    /// Optional category for grouping (reserved for future per-category metrics).
    pub category: Option<String>,
}

/// Precision, recall, and F1 metrics computed from true/false positive/negative counts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassificationMetrics {
    pub precision: f64,
    pub recall: f64,
    pub f1: f64,
    pub true_positives: usize,
    pub false_positives: usize,
    pub false_negatives: usize,
}

/// Metrics for a single term across all evaluated documents.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TermReport {
    pub term: String,
    pub metrics: ClassificationMetrics,
}

/// Full evaluation result covering all documents and terms.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluationResult {
    pub total_documents: usize,
    pub overall: ClassificationMetrics,
    pub per_term: Vec<TermReport>,
    pub systematic_errors: Vec<SystematicError>,
}

/// A term that consistently appears as a false positive across multiple documents.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystematicError {
    pub term: String,
    pub false_positive_count: usize,
    pub document_ids: Vec<String>,
}

/// Minimum number of false-positive occurrences before a term is flagged as a systematic error.
const SYSTEMATIC_ERROR_THRESHOLD: usize = 2;

/// Compute precision, recall, and F1 from raw counts.
fn compute_metrics(tp: usize, fp: usize, fn_count: usize) -> ClassificationMetrics {
    let precision = if tp + fp > 0 {
        tp as f64 / (tp + fp) as f64
    } else {
        0.0
    };
    let recall = if tp + fn_count > 0 {
        tp as f64 / (tp + fn_count) as f64
    } else {
        0.0
    };
    let f1 = if precision + recall > 0.0 {
        2.0 * precision * recall / (precision + recall)
    } else {
        0.0
    };
    ClassificationMetrics {
        precision,
        recall,
        f1,
        true_positives: tp,
        false_positives: fp,
        false_negatives: fn_count,
    }
}

/// Evaluate automata classification accuracy against ground truth.
///
/// Runs `find_matches()` on each document's text using the provided thesaurus,
/// then compares matched normalized term values against expected terms.
///
/// Returns overall micro-averaged metrics, per-term metrics, and systematic errors.
pub fn evaluate(ground_truth: &[GroundTruthDocument], thesaurus: Thesaurus) -> EvaluationResult {
    let total_documents = ground_truth.len();

    // Accumulators for overall (micro-averaged) metrics
    let mut total_tp: usize = 0;
    let mut total_fp: usize = 0;
    let mut total_fn: usize = 0;

    // Per-term accumulators: term -> (tp, fp, fn)
    let mut per_term_counts: HashMap<String, (usize, usize, usize)> = HashMap::new();

    // Track false positives per term with the document IDs where they occurred
    let mut fp_by_term: HashMap<String, Vec<String>> = HashMap::new();

    for doc in ground_truth {
        // Run find_matches with a clone of the thesaurus (find_matches takes ownership)
        let matches = find_matches(&doc.text, thesaurus.clone(), false).unwrap_or_default();

        // Collect the normalized term values from the matched results.
        // Use the nterm value (normalized_term.value) since that is what the
        // thesaurus normalizes to, and deduplicate to count each term once per doc.
        let matched_nterms: HashSet<String> = matches
            .iter()
            .map(|m| m.normalized_term.value.as_str().to_string())
            .collect();

        // Build the expected set from ground truth
        let expected_nterms: HashSet<String> =
            doc.expected_terms.iter().map(|e| e.term.clone()).collect();

        // True positives: in both matched and expected
        let tp_set: HashSet<&String> = matched_nterms.intersection(&expected_nterms).collect();
        // False positives: in matched but not expected
        let fp_set: HashSet<&String> = matched_nterms.difference(&expected_nterms).collect();
        // False negatives: in expected but not matched
        let fn_set: HashSet<&String> = expected_nterms.difference(&matched_nterms).collect();

        let doc_tp = tp_set.len();
        let doc_fp = fp_set.len();
        let doc_fn = fn_set.len();

        total_tp += doc_tp;
        total_fp += doc_fp;
        total_fn += doc_fn;

        // Update per-term counts
        for term in &tp_set {
            let entry = per_term_counts.entry((**term).clone()).or_insert((0, 0, 0));
            entry.0 += 1;
        }
        for term in &fp_set {
            let entry = per_term_counts.entry((**term).clone()).or_insert((0, 0, 0));
            entry.1 += 1;
            fp_by_term
                .entry((**term).clone())
                .or_default()
                .push(doc.id.clone());
        }
        for term in &fn_set {
            let entry = per_term_counts.entry((**term).clone()).or_insert((0, 0, 0));
            entry.2 += 1;
        }
    }

    let overall = compute_metrics(total_tp, total_fp, total_fn);

    let mut per_term: Vec<TermReport> = per_term_counts
        .into_iter()
        .map(|(term, (tp, fp, fn_count))| TermReport {
            term,
            metrics: compute_metrics(tp, fp, fn_count),
        })
        .collect();

    // Sort per-term reports by term name for deterministic output
    #[allow(clippy::unnecessary_sort_by)]
    per_term.sort_by(|a, b| a.term.cmp(&b.term));

    // Detect systematic errors: terms with false-positive count >= threshold
    let mut systematic_errors: Vec<SystematicError> = fp_by_term
        .into_iter()
        .filter(|(_, doc_ids)| doc_ids.len() >= SYSTEMATIC_ERROR_THRESHOLD)
        .map(|(term, document_ids)| SystematicError {
            false_positive_count: document_ids.len(),
            term,
            document_ids,
        })
        .collect();

    // Sort systematic errors by term name for deterministic output
    #[allow(clippy::unnecessary_sort_by)]
    systematic_errors.sort_by(|a, b| a.term.cmp(&b.term));

    EvaluationResult {
        total_documents,
        overall,
        per_term,
        systematic_errors,
    }
}

/// Load ground truth documents from a JSON file.
///
/// The file must contain a JSON array of `GroundTruthDocument` objects.
pub fn load_ground_truth(
    path: &Path,
) -> std::result::Result<Vec<GroundTruthDocument>, std::io::Error> {
    let content = std::fs::read_to_string(path)?;
    serde_json::from_str(&content)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use terraphim_types::{NormalizedTerm, NormalizedTermValue, Thesaurus};

    /// Helper: build a thesaurus from a list of (pattern, nterm) pairs.
    ///
    /// The pattern is the key that the Aho-Corasick automaton matches in text.
    /// The nterm is the normalized term value stored in NormalizedTerm.value.
    fn build_test_thesaurus(entries: &[(&str, &str)]) -> Thesaurus {
        let mut thesaurus = Thesaurus::new("test".to_string());
        for (i, (pattern, nterm)) in entries.iter().enumerate() {
            let term = NormalizedTerm::new((i + 1) as u64, NormalizedTermValue::from(*nterm));
            thesaurus.insert(NormalizedTermValue::from(*pattern), term);
        }
        thesaurus
    }

    #[test]
    fn test_evaluate_perfect_match() {
        // Thesaurus: "rust" -> nterm "rust", "async" -> nterm "async"
        let thesaurus = build_test_thesaurus(&[("rust", "rust"), ("async", "async")]);

        let ground_truth = vec![GroundTruthDocument {
            id: "doc1".to_string(),
            text: "I love rust and async programming".to_string(),
            expected_terms: vec![
                ExpectedMatch {
                    term: "rust".to_string(),
                    category: None,
                },
                ExpectedMatch {
                    term: "async".to_string(),
                    category: None,
                },
            ],
        }];

        let result = evaluate(&ground_truth, thesaurus);

        assert_eq!(result.total_documents, 1);
        assert_eq!(result.overall.true_positives, 2);
        assert_eq!(result.overall.false_positives, 0);
        assert_eq!(result.overall.false_negatives, 0);
        assert!((result.overall.precision - 1.0).abs() < f64::EPSILON);
        assert!((result.overall.recall - 1.0).abs() < f64::EPSILON);
        assert!((result.overall.f1 - 1.0).abs() < f64::EPSILON);
        assert!(result.systematic_errors.is_empty());
    }

    #[test]
    fn test_evaluate_no_matches() {
        // Thesaurus has patterns that do not appear in the text
        let thesaurus = build_test_thesaurus(&[("python", "python"), ("java", "java")]);

        let ground_truth = vec![GroundTruthDocument {
            id: "doc1".to_string(),
            text: "I love rust and async programming".to_string(),
            expected_terms: vec![
                ExpectedMatch {
                    term: "rust".to_string(),
                    category: None,
                },
                ExpectedMatch {
                    term: "async".to_string(),
                    category: None,
                },
            ],
        }];

        let result = evaluate(&ground_truth, thesaurus);

        assert_eq!(result.overall.true_positives, 0);
        assert_eq!(result.overall.false_positives, 0);
        assert_eq!(result.overall.false_negatives, 2);
        assert!((result.overall.precision - 0.0).abs() < f64::EPSILON);
        assert!((result.overall.recall - 0.0).abs() < f64::EPSILON);
        assert!((result.overall.f1 - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_evaluate_partial_match() {
        // Thesaurus has "rust" but not "async"
        let thesaurus = build_test_thesaurus(&[("rust", "rust")]);

        let ground_truth = vec![GroundTruthDocument {
            id: "doc1".to_string(),
            text: "I love rust and async programming".to_string(),
            expected_terms: vec![
                ExpectedMatch {
                    term: "rust".to_string(),
                    category: None,
                },
                ExpectedMatch {
                    term: "async".to_string(),
                    category: None,
                },
            ],
        }];

        let result = evaluate(&ground_truth, thesaurus);

        assert_eq!(result.overall.true_positives, 1);
        assert_eq!(result.overall.false_positives, 0);
        assert_eq!(result.overall.false_negatives, 1);
        // precision = 1/1 = 1.0
        assert!((result.overall.precision - 1.0).abs() < f64::EPSILON);
        // recall = 1/2 = 0.5
        assert!((result.overall.recall - 0.5).abs() < f64::EPSILON);
        // f1 = 2*1.0*0.5/(1.0+0.5) = 2/3
        let expected_f1 = 2.0 / 3.0;
        assert!((result.overall.f1 - expected_f1).abs() < 1e-10);
    }

    #[test]
    fn test_evaluate_false_positives() {
        // Thesaurus matches "rust" and "love" but ground truth only expects "rust"
        let thesaurus = build_test_thesaurus(&[("rust", "rust"), ("love", "love")]);

        let ground_truth = vec![GroundTruthDocument {
            id: "doc1".to_string(),
            text: "I love rust programming".to_string(),
            expected_terms: vec![ExpectedMatch {
                term: "rust".to_string(),
                category: None,
            }],
        }];

        let result = evaluate(&ground_truth, thesaurus);

        assert_eq!(result.overall.true_positives, 1);
        assert_eq!(result.overall.false_positives, 1);
        assert_eq!(result.overall.false_negatives, 0);
        // precision = 1/2 = 0.5
        assert!((result.overall.precision - 0.5).abs() < f64::EPSILON);
        // recall = 1/1 = 1.0
        assert!((result.overall.recall - 1.0).abs() < f64::EPSILON);
        // f1 = 2*0.5*1.0/(0.5+1.0) = 2/3
        let expected_f1 = 2.0 / 3.0;
        assert!((result.overall.f1 - expected_f1).abs() < 1e-10);
    }

    #[test]
    fn test_evaluate_systematic_errors() {
        // "the" is a common word that will match in many documents but is never expected
        let thesaurus =
            build_test_thesaurus(&[("rust", "rust"), ("the", "the"), ("async", "async")]);

        let ground_truth = vec![
            GroundTruthDocument {
                id: "doc1".to_string(),
                text: "the rust language is great".to_string(),
                expected_terms: vec![ExpectedMatch {
                    term: "rust".to_string(),
                    category: None,
                }],
            },
            GroundTruthDocument {
                id: "doc2".to_string(),
                text: "the async runtime is powerful".to_string(),
                expected_terms: vec![ExpectedMatch {
                    term: "async".to_string(),
                    category: None,
                }],
            },
            GroundTruthDocument {
                id: "doc3".to_string(),
                text: "the compiler catches errors at compile time".to_string(),
                expected_terms: vec![],
            },
        ];

        let result = evaluate(&ground_truth, thesaurus);

        // "the" should be flagged as a systematic error (FP in all 3 docs)
        assert_eq!(result.systematic_errors.len(), 1);
        let error = &result.systematic_errors[0];
        assert_eq!(error.term, "the");
        assert_eq!(error.false_positive_count, 3);
        assert_eq!(error.document_ids.len(), 3);
    }

    #[test]
    fn test_evaluate_empty_ground_truth() {
        let thesaurus = build_test_thesaurus(&[("rust", "rust")]);
        let ground_truth: Vec<GroundTruthDocument> = vec![];

        let result = evaluate(&ground_truth, thesaurus);

        assert_eq!(result.total_documents, 0);
        assert_eq!(result.overall.true_positives, 0);
        assert_eq!(result.overall.false_positives, 0);
        assert_eq!(result.overall.false_negatives, 0);
        assert!((result.overall.precision - 0.0).abs() < f64::EPSILON);
        assert!((result.overall.recall - 0.0).abs() < f64::EPSILON);
        assert!((result.overall.f1 - 0.0).abs() < f64::EPSILON);
        assert!(result.per_term.is_empty());
        assert!(result.systematic_errors.is_empty());
    }

    #[test]
    fn test_evaluate_per_term_metrics() {
        // Two documents, each expecting different terms
        let thesaurus =
            build_test_thesaurus(&[("rust", "rust"), ("async", "async"), ("tokio", "tokio")]);

        let ground_truth = vec![
            GroundTruthDocument {
                id: "doc1".to_string(),
                text: "rust and async are great together".to_string(),
                expected_terms: vec![
                    ExpectedMatch {
                        term: "rust".to_string(),
                        category: None,
                    },
                    ExpectedMatch {
                        term: "async".to_string(),
                        category: None,
                    },
                ],
            },
            GroundTruthDocument {
                id: "doc2".to_string(),
                text: "tokio powers async rust".to_string(),
                expected_terms: vec![
                    ExpectedMatch {
                        term: "tokio".to_string(),
                        category: None,
                    },
                    ExpectedMatch {
                        term: "rust".to_string(),
                        category: None,
                    },
                    ExpectedMatch {
                        term: "async".to_string(),
                        category: None,
                    },
                ],
            },
        ];

        let result = evaluate(&ground_truth, thesaurus);

        // All terms should be found perfectly
        assert_eq!(result.overall.true_positives, 5);
        assert_eq!(result.overall.false_positives, 0);
        assert_eq!(result.overall.false_negatives, 0);

        // Verify per-term reports exist for all 3 terms
        assert_eq!(result.per_term.len(), 3);

        // Per-term reports are sorted by term name
        let async_report = result.per_term.iter().find(|r| r.term == "async").unwrap();
        assert_eq!(async_report.metrics.true_positives, 2);
        assert!((async_report.metrics.precision - 1.0).abs() < f64::EPSILON);

        let rust_report = result.per_term.iter().find(|r| r.term == "rust").unwrap();
        assert_eq!(rust_report.metrics.true_positives, 2);
        assert!((rust_report.metrics.precision - 1.0).abs() < f64::EPSILON);

        let tokio_report = result.per_term.iter().find(|r| r.term == "tokio").unwrap();
        assert_eq!(tokio_report.metrics.true_positives, 1);
        assert!((tokio_report.metrics.precision - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_load_ground_truth() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("ground_truth.json");

        let data = serde_json::json!([
            {
                "id": "doc1",
                "text": "hello world",
                "expected_terms": [
                    {"term": "hello", "category": null}
                ]
            },
            {
                "id": "doc2",
                "text": "foo bar baz",
                "expected_terms": [
                    {"term": "foo", "category": "test"},
                    {"term": "bar", "category": null}
                ]
            }
        ]);

        std::fs::write(&file_path, serde_json::to_string_pretty(&data).unwrap()).unwrap();

        let docs = load_ground_truth(&file_path).unwrap();
        assert_eq!(docs.len(), 2);
        assert_eq!(docs[0].id, "doc1");
        assert_eq!(docs[0].text, "hello world");
        assert_eq!(docs[0].expected_terms.len(), 1);
        assert_eq!(docs[0].expected_terms[0].term, "hello");

        assert_eq!(docs[1].id, "doc2");
        assert_eq!(docs[1].expected_terms.len(), 2);
        assert_eq!(docs[1].expected_terms[0].category, Some("test".to_string()));
    }

    #[test]
    fn test_load_ground_truth_invalid_file() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("bad.json");
        std::fs::write(&file_path, "not valid json").unwrap();

        let result = load_ground_truth(&file_path);
        assert!(result.is_err());
    }

    #[test]
    fn test_load_ground_truth_missing_file() {
        let result = load_ground_truth(Path::new("/nonexistent/path/file.json"));
        assert!(result.is_err());
    }

    #[test]
    fn test_compute_metrics_all_zero() {
        let m = compute_metrics(0, 0, 0);
        assert!((m.precision - 0.0).abs() < f64::EPSILON);
        assert!((m.recall - 0.0).abs() < f64::EPSILON);
        assert!((m.f1 - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_compute_metrics_perfect() {
        let m = compute_metrics(10, 0, 0);
        assert!((m.precision - 1.0).abs() < f64::EPSILON);
        assert!((m.recall - 1.0).abs() < f64::EPSILON);
        assert!((m.f1 - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_evaluate_case_insensitive_matching() {
        // find_matches uses case-insensitive matching; verify the evaluation
        // correctly handles this by matching "Rust" in text against "rust" nterm.
        let thesaurus = build_test_thesaurus(&[("rust", "rust")]);

        let ground_truth = vec![GroundTruthDocument {
            id: "doc1".to_string(),
            text: "I love Rust programming".to_string(),
            expected_terms: vec![ExpectedMatch {
                term: "rust".to_string(),
                category: None,
            }],
        }];

        let result = evaluate(&ground_truth, thesaurus);

        assert_eq!(result.overall.true_positives, 1);
        assert_eq!(result.overall.false_positives, 0);
        assert_eq!(result.overall.false_negatives, 0);
    }

    #[test]
    fn test_evaluate_multiple_docs_aggregation() {
        let thesaurus = build_test_thesaurus(&[("rust", "rust"), ("go lang", "go lang")]);

        let ground_truth = vec![
            GroundTruthDocument {
                id: "doc1".to_string(),
                text: "rust is great".to_string(),
                expected_terms: vec![ExpectedMatch {
                    term: "rust".to_string(),
                    category: None,
                }],
            },
            GroundTruthDocument {
                id: "doc2".to_string(),
                text: "go lang is also great".to_string(),
                expected_terms: vec![
                    ExpectedMatch {
                        term: "go lang".to_string(),
                        category: None,
                    },
                    ExpectedMatch {
                        term: "rust".to_string(),
                        category: None,
                    },
                ],
            },
        ];

        let result = evaluate(&ground_truth, thesaurus);

        // doc1: TP=1 (rust), FP=0, FN=0
        // doc2: TP=1 (go lang), FP=0, FN=1 (rust not in text)
        assert_eq!(result.overall.true_positives, 2);
        assert_eq!(result.overall.false_positives, 0);
        assert_eq!(result.overall.false_negatives, 1);
        // precision = 2/2 = 1.0, recall = 2/3
        assert!((result.overall.precision - 1.0).abs() < f64::EPSILON);
        let expected_recall = 2.0 / 3.0;
        assert!((result.overall.recall - expected_recall).abs() < 1e-10);
    }
}
