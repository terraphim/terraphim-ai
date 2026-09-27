# Research: Academic Article on Terraphim Graph Embeddings

**Status**: Draft
**Canonical Path**: `docs/plans/research-paper-terraphim-graph-embeddings.md`
**Change Slug**: `paper-terraphim-graph-embeddings`
**Author**: Kokoro (for Alex Mikhalev)
**Date**: 2026-09-26

## Executive Summary

Terraphim's graph-embedding stack — deterministic Aho-Corasick concept
extraction (`terraphim_automata`), an integer-rank co-occurrence RoleGraph
(`terraphim_rolegraph`), and an implemented TF-IDF hybrid scoring step — is a
rare production instance of *deterministic, explainable semantic ranking*.
That contrast with probabilistic neural embeddings (node2vec/DeepWalk/Cleora)
is the article's core contribution. This research maps what exists, what the
candidate claims are, where the evidence gaps are, and what the article must
contain to be publishable at a workshop/venue-appropriate level.

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Energizing? | Yes | Alex has iterated on this theme repeatedly (build-system DSL riff, zvec comparison blog, Cleora analysis doc all exist in-repo) |
| Leverages strengths? | Yes | Working Rust codebase with benchmarks, blog pipeline, and prior analysis docs — Alex can write from code, not vapourware |
| Meets real need? | Yes | Explainability/determinism of embeddings is an active concern (EU AI Act Art. 86 transparency, RAG evaluation literature); no published account of the AC+rank-graph+hybrid approach exists |

**Proceed**: Yes — 3/3 YES.

## Problem Statement

### Description
There is no academic artefact describing Terraphim's approach to semantic
search: concept extraction via Aho-Corasick automata over a curated thesaurus,
a frequency-ranked co-occurrence graph, and hybrid TF-IDF/graph scoring. The
approach inverts the current trend (neural embeddings first) by making
*determinism, sub-millisecond matching, and auditability* the primary
requirements, and treating learned embeddings as an optional enrichment.

### Impact
- Alex / Terraphim: scholarly visibility, citable artefact for the project.
- The field: a documented production counterpoint to "embed everything"
  orthodoxy, with measurable trade-offs.

### Success Criteria
1. A complete draft (8–12 pp.) suitable for a named target venue.
2. Every empirical claim traceable to a benchmark run in the repo.
3. Related-work coverage of graph embeddings, lexical/semantic hybrids, and
   interpretable IR is current (2023–2026).

## Current State Analysis

### Existing Implementation (verified in-repo, 2026-09-26)

| Component | Location | Role in article |
|-----------|----------|-----------------|
| `terraphim_automata` | `terraphim-core` polyrepo, `crates/terraphim_automata/` | Aho-Corasick matcher, word-boundary logic (`matcher.rs`, `MIN_FIND_PATTERN_LENGTH`, `is_word_boundary_match`), FST autocomplete, replace/link generation (`replace_matches`, `LinkType`) |
| `terraphim_rolegraph` | `terraphim-core` polyrepo, `crates/terraphim_rolegraph/` | `RoleGraph`: `HashMap<u64, Node>` concepts, `HashMap<u64, Edge>` co-occurrences, integer ranks incremented during indexing; lock-free `ahash` |
| Hybrid scorer | `terraphim-service` (service polyrepo), `score/bm25_additional.rs` (`TFIDFScorer`) | 30 % TF-IDF + 70 % graph-rank blend in `TerraphimGraph` relevance function — already implemented and tested |
| Cleora gap analysis | `terraphim-ai` monorepo, `docs/src/scorers/graph-embedding-analysis.md` | Existing internal comparison: Cleora (Chebyshev-polynomial "widening" embeddings) vs integer ranks; proposes α-blend hybrid + HNSW retrieval; §5 conclusion quotes "Counts get you so far; embeddings get you the rest." |
| Benchmarks | `terraphim-ai` monorepo, `PERFORMANCE_BENCHMARKING_README.md`, `benchmark-config.json` | Infrastructure for reproducible runs |
| Prior prose | `blog/2026-02-16-zvec-vs-terraphim-comparison.md`, Twitter thread v1.8.1 | Reusable framing; not academic register |

### Architecture (as it would be presented in the paper)

1. **Offline**: curated thesaurus (JSON/markdown KG) → Aho-Corasick automaton
   build. O(Σ patterns) construction, O(text length) streaming match.
