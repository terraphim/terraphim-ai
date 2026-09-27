# Capability

Concept from the AI Dark Factory book, verified against the implementation in `terraphim-ai`, `gitea-robot`, or the Zestic Gitea fork. Category: enum.

synonyms:: Capability enum, capability-based routing

11-variant enum in crates/terraphim_types/src/capability.rs used by terraphim_router to pick the right provider. Variants: DeepThinking, FastThinking, CodeGeneration, CodeReview, Architecture, Testing, Refactoring, Documentation, Explanation, SecurityAudit, Performance.
