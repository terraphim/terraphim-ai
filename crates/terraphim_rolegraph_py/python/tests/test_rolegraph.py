"""Tests for the RoleGraph class."""

from terraphim_rolegraph import Document, LogicalOperator, RoleGraph


class TestRoleGraphCreation:
    def test_constructor(self, sample_thesaurus_json):
        rg = RoleGraph("engineer", sample_thesaurus_json)
        assert rg.role == "engineer"

    def test_role_property(self, sample_thesaurus_json):
        rg = RoleGraph("data_scientist", sample_thesaurus_json)
        assert rg.role == "data_scientist"

    def test_empty_graph_stats(self, sample_thesaurus_json):
        rg = RoleGraph("test", sample_thesaurus_json)
        stats = rg.get_graph_stats()
        assert stats.node_count == 0
        assert stats.edge_count == 0
        assert stats.document_count == 0
        assert stats.thesaurus_size == 6
        assert stats.is_populated is False

    def test_repr(self, sample_thesaurus_json):
        rg = RoleGraph("engineer", sample_thesaurus_json)
        r = repr(rg)
        assert "RoleGraph" in r
        assert "engineer" in r

    def test_invalid_thesaurus_json(self):
        with __import__("pytest").raises(ValueError):
            RoleGraph("test", "not valid json")

    def test_len_empty(self, sample_thesaurus_json):
        rg = RoleGraph("test", sample_thesaurus_json)
        assert len(rg) == 0


class TestDocumentManagement:
    def test_insert_document(self, sample_thesaurus_json):
        rg = RoleGraph("test", sample_thesaurus_json)
        doc = Document("d1", "http://x.com", "T", "machine learning is great")
        rg.insert_document("d1", doc)
        assert rg.document_count() == 1

    def test_has_document(self, populated_rolegraph):
        assert populated_rolegraph.has_document("doc1") is True
        assert populated_rolegraph.has_document("nonexistent") is False

    def test_get_document(self, populated_rolegraph):
        doc = populated_rolegraph.get_document("doc1")
        assert doc is not None
        assert doc.id == "doc1"

    def test_get_document_none(self, populated_rolegraph):
        assert populated_rolegraph.get_document("nonexistent") is None

    def test_get_all_documents(self, populated_rolegraph):
        docs = populated_rolegraph.get_all_documents()
        assert len(docs) == 3
        ids = {doc_id for doc_id, _ in docs}
        assert ids == {"doc1", "doc2", "doc3"}

    def test_document_count(self, populated_rolegraph):
        assert populated_rolegraph.document_count() == 3

    def test_len(self, populated_rolegraph):
        assert len(populated_rolegraph) == 3

    def test_find_document_ids_for_term(self, populated_rolegraph):
        doc_ids = populated_rolegraph.find_document_ids_for_term("machine learning")
        assert "doc1" in doc_ids
        assert "doc3" in doc_ids


class TestGraphQueries:
    def test_query_graph(self, populated_rolegraph):
        results = populated_rolegraph.query_graph("machine learning")
        assert len(results) > 0
        # Results are tuples of (doc_id, IndexedDocument)
        for doc_id, indexed_doc in results:
            assert isinstance(doc_id, str)
            assert indexed_doc.rank > 0

    def test_query_graph_empty_result(self, populated_rolegraph):
        results = populated_rolegraph.query_graph("quantum computing")
        assert len(results) == 0

    def test_query_graph_with_limit(self, populated_rolegraph):
        results = populated_rolegraph.query_graph("artificial intelligence", limit=1)
        assert len(results) <= 1

    def test_query_graph_with_offset(self, populated_rolegraph):
        all_results = populated_rolegraph.query_graph("artificial intelligence")
        if len(all_results) > 1:
            offset_results = populated_rolegraph.query_graph(
                "artificial intelligence", offset=1
            )
            assert len(offset_results) == len(all_results) - 1

    def test_query_graph_with_operators_or(self, populated_rolegraph):
        results = populated_rolegraph.query_graph_with_operators(
            ["machine learning", "deep learning"], LogicalOperator.Or
        )
        assert len(results) > 0

    def test_query_graph_with_operators_and(self, populated_rolegraph):
        results = populated_rolegraph.query_graph_with_operators(
            ["machine learning", "deep learning"], LogicalOperator.And
        )
        # AND results should be a subset of OR results
        or_results = populated_rolegraph.query_graph_with_operators(
            ["machine learning", "deep learning"], LogicalOperator.Or
        )
        assert len(results) <= len(or_results)

    def test_find_matching_node_ids(self, populated_rolegraph):
        ids = populated_rolegraph.find_matching_node_ids(
            "machine learning and deep learning"
        )
        assert len(ids) >= 2

    def test_is_all_terms_connected_by_path(self, populated_rolegraph):
        # Terms that co-occur in the same document should be connected
        result = populated_rolegraph.is_all_terms_connected_by_path(
            "machine learning deep learning"
        )
        assert isinstance(result, bool)


