# KgPathScorer

Scores file paths by counting knowledge-graph concept matches using Aho-Corasick automata. Implements ExternalScorer trait for fff-search, enabling hybrid KG+FFF file search. Hot-reloadable thesaurus. Returns min(unique_matches * weight_per_term, max_boost) as score boost.

synonyms:: kg scorer, path scorer, hybrid search, file scoring, KG boost, external scorer, terraphim_file_search
