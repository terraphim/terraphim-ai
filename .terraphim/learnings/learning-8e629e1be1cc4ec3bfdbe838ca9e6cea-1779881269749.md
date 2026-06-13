---
id: 8e629e1be1cc4ec3bfdbe838ca9e6cea-1779881269749
command: rch exec -- cargo test -p terraphim_orchestrator --lib 2>&1 | tail -10
exit_code: 1
source: Project
captured_at: 2026-05-27T11:27:49.749929299+00:00
working_dir: /home/alex/projects/terraphim/terraphim-ai
tags:
  - learning
  - exit-1
entities:
  - exit_classes
importance_total: 0.5900
importance_severity: 0.3000
importance_repetition: 47
importance_recency: 1.0000
importance_has_correction: false
---

## Command

`rch exec -- cargo test -p terraphim_orchestrator --lib 2>&1 | tail -10`

## Error Output

```

failures:
    tests::test_orchestrator_compound_review_manual

test result: FAILED. 791 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.24s

[1m[91merror[0m: test failed, to rerun pass `-p terraphim_orchestrator --lib`
  [2m2026-05-27T11:27:49.694916Z[0m [32m INFO[0m [1;32mrch::hook[0m[32m: [32mRemote command finished: exit=101 in 5491ms[0m
    [2;3mat[0m rch/src/hook.rs:5079 [2;3mon[0m ThreadId(1)


```

