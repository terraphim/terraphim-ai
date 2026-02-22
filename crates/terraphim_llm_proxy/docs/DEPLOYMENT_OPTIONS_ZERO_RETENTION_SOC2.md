# Deployment Options: Zero Management, Zero Data Retention, SOC 2 Readiness

**Project**: Terraphim LLM Proxy
**Status**: Draft
**Date**: 2026-02-02

## Executive Summary

- Best "zero management" option (no infra): use a managed LLM gateway (e.g., Cloudflare AI Gateway) with request/response logging disabled, plus upstream Zero Data Retention (ZDR) where available.
- Best "managed, but you run the proxy binary" option: deploy this proxy as a container to a serverless container platform (e.g., GCP Cloud Run / Azure Container Apps) and explicitly disable/exclude logs that could retain customer content.
- Caution: some platforms have low max request timeouts that are a poor fit for SSE streaming + long-running LLM requests (verify platform limits before committing).

## Multiple Interpretations (Needs Confirmation)

### "Zero data retention" could mean:

- **A)** no prompt/response storage anywhere (proxy + platform logs + upstream LLM provider), or
- **B)** proxy stores no prompt/response (but platform metadata logs may exist), or
- **C)** no customer-content retention, but allow minimal ops/security metadata retention (request id, status, latency).

### "Zero management" could mean:

- **A)** you don't operate any runtime (use a managed gateway), or
- **B)** you deploy a container/serverless service but don't manage servers.

## Current State (What Exists Today)

- This repo ships a long-running Rust HTTP service (supports SSE streaming).
- The included production deployment approach is "systemd + reverse proxy + file logs" (`DEPLOYMENT_GUIDE.md`), which is not "zero management" and is not "zero retention" if logs are kept on disk.

## Vital Few Constraints

- Must support streaming/SSE and long-running requests (the sample production config uses 10-minute timeouts).
- Must have explicit controls preventing request/response bodies from being stored (proxy logs, platform logs, observability tools).
- Must use vendors with SOC 2 reports available (for your SOC 2 program evidence and vendor risk management).

## Deployment Options (Ranked)

### 1) Managed Gateway (True "Zero Management"): Cloudflare AI Gateway

- **Pros**: no servers/containers to run; can disable gateway logging; some providers offer ZDR programs for eligible accounts.
- **Cons**: by default, gateways may log prompts/responses unless disabled; you are adding a third-party processor in the data path; ZDR behavior varies by provider and commercial terms.
- **SOC 2**: Cloudflare provides SOC 2 reporting (confirm current scope and controls).

### 2) Serverless Containers (Low Ops): Google Cloud Run (run this proxy)

- **Pros**: minimal ops; long request timeouts possible (good for streaming/long calls); simple autoscaling.
- **Data retention control**: configure logs to avoid storing request/response content (e.g., avoid printing bodies; exclude logs/disable sinks where possible).
- **SOC 2**: Google Cloud provides SOC 2 reporting.
- **Cons**: you still own build/deploy, secrets, configuration, and runtime security hardening.

### 3) Serverless Containers (Low Ops): Azure Container Apps (run this proxy)

- **Pros**: minimal ops; supports containerized services with managed ingress and autoscaling.
- **SOC 2**: Azure provides SOC 2 reporting.
- **Cons**: confirm SSE behavior and ingress/request timeout limits under your expected usage.

### 4) Managed Compute (More Ops, Strong Streaming Fit): AWS ECS Fargate + ALB (run this proxy)

- **Pros**: mature networking controls; typically a good fit for long-lived streaming; strong IAM and audit/logging controls.
- **Cons**: more moving parts than Cloud Run / ACA (ALB, ECS service/task definitions, scaling policy).
- **"Zero retention" upstream**: easiest to pair with Amazon Bedrock if you want provider-side "no prompt retention" guarantees (confirm based on current Bedrock documentation/terms).

### 5) Edge Runtime (Likely Requires Rewrite): Cloudflare Workers

- **Pros**: very low ops; edge proximity.
- **Cons**: request CPU/runtime constraints and execution model make it a non-trivial port from a Rust server binary; confirm feasibility before planning.

## Upstream "Zero Retention" (End-to-End) Reality Check

Even if the proxy retains nothing, the upstream LLM provider may retain data unless you choose/configure them accordingly. Examples (verify current terms/policies for your account tier):

- OpenAI API: API data retention defaults exist; Zero Data Retention may be available for eligible customers.
- Anthropic API: retention defaults exist; "zero retention" may be available under agreement.
- Amazon Bedrock: commonly described as not storing prompts/completions for model training; confirm exact wording and scope.
- Azure OpenAI / Azure AI: monitoring/abuse policies vary; confirm whether content logging can be disabled for your tenancy.

## Recommendation (Provisional)

- If you truly want "zero management": evaluate replacing the proxy deployment with a managed LLM gateway (and explicitly disable logging; use upstream ZDR where applicable).
- If you want "you run the proxy, but no servers": containerize and deploy to a serverless container platform (Cloud Run is a common fit) and explicitly enforce no-content logging at every layer (application logs, platform logs, APM).

## Open Questions (Answering These Determines the Best Option)

1. Does "zero retention" mean "no prompt/response stored anywhere," including upstream LLMs, or only "proxy doesn't store content"?
2. Do you require full Anthropic `/v1/messages` compatibility for Claude Code, or is OpenAI-compatible `/v1/chat/completions` enough?
3. Preferred cloud (AWS/GCP/Azure/Cloudflare) and any data residency constraints?
4. What maximum streaming duration do you need in practice (2 min / 10 min / 30+ min)?

## Notes / Next Validation Steps

- Confirm platform request/connection timeout limits for SSE, plus any proxy/load balancer defaults in front of the service.
- Confirm which log streams (app stdout/stderr, access logs, gateway logs, APM traces) could include customer content, and disable/sanitize accordingly.
- Confirm upstream provider retention terms and ZDR availability for the exact contracts you plan to use.

