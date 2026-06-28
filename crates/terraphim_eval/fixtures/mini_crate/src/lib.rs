//! Minimal fixture crate used by the terraphim_eval integration tests.
//!
//! Deliberately tiny: one passing test, one clippy-clean function. The
//! integration test (`tests/self_eval.rs`, `#[ignore]` by default) invokes
//! `cargo clippy`/`cargo test` against this crate to exercise the live
//! subprocess path end-to-end.

#[cfg(test)]
mod tests {
    #[test]
    fn fixture_test_passes() {
        assert_eq!(2 + 2, 4);
    }
}
