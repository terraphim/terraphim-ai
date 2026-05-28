# Validation Report: Issue #1881 Badlogic Pi CLI Support

## User-Visible Behaviour

ADF agent configurations can now target the upstream `badlogic/pi` CLI by using `cli_command = "pi"` with a configured model alias. The spawned command shape is:

```text
pi prompt <model-alias> <task>
```

Existing `pi-rust` configurations continue to spawn using:

```text
pi-rust -p --mode json [--provider <provider> --model <model>] <task>
```

## Acceptance Criteria

| Acceptance Criterion | Evidence | Result |
| --- | --- | --- |
| `pi` and `pi-rust` are not conflated | Separate `AgentConfig::infer_args` and `AgentConfig::model_args` branches are covered by tests. | Pass |
| Badlogic `pi` receives model alias before prompt body | `test_spawn_pi_receives_prompt_model_and_task` spawns a temporary real executable named `pi` and captures argv as `prompt`, `phi3`, `What is 2+2?`. | Pass |
| Missing badlogic `pi` model alias is rejected before spawn | `test_validate_pi_requires_model_alias` passes. | Pass |
| ADF stage evidence is issue-scoped at runtime | `adf-ctl --local trigger ... --direct --context 'issue=1881 ...'` dispatched review evidence without hardcoded issue ID in the task script. | Pass |
| Implementation quality gates pass | Formatting, tests, clippy, coverage, and UBS assessment completed. | Pass |

## Validation Notes

The real process test is the key validation evidence because it exercises the same argv-building path used by `AgentSpawner::spawn_process` and does not use mocks. It proves that a `badlogic/pi` agent receives the expected positional argument order.

The ADF proof harness also validates the workflow requirement: stage evidence must be driven by direct dispatch context and not by hardcoded issue IDs. A missing `issue=<number>` now fails fast with a clear error from `.terraphim/bin/adf-e2e-stage`.

## Residual Risks

The test uses a temporary executable named `pi` rather than installing upstream `@mariozechner/pi`. This is intentional: the acceptance target is Terraphim's command construction contract, not the external tool's runtime availability or model-serving behaviour.

The expanded `.terraphim/adf.toml` includes local absolute paths for this proof environment. That is suitable for this local ADF proof branch, but should be revisited before making it a portable template.

## Verdict

Validation passed. The change delivers the requested `badlogic/pi` support, keeps `pi-rust` stable, and demonstrates the issue-scoped ADF evidence flow requested for #1881.
