#!/usr/bin/env python3
"""
BM25 Scorer Test Utility

This script helps test and evaluate BM25F and BM25+ scoring algorithms using
the provided test datasets. It loads the test documents, runs queries through
different scoring algorithms, and compares the results.

Usage:
    python test_bm25_scorers.py [--dataset DATASET] [--query QUERY_ID]

Options:
    --dataset DATASET    Specify which dataset to use (default: documents.json)
    --query QUERY_ID     Specify a specific query ID to test (default: all queries)
"""

import argparse
import json
import math
import os
from collections import Counter, defaultdict
from typing import Dict, List, Set, Tuple, Union


class BM25Scorer:
    """Base class for BM25 scoring algorithms."""
    
    def __init__(self, documents, k1=1.2, b=0.75):
        self.documents = documents
        self.k1 = k1
        self.b = b
        self.doc_count = len(documents)
        self.avg_doc_length = 0
        self.doc_lengths = {}
        self.term_frequencies = defaultdict(lambda: defaultdict(int))
        self.document_frequencies = defaultdict(int)
        
        self._preprocess_documents()
    
    def _preprocess_documents(self):
        """Preprocess documents to calculate necessary statistics."""
        total_length = 0
        
        for doc_id, doc in self.documents.items():
            # Combine all text fields for simplicity
            text = f"{doc.get('title', '')} {doc.get('body', '')} {doc.get('description', '')} {' '.join(doc.get('tags', []))}"
            terms = self._tokenize(text)
            
            # Document length
            doc_length = len(terms)
            self.doc_lengths[doc_id] = doc_length
            total_length += doc_length
            
            # Term frequencies
            term_counts = Counter(terms)
            for term, count in term_counts.items():
                self.term_frequencies[doc_id][term] = count
                self.document_frequencies[term] += 1
        
        self.avg_doc_length = total_length / max(1, self.doc_count)
    
    def _tokenize(self, text: str) -> List[str]:
        """Simple tokenization by splitting on whitespace and lowercasing."""
        return [term.lower() for term in text.split()]
    
    def score(self, query: str) -> Dict[str, float]:
        """Score documents for the given query."""
        raise NotImplementedError("Subclasses must implement this method")


class BM25StandardScorer(BM25Scorer):
    """Standard BM25 scoring algorithm."""
    
    def score(self, query: str) -> Dict[str, float]:
        """Score documents for the given query using standard BM25."""
        query_terms = self._tokenize(query)
        scores = defaultdict(float)
        
        for term in query_terms:
            if term not in self.document_frequencies:
                continue
                
            idf = math.log((self.doc_count - self.document_frequencies[term] + 0.5) / 
                          (self.document_frequencies[term] + 0.5) + 1.0)
            
            for doc_id in self.documents:
                if term in self.term_frequencies[doc_id]:
                    tf = self.term_frequencies[doc_id][term]
                    doc_length = self.doc_lengths[doc_id]
                    
                    # BM25 term frequency component
                    numerator = tf * (self.k1 + 1)
                    denominator = tf + self.k1 * (1 - self.b + self.b * doc_length / self.avg_doc_length)
                    scores[doc_id] += idf * (numerator / denominator)
        
        return dict(scores)


class BM25PlusScorer(BM25Scorer):
    """BM25+ scoring algorithm with improved handling of rare terms."""
    
    def __init__(self, documents, k1=1.2, b=0.75, delta=1.0):
        super().__init__(documents, k1, b)
        self.delta = delta
    
    def score(self, query: str) -> Dict[str, float]:
        """Score documents for the given query using BM25+."""
        query_terms = self._tokenize(query)
        scores = defaultdict(float)
        
        for term in query_terms:
            if term not in self.document_frequencies:
                continue
                
            idf = math.log((self.doc_count - self.document_frequencies[term] + 0.5) / 
                          (self.document_frequencies[term] + 0.5) + 1.0)
            
            for doc_id in self.documents:
                tf = self.term_frequencies[doc_id].get(term, 0)
                doc_length = self.doc_lengths[doc_id]
                
                # BM25+ term frequency component with delta
                numerator = tf * (self.k1 + 1)
                denominator = tf + self.k1 * (1 - self.b + self.b * doc_length / self.avg_doc_length)
                tf_component = (numerator / denominator) + self.delta
                
                scores[doc_id] += idf * tf_component
        
        return dict(scores)


