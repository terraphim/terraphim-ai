.PHONY: test test-ci

# Run the workspace test suite via cargo-nextest.
# nextest provides per-test process isolation, configurable timeouts,
# and readable parallel output. Configuration lives in .config/nextest.toml.
test:
	cargo nextest run --workspace

# CI-optimised test run (limited test threads, same timeout policy).
test-ci:
	cargo nextest run --workspace --profile ci