2. **Indexing**: documents → sentence split → AC match → consecutive-pair
   co-occurrence edges; `edge.rank += 1`, `node.rank += 1`, `doc.rank += 1`.
3. **Query**: AC match on query → matched node IDs → score =
   Σ(node.rank + edge.rank + doc.rank), re-scored with TF-IDF (30 %) in the
   hybrid path.
4. **Optional enrichment (roadmap, not fully implemented)**: Cleora-style
   embeddings over the co-occurrence graph → cosine similarity → α-blend.

### Key distinctions available for the paper
- **Determinism**: same input → same ranks/links, no RNG, no training run.
- **Latency**: O(text) matching vs ANN traversal; sub-ms is plausible and
  benchmarkable.
- **Auditability**: ranks are human-readable counters; embeddings are not.
- **Freshness**: O(1) online updates vs re-embedding.
- **The "embedding" claim**: the graph itself *is* a discrete embedding of the
  corpus into concept-co-occurrence space; the question "are learned dense
  vectors worth their costs?" is the research question.

## Constraints

### Technical
- Benchmarks must be re-run, not quoted from old docs (machine/dates must be
  reported).
- WASM/WASM32 constraint noted in prior analysis (`std::sync::atomic`
  caveats) — relevant only if the paper claims browser portability.

### Business / Practical
- Writing time is the scarce resource; the plan must sequence writing so that
  experiments land before their sections are drafted.
- Authorship/affiliation and licensing (Apache-2.0/MIT dual) to be confirmed
  by Alex — open question.

### Non-Functional (article-level)
| Requirement | Target |
|-------------|--------|
| Length | 8–12 pp. (workshop) or 4–6 pp. (short/DBIR-style) |
| Reproducibility | All numbers from committed scripts + seeds |
| Baselines reproducible | Cleora reference run on identical corpus |

## Vital Few (Essentialism)

### Essential Constraints (Max 3)

| Constraint | Why It's Vital | Evidence |
|------------|----------------|----------|
| Every empirical claim backed by a fresh benchmark run | Academic credibility dies on un-reproducible numbers | Prior docs quote unspecified machines/dates |
| One clear research question, one hybrid system, one corpus | A single well-executed claim beats a survey | Cleora doc already sprawls across 5 roadmap items |
| Venue fit before drafting | Structure, length, and review rubric follow the venue | 8–12 pp. vs 4–6 pp. changes experiment count |

### Eliminated from Scope (5/25 rule)

| Eliminated Item | Why Eliminated |
|-----------------|----------------|
| Full HNSW/ANN retrieval implementation | Roadmap item; only needed if pure-embedding arm is in scope |
| Cleora-streaming / warm-start investigation | Tangential to the core claim |
| Medical/terminology extractors (SNOMED/UMLS/med artifacts) | Separate subsystem; one sentence at most |
| Browser/WASM portability claims | Nice aside, weakens focus |
| Novel algorithm contribution claim | This is a *systems/experience* paper, not an algorithms paper — attempting novelty framing invites the wrong reviewers |
| DSL/build-system application riff | Different paper; keep out |

## Dependencies

### Internal
| Dependency | Impact | Risk |
|------------|--------|------|
| terraphim-core polyrepo builds green | All benchmarks blocked otherwise | Low |
| TerraphimGraph hybrid path still present in service | Core system claim | Low — tests reference it |
| Benchmark harness in monorepo | Reproducibility section | Medium — check its docs are current |

### External
| Dependency | Version/State | Risk | Alternative |
|------------|---------------|------|-------------|
| Cleora (Synerise) | OSS, pure Rust | Medium — version pinning | Re-implement cited variant; or drop pure-embedding arm and cite published numbers |
| Labelled evaluation corpus with relevance judgements | **Does not exist yet** | **High — this is the critical gap** | Construct small judgements via LLM-assisted + human spot-check (declare method) or use an established IR test collection for the lexical arm |
| Citation tooling (Typst/LaTeX/Zotero) | Alex preference unknown | Low | Ask |

## Risks and Unknowns

### Known Risks
| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| No labelled corpus → weak evaluation | High | High | Time-box: build ~50–100 judged queries on a fixed corpus (e.g., INCOSE handbook, Terraphim docs); declare the method honestly |
| Reviewers say "just TF-IDF + AC, no novelty" | Medium | High | Frame as systems/experience paper with measured trade-off (determinism/latency vs semantics); target practitioner venues |
| Cleora baseline unfair (tuning) | Medium | Medium | Use defaults + one sensitivity run; report both |
| Scope creep into hybrid implementation work | Medium | Medium | Paper evaluates what exists; hybrid alpha-sweep only if cheap |

