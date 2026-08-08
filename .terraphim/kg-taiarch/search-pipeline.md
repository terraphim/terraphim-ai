# Search Pipeline

Query → thesaurus expansion (automata) → haystack fetch → merge → relevance rank → results. Async via tokio; middleware crate provides the axum service.

synonyms:: pipeline, search flow, query expansion, middleware, axum, async search
