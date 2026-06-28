//! Integration tests: parse canned cargo JSON output (no subprocess).
//!
//! These tests exercise the public parser API against fixture strings that
//! mirror the real cargo `--message-format=json` schema. They run in
//! milliseconds and require no cargo invocation.

#![allow(clippy::needless_raw_string_hashes)]

use terraphim_eval::{MetricCounts, parse_clippy_json, parse_test_json};

const CLIPPY_OUTPUT: &str = r##"{"reason":"compiler-artifact","target":{"name":"foo","kind":["lib"]}}
{"reason":"compiler-message","message":{"level":"warning","message":"unused variable: `x`"}}
{"reason":"compiler-message","message":{"level":"error","message":"cannot find value `y`"}}
{"reason":"build-script-executed"}
"##;

const TEST_OUTPUT: &str = r##"{"type":"suite","event":"started"}
{"type":"test","event":"ok","name":"tests::a"}
{"type":"test","event":"ok","name":"tests::b"}
{"type":"test","event":"failed","name":"tests::c"}
{"type":"test","event":"ignored","name":"tests::d"}
{"type":"suite","event":"ok"}
"##;

#[test]
fn parse_clippy_fixture_counts_correctly() {
    let counts = parse_clippy_json(CLIPPY_OUTPUT).unwrap();
    assert_eq!(
        counts,
        MetricCounts {
            warnings: 1,
            errors: 1,
            passed: 0,
            failed: 0,
            ignored: 0,
        }
    );
}

#[test]
fn parse_test_fixture_counts_correctly() {
    let counts = parse_test_json(TEST_OUTPUT).unwrap();
    assert_eq!(
        counts,
        MetricCounts {
            warnings: 0,
            errors: 0,
            passed: 2,
            failed: 1,
            ignored: 1,
        }
    );
}

#[test]
fn parse_clippy_empty_yields_zero_counts() {
    let counts = parse_clippy_json("").unwrap();
    assert_eq!(counts, MetricCounts::default());
}

#[test]
fn parse_test_empty_yields_zero_counts() {
    let counts = parse_test_json("").unwrap();
    assert_eq!(counts, MetricCounts::default());
}

#[test]
fn metric_record_serializes_roundtrip() {
    use terraphim_eval::{MetricRecord, PassFail};
    let record = MetricRecord::new(
        "clippy",
        "cargo-clippy",
        MetricCounts {
            warnings: 3,
            errors: 0,
            ..Default::default()
        },
        Some(101),
        None,
    );
    let json = serde_json::to_string(&record).unwrap();
    let back: MetricRecord = serde_json::from_str(&json).unwrap();
    assert_eq!(record.metric_id, back.metric_id);
    assert_eq!(record.tool, back.tool);
    assert_eq!(record.counts, back.counts);
    assert_eq!(record.pass_fail, PassFail::Pass);
}

#[test]
fn metric_record_with_errors_is_classified_fail() {
    use terraphim_eval::{MetricRecord, PassFail};
    let record = MetricRecord::new(
        "test",
        "cargo-test",
        MetricCounts {
            failed: 1,
            ..Default::default()
        },
        Some(101),
        None,
    );
    assert_eq!(record.pass_fail, PassFail::Fail);
}
