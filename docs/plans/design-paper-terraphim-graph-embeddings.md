# Implementation Plan: Academic Article — Terraphim Graph Embeddings

**Status**: Draft
**Canonical Path**: `docs/plans/design-paper-terraphim-graph-embeddings.md`
**Change Slug**: `paper-terraphim-graph-embeddings`
**Research**: `docs/plans/research-paper-terraphim-graph-embeddings.md`
**Author**: Kokoro (for Alex Mikhalev)
**Date**: 2026-09-26
**Estimated Effort**: 3–4 weeks part-time (writing) + 3–5 days (experiments)

## Overview

### Summary
A disciplined plan to write a peer-reviewable systems/experience paper on
Terraphim's deterministic semantic search stack: Aho-Corasick concept
extraction, integer-rank co-occurrence graph, and hybrid rank+TF-IDF scoring,
evaluated against the costs/benefits of learned graph embeddings (Cleora).

### Approach
Venue-anchored (pending Alex's answer on Q1), experiments-first writing
order, every empirical claim bound to a committed reproduction artefact.

### Scope

**In Scope:**
- One system description (automata → RoleGraph → hybrid scorer).
- One evaluation corpus with 50–100 judged queries.
- Two scoring arms measured (pure graph-rank; hybrid +TF-IDF), plus optional
  Cleora arm if the reference run is cheap.
- Latency, ranking-quality, memory, and update-throughput measurements.
- Determinism demonstration (identical-rank replay across runs).
- Related work: graph embeddings, entity-linking/KB-grounded IR, lexical
  baselines, interpretable IR.

**Out of Scope:**
- Any new algorithm or training method.
- The build-system DSL application (separate artefact).
- SNOMED/UMLS/medical extractors, WASM portability, HNSW retrieval.

**Avoid At All Cost** (5/25):
- Claiming algorithmic novelty (invites wrong reviewers, unwinnable).
- "Embeddings are bad" polemics — the paper measures trade-offs, not sides.
- Benchmarking on hardware/specs not reported.
- Citing numbers from internal docs without fresh reproduction.
- Scope creep into implementing the Cleora roadmap (§4 of the old doc).

## Architecture (of the article + evidence pipeline)

```
Corpus (fixed, hashed)
   │
   ├─► Indexing runs ──► RoleGraph ranks ──► Arm 1: pure rank scoring
   │                                              │
   └─► Query set (50–100) ──► Judgements          ├─► Arm 2: rank+TF-IDF hybrid
                                  │               │    (existing TFIDFScorer)
                                  ▼               ▼
                            MAP@10 / nDCG@10 ◄── per-arm scores
                            Latency p50/p95/p99
                            Memory footprint
                            Update throughput (O(1) claim check)
                            Determinism replay (byte-identical ranks)
   │
   └─► (optional) Cleora on same graph ──► Arm 3: cosine top-k
```

### Production Toolchain (RESOLVED by Alex, 2026-09-26)

**Decisions locked**: archive-first (arXiv), solo author, terraphim-ai docs
corpus, LaTeX→Typst toolchain.

**Pipeline**: Quarto single-source (`paper.qmd` + `references.bib`) rendered
with the **Typst PDF engine** (installed: `typst` + quarto 1.10.18),
**xelatex as fallback** if a rendering path misbehaves. Official base format:
`quarto-journals/article-format-template` (arXiv has no dedicated official
format). Deposit strategy: render PDF locally and deposit **PDF-first** to
arXiv (cs.DL or cs.IR) — avoids depending on arXiv's evolving Typst-source
support; if a later venue requires source, re-render the same Quarto source
with the xelatex `pdf` engine.

If the paper later moves to ACM/Elsevier: same source, swap format extension
(`quarto-journals/acm`, `/elsevier`); content untouched.

- **Local house style to reuse**: `terraphim-skills/evals/jev-eval/report/*.qmd` —
  one-minute-summary callout, exec-summary-with-table, figure-first results sections;
  proven on a real evaluation report. Terraphim book conventions exist too
  (`terraphim-internal/_quarto.yml`, `charm-impact/docs/_quarto.yml`) if the paper
  later grows a companion site.
- **Bonus**: the Terraphim Gitea fork renders Quarto natively
  (`gitea/modules/markup/quarto/`) — the draft can render in-repo on
  git.terraphim.cloud without extra CI.

Writing workflow: `paper/paper.qmd` + `references.bib` (Zotero/Better BibTeX
export) + `quarto render --to typst` (fallback `--to pdf`) → figures generated
by experiment scripts into `paper/figures/`.

**Authorship & AI disclosure (locked)**: Alex Mikhalev, sole author.
AI-assistance (research planning, experiment scaffolding, drafting support by
Kokoro/OpenClaw) acknowledged per arXiv policy — human-directed work; arXiv
now bans *fully* AI-generated submissions, so the acknowledgement line goes
in Acknowledgements and Alex reviews/approves all prose.

### Key Design Decisions
| Decision | Rationale | Alternatives Rejected |
|----------|-----------|----------------------|
| Systems/experience framing | Matches what exists in repo; achievable with 3–5 days of experiments | Algorithms framing (no novel method); survey (stale risk) |
| Own corpus + declared LLM-assisted judgements with human spot-check | Terraphim-specific claims need Terraphim-style corpus; public IR collections don't exercise thesaurus linking | INCOSE-only (thin); public collection only (costly setup, weak fit) |
| Draft experiments before intro | Prevents narrative Drift ahead of evidence | Intro-first (classic failure mode) |
| α-blend reported as *existing implementation*, not tuned by us | Honest scoping: we evaluate shipped code | Full α-sweep (only if time allows) |

## Expected Lifecycle Artefacts

| Artefact | Path | Required? |
|----------|------|-----------|
| Research (done) | `docs/plans/research-paper-terraphim-graph-embeddings.md` | Yes ✅ |
| Design (this file) | `docs/plans/design-paper-terraphim-graph-embeddings.md` | Yes |
| Experiment spec | `docs/specs/paper-terraphim-graph-embeddings.experiments.md` | Yes |
| Claims-evidence table | `docs/verification/claims-evidence-paper.md` | Yes |
| Results bundle | `docs/research/paper-results-<date>/` (raw JSON, logs, plots) | Yes |
| Reproduction script | `scripts/paper/reproduce-all.sh` + pinned env | Yes |
| Decision: venue+authorship | `docs/decisions/D-2026-NNN-paper-venue.md` | Yes (small) |
| Validation | `docs/validation/validation-report-paper.md` | Yes (claim check) |

## Article Structure (target 8–12 pp., adjust to venue)

1. **Introduction** (1–1.5 pp) — RQ: *What do deterministic graph-based
   rankings cost and buy compared with learned graph embeddings, in a
   production semantic-search system?* Contributions list (3 bullets: system,
   measured trade-offs, released artefacts).
2. **Background & Related Work** (1.5–2 pp) — AC automata; TF-IDF/BM25;
   node2vec/DeepWalk/Cleora; entity linking & KG-grounded IR; interpretable
   IR. Positioning table: our system vs each family.
3. **System Description** (2–2.5 pp) — pipeline diagram; thesaurus format;
   AC matching incl. word-boundary guards (`MIN_FIND_PATTERN_LENGTH`,
   boundary predicate — genuinely publishable engineering detail);
   RoleGraph integer ranks; hybrid scorer (30/70 blend, existing code);
   complexity table per stage.
4. **Evaluation** (2–3 pp) — setup (hw, versions, corpus hash); RQ1 ranking
   quality (MAP@10, nDCG@10, arms 1 vs 2 vs optional 3); RQ2 latency
   (p50/p95/p99, matching vs full query); RQ3 update economics (O(1) rank
   update vs re-embedding time); RQ4 determinism (replay, byte-identical);
   RQ5 memory (ints vs d-dim floats, measured).
5. **Discussion** (1 pp) — when counts suffice; the long-tail limitation
   (from the internal doc's §1.5, now evidenced); threat panel: LLM-assisted
   judgements, single corpus, untuned baselines.
6. **Conclusion & Future Work** (0.5 p) — the α-blend + ANN enrichment as
   future; "counts get you so far; embeddings get you the rest" as closing
   (with the source doc cited in acknowledgements/materials).
7. **References + Artefact appendix** — reproduction script, corpus hash,
   judgement files.

## Experiment Plan

| # | Experiment | Measures | Output | Effort |
|---|------------|----------|--------|--------|
| E1 | Verify hybrid path: run existing rolegraph/KG-ranking tests + hybrid tests | Live-code assumption | test log | 0.5 d |
| E2 | Build corpus: pick + freeze + hash (candidates: terraphim-ai docs build, INCOSE handbook) | Reproducibility anchor | `corpus/SHA256SUMS` | 0.5 d |
| E3 | Query set + judgements (50–100 queries; LLM-assisted pooling, Alex spot-checks 20 %) | Ground truth | `judgements.tsv` | 1–1.5 d |
| E4 | Arm 1 vs Arm 2 ranking quality | MAP@10, nDCG@10 | results JSON + plots | 0.5 d |
| E5 | Latency microbench (AC match alone; full query path) | p50/p95/p99 | results JSON | 0.5 d |
| E6 | Update throughput: incremental doc add vs Cleora full re-embed timing | ops/s, wall-clock | results JSON | 0.5 d |
| E7 | Memory: peak RSS indexing + query; ints vs floats accounting | MB | results JSON | 0.5 d |
| E8 | Determinism: 3× full pipeline replay | diff = ∅ | hashes | 0.25 d |
| E9 | (Optional) Cleora run, default params, same graph export | MAP@10 etc. | results JSON | 1 d |

E1 gates everything. E9 only if E1–E8 land inside budget.

## Writing Sequence (experiments-first)

| Step | Section | Depends on | Est. |
|------|---------|------------|------|
| W0 | Claims-evidence table skeleton (every bullet → E#) | design approval | 0.5 d |
| W1 | System Description | E1 | 2 d |
| W2 | Evaluation | E4–E8 (+E9?) | 2 d |
| W3 | Background & Related Work | venue scan | 1.5 d |
| W4 | Discussion + Threats | W2 | 1 d |
| W5 | Intro + Contributions + Abstract | W1–W4 | 1 d |
| W6 | Artefact appendix + repro script polish | all | 0.5 d |
| W7 | Full self-review vs claims table; Alex's pass | W6 | 1 d |

Total ≈ 15–17 focused days.

## Test Strategy (for the evidence, not the prose)

| Check | Method | Pass criterion |
|-------|--------|----------------|
| Reproducibility | `scripts/paper/reproduce-all.sh` on clean checkout | Same numbers ± floating-point noise, byte-identical determinism hashes |
| Claim coverage | Claims-evidence table audit | 100 % of empirical claims map to E# results |
| Baseline fairness | Report defaults + any tuned runs separately | No silent tuning |
| Statistics | Bootstrap CIs (1,000 resamples) over queries | CIs reported for MAP/nDCG deltas |

## API (of the reproduction tooling)

Minimal new surface; prefer existing bins/tests:

```bash
scripts/paper/reproduce-all.sh --corpus <dir> --queries <tsv> --arms 1,2[,3]
# outputs: docs/research/paper-results-<date>/{results.json, latency.json,
#          memory.json, determinism.hashes, plots/*.svg, RUN_MANIFEST.json}
```

`RUN_MANIFEST.json`: git SHAs (terraphim-core, service, monorepo), rustc
version, CPU/RAM, corpus hash, timestamps. No plot without its manifest.

## Implementation Steps

### Step 1: Verify & freeze (E1, E2) — 1 day
Run hybrid tests green on this machine; pin commits; freeze corpus + hashes.
**Gate**: tests green, corpus hash committed.

### Step 2: Judgements (E3) — 1–1.5 days
Query set from realistic retrieval intents; LLM-assisted candidate pooling;
Alex spot-checks a 20 % sample.
**Gate**: ≥50 judged queries committed.

### Step 3: Measurements (E4–E8) — 2.5 days
All arms, all metrics, manifests per run.
**Gate**: results bundle complete; claims table filled for RQ1–RQ5.

### Step 4: Optional Cleora arm (E9) — 1 day, skippable
**Gate**: only proceed if ≤1 day total.

### Step 5: Drafting W1–W7 — ~9 days
**Gate**: full draft exists; every table/figure traceable.

### Step 6: Review cycle — 2–3 days
Self-review vs claims table → Alex's edits → (venue-dependent) internal
reader. British English throughout; AI-assistance disclosed per venue policy.

### Rollback / Fallback Plan
If E3 (judgements) stalls → Plan B position paper: determinism/explainability
argument + latency/memory + determinism results only (no MAP/nDCG), targeted
at a practitioner venue. No work is discarded — experiments done under Plan A
fold into Plan B's evidence.

## Open Items

| Item | Status | Owner |
|------|--------|-------|
| Venue + deadline | **RESOLVED**: arXiv-first (cs.IR/cs.DL), PDF-first deposit; no external deadline | — |
| Authorship + AI-disclosure | **RESOLVED**: solo Alex; AI-assistance acknowledged per arXiv policy | — |
| Corpus choice | **RESOLVED**: terraphim-ai docs build (freeze + SHA256) | — |
| Writing toolchain | **RESOLVED**: Quarto + Typst engine, xelatex fallback, article-format-template | — |
| Cleora arm in/out | Citation-only default; E9 only on explicit request | Alex |
| Discourse original material | **RESOLVED 2026-09-26**: `terraphim.discourse.group` is public; 10 topics archived to `docs/research/paper-sources-discourse/`. T19 = genesis artefact. | — |

## Gate Status

**E1 EXECUTED 2026-09-26 — PASSED with one adaptation.** Evidence:
`docs/research/paper-bench-evidence-20260926/` (criterion logs + machine.txt,
Apple Silicon Mac, rustc per machine.txt; short criterion settings: 2–3 s
measurement, reduced samples — indicative numbers, not final paper numbers).

| Check | Result |
|-------|--------|
| terraphim_automata + terraphim_rolegraph tests | ✅ all green, exit 0 (incl. 5 TF-IDF unit tests in rolegraph lib.rs) |
| Bench targets compile | ✅ 4 binaries (autocomplete, throughput, symbolic-embedding, automata lib) |
| automata bench (69 timings) | ✅ run complete; **autocomplete search 1.6–2.8 µs**; exact-match FST 631 ns; index build ~linear 0.45 ms/100 → 60 ms/10k terms; serialise ~139 µs/100 → 7.7 ms/5k; fuzzy search 2.5 µs (prefix) to 8.6 ms (extra-char class) |
| rolegraph throughput bench | ✅ **query path 0.8–2.7 ms** by corpus size; AC extraction 1.2 µs/1 term → 1.2 ms/1k terms; pair-parsing flat ~3.3 µs; connectivity check 253 ns |
| symbolic_embedding_bench | ⚠️ requires `--features medical` (easy to miss — **paper repro script must pin it**); build is O(n²)-ish: 1k nodes 38 ms, 10k nodes **5.4 s**; similarity cold 38.5 ms / **warm 8.2 ns** |

**Findings for the paper:**
1. Hybrid path is live: `TFIDFScorer` exported from `terraphim_types::score`, wired via `QueryScorer::Tfidf` in `sort_documents`; rolegraph carries its own TF-IDF unit tests. Claim survives.
2. **New section needed**: `symbolic_embeddings` (749-LOC module) — deterministic set-based embeddings (ancestor/descendant transitive closures, Jaccard similarity) already benchmarked. This is Terraphim's in-house answer to learned embeddings and strengthens the thesis; warm-cache similarity at single-digit ns is a striking datapoint. Build-cost scaling (38 ms → 5.4 s for 10×) is the honest trade-off to report.
3. Bench architecture is ready for E4–E8: criterion suites exist per stage (build / search / serialise / memory / concurrency / end-to-end query); monorepo adds service-level release-gate benchmarks (`benchmark-config.json`: API < 1 s, success ≥ 99 %, mem < 1 GiB).
4. The `--features medical` gate is exactly the kind of reproduction trap the artefact appendix must document.

**E1 gate: PASSED** → E2 (corpus freeze) and E3 (judgements) are unblocked.

**E2 EXECUTED 2026-09-26 — DONE.** Corpus `terraphim-docs` frozen:
`paper/corpus/` — 130 md files copied from `docs/src/` @ `2d363b8a…`,
`SHA256SUMS` (null-delimited handling for filenames with spaces),
`CORPUS_HASH.txt = ec54d56b241ad7ed7d6be826e0c299455ea4a29db91fd66203cd6c18cf77ff66`,
`PROVENANCE.md` with regeneration command. Manifest verifies clean.
Commit decision at review time: manifest+provenance belong in git; the copied
files themselves are regenerable from the pinned commit and may be gitignored.

**Article scaffold CREATED 2026-09-26 — renders clean in 3 formats.**
`paper/paper.qmd` (full section skeleton, RQ1–RQ5, evidence-discipline
directives, every TODO tied to plan steps W1–W6), `paper/references.bib`
(12 entries incl. 4 Discourse/Vimeo primary sources; Cleora entry flagged
verify-before-submission), `_quarto.yml`. Renders: typst → `paper.pdf`
(fixed YAML gotchas: margin needs quoted dims; CSL file must exist), docx,
html. Next writing step: W1 (System Description) + E3 (judgements) in
parallel.

## Approval

- [ ] Research document reviewed by Alex
- [x] Design (this plan) — decisions locked 2026-09-26: arXiv-first, solo author, terraphim-ai docs corpus, Quarto+Typst
- [x] Venue + corpus + toolchain decisions made (Open Items)
- [x] E1 executed (hybrid path verified live) — **PASSED 2026-09-26**, evidence in `docs/research/paper-bench-evidence-20260926/`; full criterion sweep done; symbolic-embeddings module discovered → new paper section

No writing begins before E1 is green and Alex has reviewed the research doc.
