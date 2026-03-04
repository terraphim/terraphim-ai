"""Tests for wrapper types (Document, IndexedDocument, Node, Edge, GraphStats)."""

from terraphim_rolegraph import Document, GraphStats, LogicalOperator, RoleGraph


class TestDocument:
    def test_constructor_required_fields(self):
        doc = Document("d1", "http://x.com", "Title", "Body text")
        assert doc.id == "d1"
        assert doc.url == "http://x.com"
        assert doc.title == "Title"
        assert doc.body == "Body text"
        assert doc.description is None
        assert doc.tags is None
        assert doc.rank is None

    def test_constructor_all_fields(self):
        doc = Document(
            "d1",
            "http://x.com",
            "Title",
            "Body",
            description="A description",
            tags=["tag1", "tag2"],
            rank=42,
        )
        assert doc.description == "A description"
        assert doc.tags == ["tag1", "tag2"]
        assert doc.rank == 42

    def test_setters(self):
        doc = Document("d1", "http://x.com", "T", "B")
        doc.id = "d2"
        doc.title = "New Title"
        doc.body = "New Body"
        assert doc.id == "d2"
        assert doc.title == "New Title"
        assert doc.body == "New Body"

    def test_repr(self):
        doc = Document("d1", "http://x.com", "Title", "Body")
        r = repr(doc)
        assert "Document" in r
        assert "d1" in r
        assert "Title" in r


class TestIndexedDocument:
    def test_attributes(self, populated_rolegraph):
        doc = populated_rolegraph.get_document("doc1")
        assert doc is not None
        assert doc.id == "doc1"
        assert isinstance(doc.rank, int)
        assert isinstance(doc.tags, list)
        assert isinstance(doc.nodes, list)
        assert isinstance(doc.matched_edges, list)


class TestNode:
    def test_node_attributes(self, populated_rolegraph):
        nodes = populated_rolegraph.nodes_map()
        assert len(nodes) > 0
        node = next(iter(nodes.values()))
        assert isinstance(node.id, int)
        assert isinstance(node.rank, int)
        assert node.rank > 0
        assert isinstance(node.connected_with, set)
        assert len(node.connected_with) > 0

    def test_node_repr(self, populated_rolegraph):
        nodes = populated_rolegraph.nodes_map()
        node = next(iter(nodes.values()))
        r = repr(node)
        assert "Node" in r


class TestEdge:
    def test_edge_attributes(self, populated_rolegraph):
        edges = populated_rolegraph.edges_map()
        assert len(edges) > 0
        edge = next(iter(edges.values()))
        assert isinstance(edge.id, int)
        assert isinstance(edge.rank, int)
        assert isinstance(edge.doc_hash, dict)
        assert len(edge.doc_hash) > 0

    def test_edge_repr(self, populated_rolegraph):
        edges = populated_rolegraph.edges_map()
        edge = next(iter(edges.values()))
        r = repr(edge)
        assert "Edge" in r


class TestGraphStats:
    def test_all_fields(self, populated_rolegraph):
        stats = populated_rolegraph.get_graph_stats()
        assert isinstance(stats.node_count, int)
        assert isinstance(stats.edge_count, int)
        assert isinstance(stats.document_count, int)
        assert isinstance(stats.thesaurus_size, int)
        assert isinstance(stats.is_populated, bool)

    def test_repr(self, populated_rolegraph):
        stats = populated_rolegraph.get_graph_stats()
        r = repr(stats)
        assert "GraphStats" in r


class TestLogicalOperator:
    def test_and_value(self):
        assert LogicalOperator.And == LogicalOperator.And

    def test_or_value(self):
        assert LogicalOperator.Or == LogicalOperator.Or

    def test_and_or_different(self):
        assert LogicalOperator.And != LogicalOperator.Or