class BM25FScorer(BM25Scorer):
    """BM25F scoring algorithm with field weighting."""
    
    def __init__(self, documents, field_weights=None, k1=1.2, b=0.75, debug=False):
        self.debug = debug
        
        # Check if documents have the required fields
        self.has_fields = True
        sample_doc = next(iter(documents.values())) if documents else {}
        required_fields = ['title', 'body', 'description', 'tags']
        for field in required_fields:
            if field not in sample_doc:
                print(f"Warning: Field '{field}' not found in documents. BM25F may not work correctly.")
                self.has_fields = False
        
        self.field_weights = field_weights or {
            'title': 3.0,
            'body': 1.0,
            'description': 1.5,
            'tags': 2.0
        }
        self.fields = list(self.field_weights.keys())
        
        # Initialize with empty preprocessing
        self.documents = documents
        self.k1 = k1
        self.b = b
        self.doc_count = len(documents)
        
        # Field-specific statistics
        self.field_lengths = {field: {} for field in self.fields}
        self.avg_field_lengths = {field: 0 for field in self.fields}
        self.field_term_frequencies = {field: defaultdict(lambda: defaultdict(int)) for field in self.fields}
        
        # Document frequencies are shared across fields
        self.document_frequencies = defaultdict(int)
        
        self._preprocess_documents()
    
    def _preprocess_documents(self):
        """Preprocess documents to calculate field-specific statistics."""
        field_total_lengths = {field: 0 for field in self.fields}
        
        # If documents don't have the required fields, fall back to standard BM25
        if not self.has_fields:
            print("Falling back to standard BM25 preprocessing for BM25F due to missing fields.")
            super()._preprocess_documents()
            return
        
        # First, collect all terms across all fields to calculate document frequencies
        all_terms = set()
        for doc_id, doc in self.documents.items():
            doc_terms = set()
            for field in self.fields:
                if field == 'tags' and isinstance(doc.get(field), list):
                    # Handle tags as a space-separated string
                    text = ' '.join(doc.get(field, []))
                else:
                    text = doc.get(field, '')
                
                # Skip if field doesn't exist
                if text is None:
                    text = ''
                
                terms = self._tokenize(text)
                doc_terms.update(terms)
            
            # Update document frequencies
            for term in doc_terms:
                self.document_frequencies[term] += 1
            all_terms.update(doc_terms)
        
        if self.debug:
            print(f"Total unique terms across all documents: {len(all_terms)}")
            print(f"Sample terms: {list(all_terms)[:10]}")
        
        # Now process each field separately for term frequencies and field lengths
        for doc_id, doc in self.documents.items():
            for field in self.fields:
                if field == 'tags' and isinstance(doc.get(field), list):
                    # Handle tags as a space-separated string
                    text = ' '.join(doc.get(field, []))
                else:
                    text = doc.get(field, '')
                
                # Skip if field doesn't exist
                if text is None:
                    text = ''
                
                terms = self._tokenize(text)
                
                # Field length
                field_length = len(terms)
                self.field_lengths[field][doc_id] = field_length
                field_total_lengths[field] += field_length
                
                # Field term frequencies
                term_counts = Counter(terms)
                for term, count in term_counts.items():
                    self.field_term_frequencies[field][doc_id][term] = count
        
        # Calculate average field lengths
        for field in self.fields:
            self.avg_field_lengths[field] = field_total_lengths[field] / max(1, self.doc_count)
            print(f"Average length for field '{field}': {self.avg_field_lengths[field]:.2f}")
        
        if self.debug:
            # Print document frequencies for some terms
            sample_terms = list(all_terms)[:5]
            for term in sample_terms:
                print(f"Document frequency for term '{term}': {self.document_frequencies[term]}")
            
            # Print term frequencies for a sample document
            sample_doc_id = next(iter(self.documents.keys()))
            print(f"Term frequencies for document {sample_doc_id}:")
            for field in self.fields:
                if sample_doc_id in self.field_term_frequencies[field]:
                    print(f"  Field '{field}': {dict(self.field_term_frequencies[field][sample_doc_id])}")
    
    def score(self, query: str) -> Dict[str, float]:
        """Score documents for the given query using BM25F with field weighting."""
        # If documents don't have the required fields, fall back to standard BM25
        if not self.has_fields:
            print("Using standard BM25 scoring for BM25F due to missing fields.")
            standard_scorer = BM25StandardScorer(self.documents, self.k1, self.b)
            return standard_scorer.score(query)
        
        query_terms = self._tokenize(query)
        if self.debug:
            print(f"Query terms: {query_terms}")
            for term in query_terms:
                print(f"Document frequency for query term '{term}': {self.document_frequencies.get(term, 0)}")
        
        scores = defaultdict(float)
        
        for term in query_terms:
            if term not in self.document_frequencies:
                if self.debug:
                    print(f"Term '{term}' not found in any document")
                continue
                
            idf = math.log((self.doc_count - self.document_frequencies[term] + 0.5) / 
                          (self.document_frequencies[term] + 0.5) + 1.0)
            if self.debug:
                print(f"Term '{term}' IDF: {idf:.4f}")
            
            for doc_id in self.documents:
                # Calculate weighted term frequency across all fields
                weighted_tf = 0
                if self.debug:
                    print(f"Processing document {doc_id} for term '{term}'")
                
                for field in self.fields:
                    # Skip if field doesn't exist for this document
                    if doc_id not in self.field_lengths[field]:
                        if self.debug:
                            print(f"  Field '{field}' not found in document {doc_id}")
                        continue
                    
                    field_tf = self.field_term_frequencies[field][doc_id].get(term, 0)
                    if field_tf == 0:
                        continue
                        
                    field_length = self.field_lengths[field][doc_id]
                    avg_field_length = self.avg_field_lengths[field]
                    weight = self.field_weights[field]
                    
                    # BM25F per-field term frequency component
                    field_component = weight * field_tf / (1 - self.b + self.b * field_length / avg_field_length)
                    weighted_tf += field_component
                    
                    if self.debug:
                        print(f"  Field '{field}': tf={field_tf}, length={field_length}, avg_length={avg_field_length:.2f}, weight={weight}, component={field_component:.4f}")
                
                # Apply k1 saturation to the weighted term frequency
                if weighted_tf > 0:
                    tf_component = weighted_tf * (self.k1 + 1) / (self.k1 + weighted_tf)
                    score_contribution = idf * tf_component
                    scores[doc_id] += score_contribution
                    
                    if self.debug:
                        print(f"  Weighted TF: {weighted_tf:.4f}, TF component: {tf_component:.4f}, Score contribution: {score_contribution:.4f}")
        
        return dict(scores)


