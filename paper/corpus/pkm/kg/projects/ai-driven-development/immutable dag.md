# Immutable DAG

An immutable directed acyclic graph used as the storage model for agent context (as in CXDB). Preserves the full branching history of agent interactions -- conversations, tool calls, decisions, and outputs -- in a structure that cannot be retroactively modified. Turn nodes contain monotonically-increasing IDs, parent references, depth from root, and BLAKE3 content hashes. Enables audit, replay, and debugging of autonomous agent workflows.

synonyms:: immutable directed acyclic graph, context dag, agent history graph, turn dag, agent audit trail, conversation graph, branching conversation history
