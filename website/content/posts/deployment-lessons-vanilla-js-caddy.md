+++
title="Deployment Lessons: Vanilla JS UI + Caddy + 1Password"
date=2026-01-19

[taxonomies]
categories = ["Engineering", "CI", "Deployment"]
tags = ["Terraphim", "deployment", "caddy", "1password", "websocket", "javascript"]
[extra]
toc = true
comments = true
+++

A case study in not fighting the repository: reading existing scripts first, choosing boring tech for deployability, and using Caddy + 1Password patterns to ship reliably.

<!-- more -->

## The Problem

We needed to ship a simple UI quickly. The first instinct was to add new infrastructure (Docker, nginx, a frontend framework). That was the wrong move.

## The Turning Point

The repo already had a deployment pattern. The lesson was not about tooling. It was about reading and following the existing operational contract.

## The Pattern: Boring UI, Fast Deployment

- Vanilla JS means no build step and faster iteration.
- Caddy handles static hosting, HTTPS, and reverse proxying with minimal configuration.

## Hybrid Delivery: Polling + WebSocket

Polling provides reliability. WebSocket provides UX.

Combining both yields a system that still works when the real-time channel is flaky.

## Secrets: 1Password Runtime Injection

Operational secrets should not land in `.env` files committed to disk.

The pattern that scales is runtime injection via `op run`.

## References

- mdBook (canonical + appendices): https://docs.terraphim.ai/src/domains/ci/case-studies/deployment-lessons-vanilla-js-caddy.html
- Source notes: `docs/archive/root/lessons-learned.md`
