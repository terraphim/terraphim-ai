# ADF Flow Verification and Validation Evidence

Issue: 1882
Date: 2026-05-29 11:43 BST
Scope: prove `adf-ctl flow` can execute useful local work, not just dispatch or comment.

## Verification

### Design Elements Checked

| Design element | Evidence | Status |
|---|---|---|
| Flow discovery from `.terraphim/flows/<name>.toml` | `adf-ctl flow adf-useful-work-proof --context "issue=1882"` loaded `.terraphim/flows/adf-useful-work-proof.toml` | PASS |
| Context substitution | Output artefact contains `Issue: 1882` and each slot file contains `issue=1882` | PASS |
| Matrix execution | Flow state contains three `matrix-slot-work` envelopes with exit code `0` | PASS |
| Matrix aggregate substitution | Proof artefact contains `Matrix successes: 3` and `Matrix exit codes: 0,0,0` | PASS |
| Numeric gate evaluation | `require-all-slots` gate passed after `success_count >= 3` | PASS |
| Durable artefact production | `.docs/adf/1882/adf-flow-useful-work-proof.md` was written by the flow | PASS |

### Commands Run

```bash
cargo run -p terraphim_orchestrator --bin adf-ctl -- flow adf-useful-work-proof --context "issue=1882"
ubs 'crates/terraphim_orchestrator/src/flow/executor.rs' 'crates/terraphim_orchestrator/src/lib.rs' '.terraphim/bin/adf-issue-stage'
rch exec -- cargo test -p terraphim_orchestrator flow::executor
bash -n '.terraphim/bin/adf-issue-stage'
git diff --check -- 'crates/terraphim_orchestrator/src/flow/executor.rs' 'crates/terraphim_orchestrator/src/lib.rs' '.terraphim/bin/adf-issue-stage' '.terraphim/flows/adf-useful-work-proof.toml' '.docs/adf/1882/research-proposal-3.md' '.docs/adf/1882/adf-flow-useful-work-proof.md'
rch exec -- cargo llvm-cov -p terraphim_orchestrator --lib --summary-only -- flow::executor
```

### Results

| Check | Result |
|---|---|
| `adf-ctl flow` execution | PASS, flow status `Completed` |
| UBS | PASS for critical findings: `Critical issues: 0` |
| Targeted tests | PASS, 31 flow executor tests passed |
| Shell syntax | PASS |
| Diff whitespace | PASS |
| Coverage | `flow/executor.rs` line coverage: 83.83% |

## Validation

### Acceptance Scenario

**Scenario:** A user wants to prove that local ADF flow execution can perform useful project work without invoking model agents or external services.

Steps:

1. Run `cargo run -p terraphim_orchestrator --bin adf-ctl -- flow adf-useful-work-proof --context "issue=1882"`.
2. Confirm the CLI loads `.terraphim/flows/adf-useful-work-proof.toml`.
3. Confirm the CLI reports `Flow 'adf-useful-work-proof' finished: Completed`.
4. Inspect `.docs/adf/1882/adf-flow-useful-work-proof.md`.
5. Inspect `.terraphim/flow-state/flow-adf-useful-work-proof-5a1a8ef8-0b87-4a32-a4ca-856a9679037f.json`.

Expected outcome:

- The flow produces a durable Markdown artefact.
- The artefact proves matrix slot execution, aggregate substitution, and issue-context propagation.
- The persisted flow state proves the gate and action steps completed successfully.

Actual outcome:

- `.docs/adf/1882/adf-flow-useful-work-proof.md` exists and contains three slot outputs.
- Matrix success count is `3`.
- Matrix exit codes are `0,0,0`.
- Flow state status is `completed`.
- Gate step `require-all-slots` has exit code `0`.
- Final action step `assemble-proof` has exit code `0`.

Validation decision: PASS.

## Limitations

- This proof intentionally avoids live model calls. It proves the flow engine and CLI can do useful deterministic work; it does not prove that each configured model provider can successfully complete a research/design prompt.
- Matrix execution is currently sequential. The proof validates correctness, not parallel throughput.
- The proof flow is a local validation artefact. Production k=3 research/design flow still needs provider availability checks before a full live run.
