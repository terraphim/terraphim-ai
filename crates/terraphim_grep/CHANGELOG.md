# Changelog

All notable changes to `terraphim_grep` are documented here.

## [1.20.0] - 2026-05-25

### Added
- Initial release: hybrid grep with KG boosting and LLM synthesis fallback.
- `boost_chunks_with_kg` re-ranks chunks so files matching thesaurus concepts rank higher.
- End-to-end hybrid pipeline: `fff-search` code retrieval + parallel KG concept extraction
  + KG-aware ranking boost + sufficiency judging + LLM synthesis with citations.
- CLI wires `terraphim_service::llm::build_llm_from_role` for provider-agnostic LLM access.
- Graceful degradation: returns chunks even without an LLM configured.
- Four-layer test pyramid (L1 inline, L2 router-capability, L3 e2e, L4 manual quality gate).
- Criterion benchmarks for `code_only`, `hybrid_with_kg`, `fuse_and_rank`, and
  `kg_boost_overhead`.

### Dependencies
- `fff-search` bumped from git to crates.io 0.8.2.
