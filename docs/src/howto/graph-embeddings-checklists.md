# Graph Embeddings for Checklists

> Originally published on the Terraphim blog: [Graph Embeddings for Checklists](https://terraphim.ai/posts/graph-embeddings-checklists/) (20 October 2026).

A checklist is an enumerated set of terms that must be checked. That is the whole problem, and it is the reason vector embeddings are the wrong tool for it: a nearest-neighbour index returns fuzzy neighbours, it cannot enumerate a fixed set and report which members fired. A knowledge graph can. Write the checklist once as synonyms under a normalised term, compile it into an Aho-Corasick automaton, and every check becomes a deterministic match with a named edge you can audit.

This guide shows that machinery applied to the two places a checklist earns its keep around an LLM call: `terraphim-grep` with a role thesaurus before the call, and `terraphim-agent` hooks plus learning after it. Every command below was run while writing this article; the outputs are real.

## The Checklist Is an Automaton, Not a Promise

The usual pattern for "checklists" in agent workflows is a paragraph in a system prompt that the model promises to honour. That is a probabilistic check enforced by the thing being checked. Terraphim inverts it: the checklist lives outside the model, in the same structure that powers [graph embeddings](https://terraphim.ai/docs/graph-embeddings/).

A checklist item set looks like this in the knowledge graph. This is `bun install.md` from the rewrite graph on this machine:

```markdown
# bun add

Install dependencies using Bun package manager.

synonyms:: npm install, yarn install, pnpm install, npm i, yarn add, pnpm add
```

The filename gives the normalised term the hook reports back (the suggestion in the findings output below is `bun install`, matching `bun install.md`), while the heading names the approved form inside the file. The `synonyms::` line is the checklist itself: every surface form that must be recognised and dealt with. Compile the directory of such files into `thesaurus.json` and you get the "Terraphim Engineer" role used below, currently 219 entries. Rebuilding the automaton after an edit takes about 20 ms, a figure verified in [Why Graph Embeddings Matter](https://terraphim.ai/posts/why-graph-embeddings-matter/), so the checklist is cheap to change and free to consult.

## Before the LLM Call: Retrieval as Checklist Execution

The first job of a checklist is deciding what goes into the prompt. "Gather relevant context" is vague; "which of the enumerated terms in this role's graph appear in the repository, and what do they pull in" is a check.

Here is the query run over the site's own source while preparing this article, using the Terraphim Engineer thesaurus:

```bash
terraphim-grep "quality gate" --haystack all --paths . \
  --thesaurus ~/.config/terraphim/thesaurus.json -n 8 -C 1 --json
```

The result, abridged:

```json
"stats": {
  "search_latency_ms": 53,
  "chunks_returned": 8,
  "kg_hits": 8
},
"concepts": [
  {"name": "accept"}, {"name": "accepted"}, {"name": "escalate"},
  {"name": "fail"}, {"name": "improve"}, {"name": "judge-verdicts"},
  {"name": "needs improvement"}, {"name": "pass"}
]
```

Two things worth pausing on. First, the concepts list is the checklist executing: the graph expanded "quality gate" into its item set (accept, fail, escalate, pass, judge-verdicts and friends) and matched those terms across the tree. Second, the chunks it returned came from `.github/workflows/*.yml` and `campaigns/skills-2026/*`, files that contain none of the words "quality" or "gate". They matched on `fail`, `curl --fail`, `accepted`: items on the checklist, not words in the query. Fifty-three milliseconds, no embedding model, and every hit can name the edge that produced it.

### The honest empty case

A checklist that cannot say "not covered" is useless, so here is the control run against the same thesaurus:

```bash
terraphim-grep "checklist" --haystack all --paths . \
  --thesaurus ~/.config/terraphim/thesaurus.json -n 8 -C 2 --json
```

```json
"kg_hits": 0,
"sufficiency": "SearchOnly",
"sufficiency_explanation": "Found 4 chunks but KG confidence 0.00 < 0.50,
  diversity 1 < 2; returning search results only (pass --answer or
  --force-rlm to synthesise)."
```

Zero KG hits, four plain lexical chunks, and an explicit admission that graph confidence was too low to claim concept coverage. The tool does not pretend the checklist applied. If you want "checklist" to be a graph concept, you write it into a `kg/*.md` file with a `synonyms::` line and rebuild; the gap is a text edit, not a retraining run. The project-local variant of this workflow (a `.terraphim/` directory in your repo with its own role thesaurus) is documented in the [terraphim-grep offline setup](https://terraphim.ai/posts/skill-spotlight-terraphim-grep/) and the concepts can be enriched with code anchors mined by ast-grep.

### Reproduce it

The artefacts behind these outputs are committed in the website repository: `.terraphim/` holds `kg/` concept files (`validation gate`, `judge-verdicts`, `hook`, `grounding gate` and friends), `rewrite/` holds the hook checklist, and `thesaurus.json` is the compiled 39-entry public subset of the working role. Run the same command against it:

```bash
terraphim-grep "quality gate" --haystack all --paths . \
  --thesaurus .terraphim/thesaurus.json -n 8 -C 1 --json
```

Same eight concepts, same honest `kg_hits: 0` on the "checklist" control. For larger published checklist graphs, `docs/src/kg/checklists/` in the terraphim-ai repository carries the code review and security concepts.

## After the LLM Call: The Hook Is a Findings Checklist

The second job of a checklist is gating what comes back. In current releases, `terraphim-agent hook` does not silently rewrite the agent's proposed action; it returns a machine-readable findings list. That distinction matters: the hook's job is to *check*, and a check that mutates silently is a check you cannot audit.

Run from the directory holding the rewrite config, against a tool call an agent might propose:

```bash
echo '{"tool_name":"Bash","tool_input":{"command":"npm install react"}}' \
  | terraphim-agent hook --hook-type pre-tool-use --json
```

```json
{"tool_input":{"command":"npm install react"},"tool_name":"Bash",
 "validations":{"findings":[{"matched_term":"npm install",
 "suggested_replacement":"bun install"}],"has_findings":true}}
```

Read that output as a completed checklist line: item "npm install" found, prescribed action "bun install", flag raised. The hook types cover the lifecycle (`pre-tool-use`, `post-tool-use`, `pre-commit`, `prepare-commit-msg`), and `--with-guard` is on by default for pre-tool-use so destructive commands get intercepted in the same pass. The git-hook end of this is covered in [Teaching AI Coding Agents with Knowledge Graph Hooks](https://terraphim.ai/posts/teaching-ai-agents-with-knowledge-graphs/) and the end-to-end verified flow in [Native Hook Support](https://terraphim.ai/posts/native-hook-support/).

### When the checklist itself fails

Checklists also cover the checking tool. Two honest notes from this session:

- `terraphim-agent learn query "checklist"` returned a list of past failed commands (exit-1 runs captured by the PostToolUse hook); the learning store is doing its job remembering what went wrong before.
- `terraphim-agent replace` in this build reports `replacements: 1, "changed": true` while returning the input unchanged. The findings-based `hook` path above is the contract to rely on; if `replace` is your integration point, verify the output rather than the flag. This is itself a captured learning; see [Teaching AI Agents to Learn from Their Mistakes](https://terraphim.ai/posts/teaching-ai-agents-to-learn-from-mistakes/) for how such a failure becomes a stored correction and eventually a new synonym.

## The Loop, Closed

Put the phases together and the checklist lifecycle is:

1. **Declare**: write `kg/*.md` files; heading = normalised item, `synonyms::` = surface forms to catch.
2. **Compile**: generate `thesaurus.json`; ~20 ms per role.
3. **Retrieve (pre-LLM)**: `terraphim-grep` with the role thesaurus expands queries into concept terms and pulls grounded chunks, or honestly reports `kg_hits: 0`.
4. **Draft**: the LLM call runs with checklist-shaped context instead of a vibes-based prompt.
5. **Gate (post-LLM)**: `terraphim-agent hook` emits findings; guards intercept destructive commands; failed commands are captured by the learning hook.
6. **Correct**: `learn correct` / `learn correction add` turn observed failures into reviewed rules; `memory capture` stores authored lessons; the synonyms grow and the automaton recompiles.

Step 6 is what makes this different from a static lint list. Corrections feed back into the graph, so the checklist the next session runs against is the checklist this session finished, a property the `learn`/`memory` subcommands exist to provide, described in the [learning capture spotlight](https://terraphim.ai/posts/skill-spotlight-learning-capture/).

## Why This Is Not a Vector Problem

A vector index answers "what is near this?" A checklist asks "which of these exact items occurred, and what was the prescribed response?" Those are different questions with different audit requirements. Graph embeddings give you the second in tens of milliseconds on a laptop, with an explainable edge for every match and no network call. The full performance case, including the 1.4-million-pattern sub-millisecond figure, is in [Why Graph Embeddings Matter](https://terraphim.ai/posts/why-graph-embeddings-matter/) and [Sub-Millisecond Context with Knowledge Graphs](https://terraphim.ai/posts/sub-millisecond-context-knowledge-graphs/); the mechanism is on the [Graph Embeddings reference](https://terraphim.ai/docs/graph-embeddings/) page.

For the wiring in your own agent setup, the [command rewriting how-to](../command-rewriting-howto.md) walks through where the synonym files live, how the role graph is built, and how host hooks call the matcher. Write the checklist once; let the automaton enforce it on both sides of the call.
