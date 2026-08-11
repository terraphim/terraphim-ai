# Repo-local agent memory

Durable, repo-scoped memory for agents working in this repository (fleet
standard §8). Root `memory/` is a read-only haystack in `.terraphim/config.json`,
so everything here is searchable via terraphim-agent / terraphim-grep.

- `YYYY-MM-DD.md` — daily working notes (decisions, evidence, pointers)
- `regressions.md` — failures turned into guardrails: `| date | what broke | rule | status |`
- Long-term curated notes live in the global Terraphim KG, not here — promote
  distilled learnings, keep raw logs local.