def load_dataset(dataset_path: str) -> Tuple[Dict, List]:
    """Load test dataset from JSON file."""
    with open(dataset_path, 'r') as f:
        data = json.load(f)
    
    # Handle different dataset formats
    if 'documents' in data:
        documents = {doc['id']: doc for doc in data['documents']}
        queries = data.get('queries', [])
    else:
        # If it's just an array of documents
        if isinstance(data, list):
            documents = {doc['id']: doc for doc in data}
        else:
            # If it's a direct object of documents
            documents = {doc_id: doc for doc_id, doc in data.items()}
        
        # Try to load queries from a separate file
        queries_path = os.path.join(os.path.dirname(dataset_path), 'queries.json')
        if os.path.exists(queries_path):
            with open(queries_path, 'r') as f:
                queries_data = json.load(f)
                queries = queries_data.get('queries', [])
        else:
            queries = []
    
    # Print some debug info
    print(f"Loaded {len(documents)} documents and {len(queries)} queries from {dataset_path}")
    if len(documents) > 0:
        print(f"Sample document keys: {list(documents.keys())[:3]}")
        sample_doc = next(iter(documents.values()))
        print(f"Sample document fields: {list(sample_doc.keys())}")
    if len(queries) > 0:
        print(f"Sample query IDs: {[q.get('id') for q in queries[:3]]}")
    
    return documents, queries


