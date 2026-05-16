## Summary

<!-- Describe what this PR does and why -->

## Test plan

- [ ] `cargo test --workspace` passes locally
- [ ] `cargo fmt --all -- --check` passes
- [ ] `cargo clippy --workspace --all-targets` clean
- [ ] Relevant integration tests verified

## Ranking change ACK

<!-- Complete this section ONLY if ranking snapshot files were updated -->

- [ ] **Ranking change ACK** -- I ran `UPDATE_RANKING_SNAPSHOTS=1 cargo test -p terraphim_service ranking_regression` locally, reviewed the snapshot diff, and confirm this is an intentional ranking change

<!-- If ranking snapshots are unchanged, leave the checkbox unchecked and delete this note -->

## Related issues

<!-- Refs #ISSUE or Fixes #ISSUE -->
