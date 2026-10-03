# GEPA Research

**Category:** AI Tooling  
**Domain:** Autonomous Optimisation, Evolutionary Search  
**Source:** https://github.com/CyrusNuevoDia/gepa-research  

A plugin for agentic coding frameworks that optimises code using the GEPA algorithm (Genetic-Pareto LLM-driven search). Given a codebase, it discovers metrics to optimise, instruments evaluation, and runs reflection-driven evolutionary optimisation with Pareto-efficient candidate selection.

## Core Algorithm

GEPA (Genetic-Pareto LLM-driven search) maintains a Pareto frontier of candidate solutions. For each iteration:
1. Select a parent candidate from the frontier
2. An LLM reflects on diagnostic side-info (stdout, stderr, task traces, gate failures)
3. Propose targeted edits to the candidate
4. Evaluate in an isolated git worktree
5. If score improves and gates pass, commit to the frontier
6. Stop on budget exhaustion or stall (no improvement for N iterations)

## Key Components

- **GepaResearchAdapter**: Bridge between GEPA evaluator protocol and git worktree/benchmark machinery
- **Per-candidate git worktrees**: Full audit trail, safe rollback, isolated evaluation
- **Inherited gate system**: Safety checks propagate down the experiment tree
- **SDK** (Python + Node): `Run` class with `log()`, `report()`, `finish()` for benchmark instrumentation
- **Flask dashboard**: Real-time visualisation of candidate lineage DAG

## Applicability

- Agent skill and prompt optimisation
- Rust performance optimisation (pi_agent_rust, terraphim-ai)
- Meta-learning loop engine (P3 autonomous research in self-improving framework)
- Content quality optimisation with LLM-judged scoring

## Dependencies

gepa>=0.1.0, flask>=3.0.0, litellm>=1.83.10, typer>=0.12, portalocker>=2.8.0

## Related Concepts

- evolutionary optimisation
- Pareto frontier
- LLM reflection
- autonomous research
- git worktree isolation
