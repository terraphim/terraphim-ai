# Archive: Specs Extracted With Polyrepo Split (#1910)

These specs were written when the code they describe resided in this monorepo. After
the polyrepo extraction (Gitea #1910), the referenced crates moved to standalone
repositories and are consumed from the `terraphim` private registry.

All acceptance criteria in these specs have been verified as **PASS** against the
vendored registry crates (v1.20.4, validated 2026-06-25).

The canonical home for ongoing spec work is the downstream repo:
- `terraphim-core` — for `terraphim_automata` / `terraphim_rolegraph` specs
- `terraphim-agents` — for `terraphim_agent::learnings` specs

**Refs: Gitea #2972**