### Open Questions (for Alex)
0. **Discourse original material — RESOLVED 2026-09-26**: correct hostname is
   `terraphim.discourse.group` (official Discourse hosting; Alex's first
   pointer used `.org`, which does not exist). Public, unauthenticated read
   access. Full topic JSON archived under
   `docs/research/paper-sources-discourse/topic-{5,9,11,13,14,15,16,17,18,19}.json`.
   Primary sources for the paper:
   - **T19 “Terraphim Graph embeddings optimisation”** (2023-11-09, alex):
     before/after performance screenshots + Vimeo video “Refactoring
     Terraphim Graph Embeddings in Rust — Results 2023-11-07”. The paper's
     documented genesis event.
   - **T17 “Terraphim AI — what is the difference?”** (2023-10-23, alex):
     “Privacy first: instead of moving data, we codify and move knowledge
     graphs, which allows us to build fast graph embeddings deployable even
     into the browser via wasm”; action-oriented ontologies; role-based
     lenses; YAGNI. Conceptual positioning, citable.
   - **T14 “What neuro-semantic reference architecture will survive?”**
     (2023-10-12, alex.turkhanov): recumbent-vs-upright socio-technical
     framing; trust architectures; Ciborra. Intro/related-work positioning.
   - **T16** use cases (INCOSE process model, role-based skills search);
     **T15** “Why search is broken” evidence thread (workplace search
     statistics); **T11** role-based search theory (WordNet roles);
     **T13** search-modes essay; **T9** release announcement (Rust rewrite,
     Firecracker/Tauri packaging); **T5** welcome; **T18** Innovate UK
     reports.
   **Terminology note (important for the paper)**: in 2023 usage,
   “Terraphim graph embeddings” denoted the project's own deterministic
   graph-rank structure (deployable via wasm) — not learned dense vectors.
   The paper must state this history explicitly: the 2023 "embeddings"
   = integer-rank co-occurrence graph; the academic embedding literature
   (node2vec/Cleora) is the *comparison*, and the internal 2024/25 Cleora
   analysis doc is the bridge.
1. **Venue/timeline**: workshop paper (SEA, MI-IR-style), practitioner venue
   (The Rust magazines are non-academic), arXiv-first, or a full conference
   submission? This sets length and experiment count.
2. **Authorship & affiliation**: solo Alex, or Alex + Kokoro (disclose AI
   assistance per venue policy)?
3. **Corpus choice**: Terraphim's own docs, INCOSE handbook (already used in
   CI), or a public IR collection (adds comparability, costs setup)?
4. **Pure-embedding arm in or out?** Comparing *existing* hybrid (rank+TF-IDF)
   against published Cleora numbers is cheap; running Cleora ourselves on the
   same corpus is fairer but more work.
5. **Writing toolchain**: LaTeX (Overleaf), Typst, or Pandoc-markdown?

### Assumptions Explicitly Stated
| Assumption | Basis | Risk if Wrong | Verified? |
|------------|-------|---------------|-----------|
| The hybrid TF-IDF+graph path is live code, not just a doc claim | Docs assert tests pass; scorer file exists | Paper's core system wouldn't exist → pivot to design/experience paper only | No — must run the tests |
| Sub-ms matching is achievable on the chosen corpus | AC is O(text); prior docs claim it | Latency claim weakened → measure, don't assert | No — must benchmark |
| Alex wants an *academic* (peer-reviewable) artefact, not a long blog | Explicit: "academic article" | If blog-style wanted, cut rigor; different plan | Partially — venue question above resolves it |

### Multiple Interpretations Considered
| Interpretation | Implications | Why Chosen/Rejected |
|----------------|--------------|---------------------|
| A: Experience/systems paper describing the architecture + measured trade-offs | Feasible now; practitioner-friendly | **Chosen** — matches existing artefacts |
| B: Novel-algorithm paper (new embedding method) | Requires new method + SOTA comparison | Rejected — no new algorithm exists in repo |
| C: Position/opinion paper ("deterministic semantic search is underrated") | Light experiments, heavy argument | Viable fallback if corpus work stalls; keep as Plan B |

## Research Findings