def run_test(dataset_path: str, query_id: str = None, debug: bool = False):
    """Run tests on the specified dataset and query."""
    documents, queries = load_dataset(dataset_path)
    
    if not documents:
        print(f"No documents found in {dataset_path}")
        return
    
    # Always try to load queries.json if it exists
    script_dir = os.path.dirname(os.path.abspath(__file__))
    queries_path = os.path.join(script_dir, 'queries.json')
    if os.path.exists(queries_path) and not queries:
        print(f"Loading queries from {queries_path}")
        with open(queries_path, 'r') as f:
            queries_data = json.load(f)
            queries = queries_data.get('queries', [])
        print(f"Loaded {len(queries)} queries from {queries_path}")
        if len(queries) > 0:
            print(f"Sample query IDs: {[q.get('id') for q in queries[:3]]}")
    
    # Initialize scorers
    bm25_standard = BM25StandardScorer(documents)
    bm25_plus = BM25PlusScorer(documents)
    bm25f = BM25FScorer(documents, debug=debug)
    
    # Filter queries if query_id is specified
    if query_id:
        queries = [q for q in queries if q.get('id') == query_id]
        if not queries:
            print(f"Query ID '{query_id}' not found in dataset")
            return
    
    # If no queries are available, create a simple one for testing
    if not queries:
        print("No queries found. Creating a simple test query.")
        # Extract some terms from the first document for a test query
        first_doc = list(documents.values())[0]
        title_terms = first_doc.get('title', '').split()[:2]
        test_query = ' '.join(title_terms)
        queries = [{'id': 'test_query', 'query': test_query, 'description': 'Auto-generated test query'}]
    
    # Run queries
    for query in queries:
        print(f"\nQuery: {query.get('id', 'unknown')} - '{query.get('query', '')}'")
        print(f"Description: {query.get('description', 'N/A')}")
        print("-" * 80)
        
        # Get scores from each scorer
        standard_scores = bm25_standard.score(query['query'])
        plus_scores = bm25_plus.score(query['query'])
        f_scores = bm25f.score(query['query'])
        
        # Sort documents by score
        standard_ranked = sorted(standard_scores.items(), key=lambda x: x[1], reverse=True)
        plus_ranked = sorted(plus_scores.items(), key=lambda x: x[1], reverse=True)
        f_ranked = sorted(f_scores.items(), key=lambda x: x[1], reverse=True)
        
        # Print results
        print("Standard BM25 Results:")
        for doc_id, score in standard_ranked[:5]:
            print(f"  {doc_id}: {score:.4f} - {documents[doc_id]['title']}")
        
        print("\nBM25+ Results:")
        for doc_id, score in plus_ranked[:5]:
            print(f"  {doc_id}: {score:.4f} - {documents[doc_id]['title']}")
        
        print("\nBM25F Results:")
        for doc_id, score in f_ranked[:5]:
            if f_ranked:
                print(f"  {doc_id}: {score:.4f} - {documents[doc_id]['title']}")
            else:
                print("  No results")
        
        # Check expected results if available
        if 'expected_results' in query:
            print("\nExpected Results:")
            for scorer, expected_docs in query['expected_results'].items():
                print(f"  {scorer}: {', '.join(expected_docs)}")


def main():
    """Main entry point for the script."""
    parser = argparse.ArgumentParser(description="Test BM25 scoring algorithms")
    parser.add_argument("--dataset", default="documents.json", help="Dataset file to use")
    parser.add_argument("--query", help="Specific query ID to test")
    parser.add_argument("--debug", action="store_true", help="Enable debug output")
    args = parser.parse_args()
    
    # Resolve dataset path
    script_dir = os.path.dirname(os.path.abspath(__file__))
    dataset_path = os.path.join(script_dir, args.dataset)
    
    if not os.path.exists(dataset_path):
        print(f"Dataset file not found: {dataset_path}")
        return
    
    run_test(dataset_path, args.query, args.debug)


if __name__ == "__main__":
    main() 