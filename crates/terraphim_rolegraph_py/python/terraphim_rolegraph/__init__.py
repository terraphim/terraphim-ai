"""Terraphim RoleGraph - Knowledge graph operations for AI agents.

Python bindings for the Rust terraphim_rolegraph crate, providing
document insertion, graph queries, path connectivity, serialization,
and statistics for role-based knowledge graphs.
"""

from terraphim_rolegraph.terraphim_rolegraph import (
    Document,
    Edge,
    GraphStats,
    IndexedDocument,
    LogicalOperator,
    Node,
    RoleGraph,
    __version__,
    magic_pair,
    magic_unpair,
    split_paragraphs,
)

__all__ = [
    "RoleGraph",
    "Document",
    "IndexedDocument",
    "Node",
    "Edge",
    "GraphStats",
    "LogicalOperator",
    "magic_pair",
    "magic_unpair",
    "split_paragraphs",
    "__version__",
]
