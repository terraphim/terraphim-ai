# Thesaurus Cache Flush

Concept from the AI Dark Factory book, verified against the implementation in `terraphim-ai`, `gitea-robot`, or the Zestic Gitea fork. Category: behaviour.

synonyms:: thesaurus flush, KG ingest verification

The compiled thesaurus cache flushes automatically after KG markdown edits (Refs #945, commit bf1b7f11c). Newly written haystack notes become searchable on the next terraphim-agent search call without manual reindex.
