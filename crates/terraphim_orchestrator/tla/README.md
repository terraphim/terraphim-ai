# ADF Orchestrator TLA+ Models

These bounded specifications are refactor guardrails for the OTP orchestrator redesign.

- `AdfRegistry.tla`: project-scoped agent registry merge and lookup invariants.
- `AdfBuildDispatch.tla`: deterministic `adf/build` pending and terminal status lifecycle.
- `AdfRunSupervisor.tla`: retry, escalation, terminal status, and active-run release invariants.
- `AdfProviderHealthActor.tla`: provider probe isolation from lifecycle processing.

They intentionally model ADF at the message/state-transition boundary rather than at Rust `await` points. The Rust test harness treats TLC execution as optional locally until the `tlaplus-ts`/TLC toolchain is installed in CI.
