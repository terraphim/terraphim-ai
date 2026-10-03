# Bedrock Feature Gate

The `dlt-bedrock-client` crate gates the AWS Bedrock integration behind an optional Cargo feature (`bedrock`), so the default build compiles without the AWS SDK and routes LLM calls through OpenRouter instead. This keeps pilot/preview environments light and key-based while reserving Bedrock (data-residency path) for FOC deployments, which will enable `features = ["bedrock"]` in deploy manifests. OpenRouter live tests run with `OPENROUTER_API_KEY` from OdiloVault; live Bedrock discovery tests stay `--include-ignored` until IAM is available.

synonyms:: bedrock feature gate, dlt-bedrock-client, bedrock client, openrouter fallback, openrouter integration, bedrock feature flag

## Related Concepts
- Embedding Pipeline
- Digital Learning Twin
- Sidecar Deployment

## Sources
- `.agent/handoffs/2026-07-10-oditech-778-bedrock-feature-gate.md` (ODITECH-778, PR #862)
- `rust/crates/dlt-bedrock-client/README.md`