class TestGraphStats:
    def test_node_count_increases(self, sample_thesaurus_json):
        rg = RoleGraph("test", sample_thesaurus_json)
        assert rg.get_node_count() == 0
        doc = Document(
            "d1", "http://x.com", "T", "machine learning and deep learning"
        )
        rg.insert_document("d1", doc)
        assert rg.get_node_count() > 0

    def test_edge_count_increases(self, sample_thesaurus_json):
        rg = RoleGraph("test", sample_thesaurus_json)
        assert rg.get_edge_count() == 0
        doc = Document(
            "d1", "http://x.com", "T", "machine learning and deep learning"
        )
        rg.insert_document("d1", doc)
        assert rg.get_edge_count() > 0

    def test_is_graph_populated(self, sample_thesaurus_json, populated_rolegraph):
        empty = RoleGraph("test", sample_thesaurus_json)
        assert empty.is_graph_populated() is False
        assert populated_rolegraph.is_graph_populated() is True

    def test_validate_documents(self, populated_rolegraph):
        warnings = populated_rolegraph.validate_documents()
        assert isinstance(warnings, list)

    def test_get_graph_stats(self, populated_rolegraph):
        stats = populated_rolegraph.get_graph_stats()
        assert stats.node_count > 0
        assert stats.edge_count > 0
        assert stats.document_count == 3
        assert stats.thesaurus_size == 6
        assert stats.is_populated is True


class TestSerialization:
    def test_to_json_roundtrip(self, populated_rolegraph):
        json_str = populated_rolegraph.to_json()
        assert isinstance(json_str, str)
        assert len(json_str) > 0

        restored = RoleGraph.from_json(json_str)
        assert restored.document_count() == populated_rolegraph.document_count()
        assert restored.get_node_count() == populated_rolegraph.get_node_count()
        assert restored.get_edge_count() == populated_rolegraph.get_edge_count()

    def test_to_json_pretty(self, populated_rolegraph):
        json_str = populated_rolegraph.to_json_pretty()
        assert "\n" in json_str

    def test_roundtrip_preserves_query_results(self, populated_rolegraph):
        json_str = populated_rolegraph.to_json()
        restored = RoleGraph.from_json(json_str)

        original_results = populated_rolegraph.query_graph("machine learning")
        restored_results = restored.query_graph("machine learning")
        assert len(original_results) == len(restored_results)

    def test_from_json_invalid(self):
        with __import__("pytest").raises((ValueError, RuntimeError)):
            RoleGraph.from_json("not valid json")


class TestInspection:
    def test_nodes_map(self, populated_rolegraph):
        nodes = populated_rolegraph.nodes_map()
        assert isinstance(nodes, dict)
        assert len(nodes) > 0
        for node_id, node in nodes.items():
            assert isinstance(node_id, int)
            assert node.id == node_id
            assert node.rank > 0

    def test_edges_map(self, populated_rolegraph):
        edges = populated_rolegraph.edges_map()
        assert isinstance(edges, dict)
        assert len(edges) > 0
        for edge_id, edge in edges.items():
            assert isinstance(edge_id, int)
            assert edge.id == edge_id
