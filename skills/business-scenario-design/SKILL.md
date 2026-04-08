---
name: business-scenario-design
description: Use this skill to design end-to-end business scenarios from personas, jobs-to-be-done, and domain models. Produces structured scenarios covering happy paths, exception flows, and automation boundaries aligned with ZDP Define stage.
license: Apache-2.0
---

# Business Scenario Design

## Purpose

Translate personas and jobs-to-be-done (JTBD) into actionable end-to-end business scenarios. Each scenario describes how value is created and delivered through a sequence of actor interactions, system responses, and domain entity transformations. Scenarios serve as the bridge between strategic intent (PVVH) and technical design (architecture, test plans).

## When to Use

- ZDP Define stage: deriving business scenarios from personas and JTBD
- When acceptance criteria need grounding in business context
- When mapping automation boundaries (what the system does vs. what humans do)
- Before `/acceptance-testing` to provide scenario source material
- When validating that a domain model covers all required interactions

## Inputs (Ask If Missing)

Before proceeding, gather:
- **Personas**: Who are the actors? What are their goals and constraints?
- **JTBD Pack**: What jobs are users trying to accomplish? What outcomes matter?
- **Domain Model**: What entities, relationships, and events exist in the domain?
- **Existing Process Descriptions**: How does the current (manual or automated) process work?
- **PVVH Reference**: What value hypothesis are these scenarios intended to validate?

## Workflow

1. **Identify actors and trigger events** from personas and JTBD. Each scenario starts with an actor performing a trigger action.
2. **Map the value creation flow**: trace how value is delivered from trigger to outcome, through which domain entities and system components.
3. **Define the happy path**: the primary success flow where everything works as intended.
4. **Define exception paths**: what happens when preconditions fail, actors make errors, or external dependencies are unavailable.
5. **Define failure paths**: unrecoverable failures and how the system communicates them.
6. **Mark automation boundaries**: for each step, decide whether it is automated, human-driven, or hybrid (human-in-the-loop).
7. **Trace to domain model**: every scenario step should reference specific domain entities and events.
8. **Validate completeness**: does every persona have at least one scenario? Does every critical JTBD have a scenario?

## Business Scenario Template

```markdown
# Business Scenario: {title}

**ID**: BS-{NNN}
**Date**: {YYYY-MM-DD}
**Status**: Draft | Review | Approved
**PVVH Reference**: {which value hypothesis this scenario validates}

## Actors

| Actor | Persona | Role in Scenario |
|-------|---------|------------------|
| {e.g., Warehouse Manager} | {persona name} | {initiator/approver/observer} |

## Trigger Event

{What event or action initiates this scenario}

## Pre-conditions

- {Condition 1 that must be true before the scenario can start}
- {Condition 2}

## Happy Path

| Step | Actor | Action | System Response | Domain Entities |
|------|-------|--------|-----------------|-----------------|
| 1 | {actor} | {what they do} | {what the system does} | {entities touched} |
| 2 | {actor} | {what they do} | {what the system does} | {entities touched} |

## Exception Paths

| ID | Exception | Trigger Condition | Handling | Outcome |
|----|-----------|-------------------|----------|---------|
| EX-1 | {what goes wrong} | {when/how it triggers} | {system/human response} | {result} |

## Failure Paths

| ID | Failure | Detection | User Communication | Recovery |
|----|---------|-----------|-------------------|----------|
| FL-1 | {unrecoverable failure} | {how detected} | {what user sees} | {recovery steps} |

## Automation Boundaries

| Step | Automated / Human / Hybrid | Rationale |
|------|---------------------------|-----------|
| 1 | {Automated} | {why this can be automated} |
| 2 | {Human} | {why human judgment is required} |

## Success Criteria

- {Measurable outcome 1}
- {Measurable outcome 2}

## Traceability

| Scenario Step | Domain Entity | JTBD | Persona |
|---------------|---------------|------|---------|
| Step 1 | {entity name} | {JTBD reference} | {persona name} |

## Post-conditions

- {State of the system after successful completion}
- {Notifications or side effects triggered}

## Open Questions

| # | Question | Impact | Owner |
|---|----------|--------|-------|
| 1 | {unresolved question} | {what it blocks} | {who can answer} |
```

## Validation Checklist

Before marking a scenario set as complete:
- [ ] Every persona has at least one scenario
- [ ] Every critical JTBD is covered by at least one scenario
- [ ] All happy paths are complete (trigger to outcome)
- [ ] Exception paths cover the most likely failure modes
- [ ] Automation boundaries are explicitly stated for every step
- [ ] All domain entities referenced exist in the domain model
- [ ] Traceability links are populated

## Orchestration Patterns

This skill is invoked by:
- `zdp-orchestrator` at the Define stage

This skill provides input to:
- `acceptance-testing` -- scenarios become the source for UAT test cases
- `disciplined-design` -- scenarios inform the design brief scope
- `architecture` -- scenarios reveal integration points and data flows

## Guidelines

- Use domain model terminology consistently -- do not invent new terms
- Each scenario must trace to at least one persona and at least one JTBD
- Keep scenarios focused: one primary value flow per scenario
- Exception paths are not edge cases -- they are expected alternative flows that the system must handle
- Automation boundaries are design decisions, not implementation details -- justify them
