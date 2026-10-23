# Example Knowledge Graph

This directory is the working example knowledge graph published with the documentation. Every file is a concept: the filename is the normalised term, the heading names the concept, and the `synonyms::` line enumerates the surface forms that must resolve to it.

```markdown
# Search

Search functionality and document retrieval.

synonyms:: document retrieval, keyword lookup
```

Concept files with a `checklist::` line additionally declare a named checklist that `terraphim-agent validate --checklist <name>` enforces: see [checklists/code_review.md](./checklists/code_review.md) and [checklists/security.md](./checklists/security.md).

## The compiled artefact

`../thesaurus.json` is the compiled graph for the **Terraphim Engineer** role. It is generated from every `synonyms::` line under `docs/src/` -- not only this directory -- which means every article in the book is itself a concept node with its own `checklist::`. Regenerate after editing concepts:

```bash
python3 - <<'PY'
import os, json, glob
data, cid = {}, 100
for f in sorted(glob.glob("src/**/*.md", recursive=True)):
    nterm = os.path.splitext(os.path.basename(f))[0]
    txt = open(f, encoding="utf-8").read()
    syn = next((l.split("::",1)[1] for l in txt.splitlines()
                if l.strip().lower().startswith("synonyms::")), "")
    if not syn:
        continue
    for t in dict.fromkeys([nterm.lower()] + [s.strip().lower() for s in syn.split(",") if s.strip()]):
        data.setdefault(t, {"id": cid, "nterm": nterm})
    cid += 1
json.dump({"name": "Terraphim Engineer", "data": data}, open("src/thesaurus.json","w"), indent=2)
PY
```

## Using it

```bash
# Ranked search with concept expansion
terraphim-grep "quality gate" --paths docs/src \
  --thesaurus docs/src/thesaurus.json -n 8 -C 1 --json

# Validate a text against a named checklist
terraphim-agent validate --role "Terraphim Engineer" \
  --checklist code_review < draft.md
```

Every docs article carries the same two directive lines near its heading, so the graph, the book, and the agent checklists stay one edit away from each other.

synonyms:: example knowledge graph, terraphim engineer kg, kg example, concept graph
checklist:: the compiled artefact, using it
