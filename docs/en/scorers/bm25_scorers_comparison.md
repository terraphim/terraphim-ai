# BM25 Scorers Comparison

This document provides a comparison of the different BM25 scoring algorithms implemented in Terraphim AI.

## Overview

Terraphim AI now supports multiple BM25-based scoring algorithms for document ranking:

1. **BM25F** - A field-weighted version of BM25 that handles multiple fields with different weights
2. **BM25+** - An improved version of BM25 that addresses the lower-bounding problem
3. **Okapi BM25** - The classic BM25 algorithm
4. **TFIDF** - The traditional Term Frequency-Inverse Document Frequency algorithm
5. **Jaccard** - A similarity measure based on the intersection over union of terms
6. **QueryRatio** - A simple measure based on the proportion of query terms in a document

## Algorithm Descriptions

### BM25F

BM25F extends the classic BM25 algorithm by allowing different parts of a document to be weighted differently. For example, a match in the title field might be considered more important than a match in the body field.

The formula for BM25F is:

```
score(D,Q) = ∑(t in Q) IDF(t) * (weighted_tf(t,D) / (K1 * length_norm + weighted_tf(t,D)))
```

Where:
- `D` is a document
- `Q` is the query
- `t` is a term in the query
- `IDF(t)` is the inverse document frequency of term `t`
- `weighted_tf(t,D)` is the weighted term frequency of `t` in document `D`
- `K1` is a parameter that controls term frequency saturation (default: 1.2)
- `length_norm` is the document length normalization factor

### BM25+

BM25+ is an improved version of BM25 that addresses the lower-bounding problem for term frequency normalization. It provides more accurate relevance scoring, especially for short documents and rare terms.

The formula for BM25+ is:

```
score(D,Q) = ∑(t in Q) IDF(t) * ((tf(t,D) * (K1 + 1)) / (K1 * length_norm + tf(t,D)) + delta)
```

Where:
- `delta` is a small constant that ensures a lower bound on term frequency (default: 1.0)

### Okapi BM25

Okapi BM25 is the classic BM25 algorithm that considers term frequency, inverse document frequency, and document length normalization.

The formula for Okapi BM25 is:

```
score(D,Q) = ∑(t in Q) IDF(t) * ((tf(t,D) * (K1 + 1)) / (K1 * length_norm + tf(t,D)))
```

### TFIDF

TF-IDF (Term Frequency-Inverse Document Frequency) is a classic information retrieval algorithm that weighs terms based on their frequency in a document and their rarity across the corpus.

The formula for TF-IDF is:

```
score(D,Q) = ∑(t in Q) tf(t,D) * idf(t)
```

Where:
- `tf(t,D)` is the term frequency of `t` in document `D`
- `idf(t)` is the inverse document frequency of term `t`

### Jaccard

Jaccard similarity is a statistical measure used to gauge the similarity and diversity of sample sets. In the context of document search, it measures the overlap between query terms and document terms.

The formula for Jaccard similarity is:

```
J(A,B) = |A ∩ B| / |A ∪ B|
```

For document search:
- A is the set of terms in the query
- B is the set of terms in the document
- The intersection is the number of terms that appear in both the query and document
- The union is the total number of unique terms in both the query and document

#### Implementation Details

The Jaccard similarity implementation in Terraphim AI follows this approach:

1. Tokenize the query and document into term sets
2. Calculate the intersection (terms present in both sets)
3. Calculate the union (all unique terms from both sets)
4. Compute the Jaccard coefficient as intersection/union

```rust
pub fn score(&self, query: &str, doc: &Document) -> f64 {
    let query_terms: Vec<String> = query.split_whitespace()
        .map(|s| s.to_lowercase())
        .collect();
    
    if query_terms.is_empty() || self.doc_count == 0 {
        return 0.0;
    }
    
    let doc_terms: Vec<String> = doc.body.split_whitespace()
        .map(|s| s.to_lowercase())
        .collect();
    
    if doc_terms.is_empty() {
        return 0.0;
    }
    
    // Create sets for query and document terms
    let query_set: std::collections::HashSet<&String> = query_terms.iter().collect();
    let doc_set: std::collections::HashSet<&String> = doc_terms.iter().collect();
    
    // Calculate intersection size
    let intersection_size = query_set.intersection(&doc_set).count();
    
    // Calculate union size
    let union_size = query_set.union(&doc_set).count();
    
    // Jaccard similarity formula
    if union_size > 0 {
        intersection_size as f64 / union_size as f64
    } else {
        0.0
    }
}
```

#### Key Characteristics

- **Range**: Always between 0 and 1, where 0 means no overlap and 1 means identical sets
- **Simplicity**: Easy to understand and implement
- **Term Presence**: Focuses on term presence/absence rather than frequency
- **Equal Weighting**: All terms are given equal weight
- **Normalization**: Naturally normalized by the size of the union

#### Testing and Validation

The Jaccard similarity implementation has been thoroughly tested with:

1. **Basic Validation Tests**: Using documents with predictable term overlap to verify scores match expected values
2. **Edge Case Tests**: Ensuring correct behavior with empty queries, empty documents, and documents with no overlap
3. **Comparison Tests**: Comparing Jaccard scores with other scoring methods like QueryRatio and TFIDF
4. **Visualization Tests**: Demonstrating the calculation process for better understanding

All tests confirm that the Jaccard similarity implementation correctly measures the overlap between query and document terms, providing a simple yet effective scoring mechanism.

