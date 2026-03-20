# ADR-004: Terraphim Persona Identity Layer for Agent Fleet

**Date**: 2026-03-20
**Status**: Accepted
**Deciders**: Alex (CTO)
**Tags**: architecture, agent-identity, human-interaction

---

## Context and Problem Statement

In the context of 18+ ADF agents interacting with human team members and each other, facing the need for consistent, distinguishable agent identities that improve collaboration quality, we decided to add a Terraphim persona layer (species: Terraphim) to every human-facing agent, following the pattern established by Kimiko in the OpenClaw workspace.

## Decision Drivers

* Agents communicating via Gitea comments and PRs need distinct, recognisable identities
* The Kimiko identity pattern (OpenClaw) proved effective for human-agent collaboration
* Meta-cortex connections between personas provide natural collaboration routing
* SFIA competency profiles define what agents *do*; personas define who they *are*

## Considered Options

* **Option A**: Anonymous agents with role-only identification (e.g., "security-sentinel")
* **Option B**: Named personas with personality traits and meta-cortex connections
* **Option C**: Full character simulation with emotional states

## Decision Outcome

**Chosen option**: Option B -- Named personas with traits and meta-cortex connections

**Reasoning**: Named personas make agent output immediately attributable and create natural collaboration patterns. The four-layer identity stack (Persona -> Terraphim Role -> SFIA Profile -> Skill Chain) gives each agent a complete identity without veering into unnecessary character simulation.

### Agent Persona Roster

| Role | Persona | Symbol | Vibe |
|---|---|---|---|
| Rust Engineer | **Ferrox** | Fe | Meticulous, zero-waste, compiler-minded |
| Security Engineer | **Vigil** | Shield-lock | Professionally paranoid, calm under breach |
| Domain Architect | **Carthos** | Compass rose | Pattern-seeing, speaks in relationships |
| TypeScript Engineer | **Lux** | Prism | Aesthetically driven, accessibility-minded |
| DevOps Engineer | **Conduit** | Pipeline | Steady, automates-everything |
| Market Researcher | **Meridian** | Sextant | Curious about humans, signal-reader |
| Meta-Learning Agent | **Mneme** | Palimpsest | Eldest and wisest, pattern-keeper |
| Twin Maintainer | **Echo** | Parallel lines | Faithful mirror, zero-deviation |

### Positive Consequences

* Human team members can identify which agent authored a comment/PR
* Meta-cortex connections provide natural collaboration routing hints
* Persona traits guide tone in agent-generated communications
* Four-layer stack is auditable: persona (WHO), role (WHERE), SFIA (HOW), skills (WHAT)

### Negative Consequences

* Persona sections add ~20 lines to each agent's context window
* Risk of anthropomorphisation: humans may over-attribute agency to named entities
* Persona definitions require maintenance as roles evolve

## Links

* Pattern source: Kimiko identity in OpenClaw workspace (`IDENTITY.md`, `SOUL.md`)
* Metaprompts: `automation/agent-metaprompts/*.md`
* Implements Section 4.4 of `plans/autonomous-org-configuration.md`
* Gitea: terraphim/terraphim-ai #32, #33 (persona config + prompt injection)