### Key Insights
1. The system's genuine differentiator is *inverting the embedding pipeline*:
   deterministic concept extraction → graph "embedding" (integer ranks in
   co-occurrence space) → optional learned enrichment, rather than the
   reverse.
2. An internal doc already frames the Cleora comparison — the academic article
   is largely a rigorous re-creation of that analysis with real benchmarks and
   related work.
3. The critical missing piece is an evaluation corpus with relevance
   judgements; everything else is buildable from the repo.
4. The project has a documented public genesis (Discourse, Oct–Nov 2023):
   positioning (T17), architecture-philosophy (T14), and a measurable
   optimisation event with screenshots and video (T19). The paper can cite a
   continuous 2023→2026 lineage — forum post → internal analysis doc → this
   article — and must pin down the terminology shift ("graph embeddings" once
   meant the deterministic rank graph itself).

### Relevant Prior Art (starting set, to be verified)
- **Terraphim Discourse primary sources (2023)**: T19 embeddings-optimisation
  post + Vimeo demo video (genesis artefacts); T17 positioning post;
  T14 neuro-semantic architecture thread. Archived in
  `docs/research/paper-sources-discourse/`.
- Cleora (Bien et al., 2021/2022, Synerise) — "widening" Chebyshev embeddings.
- Node2vec (Grover & Leskovec, 2016), DeepWalk (Perozzi et al., 2014) —
  classical graph embeddings.
- TF-IDF (Sparck Jones) and BM25 (Robertson & Zaragoza) — lexical scoring.
- Learning-to-rank / hybrid IR literature; reciprocal rank fusion (Cormack et
  al., 2009).
- Interpretability/explainability in IR (e.g., work on transparent ranking,
  EURLEX/Recital transparency discussions).
- Aho-Corasick (1975) itself; finite-state transducers (fst crate).
- Knowledge-graph-grounded IR (entity-linking literature: TAGME, GENRE-style
  retrieval) — closest neighbouring field; must position against entity
  linking explicitly.

### Technical Spikes Needed
| Spike | Purpose | Est. Effort | Status |
|-------|---------|-------------|--------|
| Run hybrid-path unit/bench tests on this machine | Verify live-code assumption | 1–2 h | ✅ **DONE 2026-09-26** — all green; full criterion sweep captured (see design doc Gate Status) |
| Mini relevance-judgement set (30–50 queries, 1 corpus) | Feasibility of evaluation | 1 day | Pending (E3) |
| Cleora reference run (or citation-only decision) | Decide baseline arm | 0.5–1 day | Default: citation-only |
| Venue scan (CFPs, 2026–27 deadlines) | Anchor the timeline | 2 h | Closed — arXiv-first chosen |

## Recommendations

### Proceed/No-Proceed
**Proceed** — Plan A (systems/experience paper) with Plan B (position paper)
fallback if the evaluation corpus proves too costly.

### Scope Recommendations
- One system, one corpus, two scoring arms (pure graph-rank vs hybrid
  rank+TF-IDF), optional third (Cleora) only if the reference run is cheap.
- Metrics: latency percentiles, MAP@10 / nDCG@10, memory footprint, update
  (indexing) throughput. Determinism demonstrated by identical-rank replay.

### Risk Mitigation Recommendations
- Lock venue + length *before* writing; draft experiments section first,
  intro last.
- Keep the Cleora arm citation-based until the corpus exists.

## Next Steps

If approved:
1. Resolve Open Questions 1–5 with Alex (venue, authorship, corpus, baseline
   arm, toolchain).
2. Phase 2: `design-paper-terraphim-graph-embeddings.md` — article skeleton,
   experiment plan, writing sequence, reproduction artefacts.
3. Phase 2.5 spec interview: pin down claims-vs-evidence table before any
   prose.

## Appendix

### Reference Materials
- `docs/src/scorers/graph-embedding-analysis.md` (monorepo) — internal
  Cleora comparison, the seed document.
- `terraphim-core/crates/terraphim_automata/src/matcher.rs` — word-boundary
  and minimum-length match logic worth citing in the systems description.
- `PERFORMANCE_BENCHMARKING_README.md`, `benchmark-config.json` — benchmark
  infrastructure.
- `blog/2026-02-16-zvec-vs-terraphim-comparison.md` — prior public framing.
- `~/projects/terraphim/terraphim_graph_embeddings_build_system.md` — the DSL
  riff conversation; explicitly *out of scope*, but shows the idea's pull.