### QueryRatio

Query Ratio measures the proportion of query terms that appear in a document, providing a simple but effective relevance measure.

The formula for Query Ratio is:

```
score(D,Q) = |Q ∩ D| / |Q|
```

Where:
- `|Q ∩ D|` is the number of terms that appear in both the query and the document
- `|Q|` is the number of terms in the query

## Implementation Details

All scorers follow a similar implementation pattern:

1. **Initialization**: Each scorer is initialized with a corpus of documents to calculate statistics like average document length and term document frequencies.
2. **Scoring**: Each scorer provides a `score` method that takes a query and a document and returns a relevance score.

### Parameters

The BM25-based scorers (BM25F, BM25+, Okapi BM25) share common parameters:

- `k1`: Controls term frequency saturation (default: 1.2)
- `b`: Controls document length normalization (default: 0.75)
- `delta`: Used by BM25+ to ensure a lower bound on term frequency (default: 1.0)

BM25F also uses field weights:

- `title`: Weight for document title (default: 3.0)
- `body`: Weight for document body (default: 1.0)
- `description`: Weight for document description (default: 2.0)
- `tags`: Weight for document tags (default: 2.5)

## When to Use Each Scorer

- **BM25F**: Use when you have structured documents with multiple fields and want to prioritize matches in certain fields.
- **BM25+**: Use when you need to handle rare terms better and want improved performance on short documents.
- **Okapi BM25**: Use as a general-purpose scorer when you don't need field weighting or special handling of rare terms.
- **TFIDF**: Use when you want a simpler scoring algorithm that still considers term frequency and document frequency.
- **Jaccard**: Use when you want to measure the similarity between the query and document based on the overlap of terms. Particularly useful for:
  - Comparing short texts or documents
  - Measuring similarity between sets of tags or categories
  - Finding documents with similar vocabulary, regardless of term frequency
  - When the presence/absence of terms is more important than their frequency
  - When you need a normalized score between 0 and 1
- **QueryRatio**: Use when you want a simple measure of how many query terms appear in a document.

## Test Results

The scorers have been tested on a variety of queries and documents. The tests show that:

1. BM25F and BM25+ generally produce similar rankings, with BM25F giving more weight to matches in title fields.
2. BM25+ handles rare terms better than Okapi BM25.
3. All scorers normalize document length to avoid bias toward longer documents.
4. All scorers handle term frequency saturation, showing diminishing returns for very high term frequencies.
5. All scorers rank relevant documents higher than non-relevant ones.

## Scorer Comparison

The following table summarizes the key characteristics of each scorer:

| Scorer | Score Range | Considers Term Frequency | Considers Document Length | Field Weighting | Handles Rare Terms | Complexity | Best Use Case |
|--------|-------------|--------------------------|---------------------------|-----------------|-------------------|------------|--------------|
| BM25F | Unbounded | Yes | Yes | Yes | Moderate | High | Structured documents with multiple fields |
| BM25+ | Unbounded | Yes | Yes | No | Excellent | Medium | Short documents and rare terms |
| Okapi BM25 | Unbounded | Yes | Yes | No | Good | Medium | General-purpose search |
| TFIDF | Unbounded | Yes | No | No | Good | Low | Simple relevance ranking |
| Jaccard | 0 to 1 | No | No | No | Poor | Low | Term overlap measurement |
| QueryRatio | 0 to 1 | No | No | No | Poor | Low | Query term coverage |

## Usage

To use these scorers in Terraphim AI, specify the desired relevance function in your search query:

```rust
let search_query = SearchQuery {
    search_term: NormalizedTermValue::new("your query".to_string()),
    skip: None,
    limit: None,
    role: Some(RoleName::new("your role")),
};

// Use the BM25F scorer
let documents = score::rescore_documents(&search_query, documents, RelevanceFunction::BM25F);

// Use the BM25+ scorer
let documents = score::rescore_documents(&search_query, documents, RelevanceFunction::BM25Plus);

// Use the Okapi BM25 scorer
let documents = score::rescore_documents(&search_query, documents, RelevanceFunction::OkapiBM25);

// Use the TFIDF scorer
let documents = score::rescore_documents(&search_query, documents, RelevanceFunction::TFIDF);

// Use the Jaccard scorer
let documents = score::rescore_documents(&search_query, documents, RelevanceFunction::Jaccard);

// Use the QueryRatio scorer
let documents = score::rescore_documents(&search_query, documents, RelevanceFunction::QueryRatio);
```

## References

1. Robertson, S. E., & Zaragoza, H. (2009). The probabilistic relevance framework: BM25 and beyond. Foundations and Trends in Information Retrieval, 3(4), 333-389.
2. Lv, Y., & Zhai, C. (2011). Lower-bounding term frequency normalization. In Proceedings of the 20th ACM international conference on Information and knowledge management (pp. 7-16).
3. Robertson, S. E., Walker, S., Jones, S., Hancock-Beaulieu, M. M., & Gatford, M. (1995). Okapi at TREC-3. NIST SPECIAL PUBLICATION SP, 109-109.
4. Salton, G., & Buckley, C. (1988). Term-weighting approaches in automatic text retrieval. Information processing & management, 24(5), 513-523.
5. Jaccard, P. (1901). Étude comparative de la distribution florale dans une portion des Alpes et des Jura. Bulletin de la Société Vaudoise des Sciences Naturelles, 37, 547-579. 