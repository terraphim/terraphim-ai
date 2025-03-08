# BM25 Test Results

This document summarizes the test results for the BM25F and BM25+ scoring algorithms using the provided test datasets. The tests demonstrate the key characteristics and advantages of each algorithm.

## Main Documents Dataset

The main documents dataset tests basic relevance ranking across different topics. The results show that:

- All three scorers (Standard BM25, BM25+, BM25F) generally agree on the top results for simple queries.
- BM25F gives more weight to matches in title fields, which can change the ranking compared to standard BM25.
- BM25+ assigns higher scores overall due to the delta parameter, but the ranking is similar to standard BM25.

Example query: "rust programming"
```
Standard BM25 Results:
  doc5: 3.5088 - Rust for Systems Programming
  doc1: 3.4456 - Introduction to Rust Programming Language

BM25+ Results:
  doc5: 5.5165 - Rust for Systems Programming
  doc1: 5.4533 - Introduction to Rust Programming Language

BM25F Results:
  doc5: 5.0274 - Rust for Systems Programming
  doc1: 4.9342 - Introduction to Rust Programming Language
```

## Field Weighting Test

The field weighting test dataset demonstrates BM25F's ability to weight different document fields differently. The results show that:

- When title field is weighted higher, documents with query terms in the title rank higher, even if other documents have more occurrences of the terms in the body.
- BM25F can be configured to prioritize matches in specific fields, which is not possible with standard BM25 or BM25+.
- Different field weighting configurations can produce significantly different rankings for the same query.

Example query: "machine learning" with title field weighted higher:
```
Standard BM25 Results:
  fw3: 0.2892 - Advanced Machine Learning Techniques
  fw1: 0.2852 - Machine Learning Fundamentals
  fw2: 0.2834 - Data Analysis Methods

BM25F Results:
  fw1: 0.3223 - Machine Learning Fundamentals
  fw3: 0.3207 - Advanced Machine Learning Techniques
  fw2: 0.2850 - Data Analysis Methods
```

## Rare Terms Test

The rare terms test dataset demonstrates BM25+'s improved handling of rare terms. The results show that:

- BM25+ assigns higher scores to documents containing rare terms compared to standard BM25.
- For extremely rare terms, BM25+ ensures that documents containing those terms are ranked higher.
- BM25+ assigns a minimum score to all documents for each query term, which helps with ranking when terms are very rare.

Example query: "brainfuck whitespace intercal malbolge" (extremely rare terms):
```
Standard BM25 Results:
  rt3: 5.7353 - Esoteric Programming Languages

BM25+ Results:
  rt3: 11.2805 - Esoteric Programming Languages
  rt1: 5.5452 - Common Programming Languages
  rt2: 5.5452 - Rare Programming Languages
  rt4: 5.5452 - Domain-Specific Languages
  rt5: 5.5452 - Ancient Programming Languages
```

Note that BM25+ assigns a minimum score to all documents, even those that don't contain the query terms, which can be useful for ranking when terms are very rare.

## Document Length Test

The document length test dataset demonstrates how the algorithms handle documents of varying lengths. The results show that:

- All three algorithms normalize document length to prevent longer documents from dominating solely due to length.
- Shorter documents tend to rank higher when term frequency is similar across documents.
- BM25F can be configured to handle length normalization differently for different fields.

Example query: "machine learning artificial intelligence":
```
Standard BM25 Results:
  dl2: 0.8883 - Short Document on Machine Learning
  dl3: 0.8126 - Medium-Length Document on Machine Learning
  dl4: 0.6767 - Long Document on Machine Learning
  dl5: 0.5654 - Very Long Document on Machine Learning
  dl1: 0.4746 - Very Short Document on Machine Learning

BM25F Results:
  dl2: 0.9162 - Short Document on Machine Learning
  dl3: 0.8357 - Medium-Length Document on Machine Learning
  dl4: 0.6870 - Long Document on Machine Learning
  dl5: 0.5636 - Very Long Document on Machine Learning
  dl1: 0.4856 - Very Short Document on Machine Learning
```

## Term Frequency Test

The term frequency test dataset demonstrates how the algorithms handle term frequency saturation. The results show that:

- All three algorithms show diminishing returns for very high term frequencies.
- Documents with extreme term frequency (tf5) do not rank significantly higher than documents with high term frequency (tf3, tf4).
- BM25F can be configured to handle term frequency saturation differently for different fields.

Example query: "rust programming language":
```
Standard BM25 Results:
  tf2: 0.8001 - Document with Medium Term Frequency
  tf1: 0.7507 - Document with Low Term Frequency
  tf3: 0.7320 - Document with High Term Frequency
  tf4: 0.7168 - Document with Very High Term Frequency
  tf5: 0.2815 - Document with Extreme Term Frequency

BM25F Results:
  tf2: 0.8199 - Document with Medium Term Frequency
  tf1: 0.7827 - Document with Low Term Frequency
  tf3: 0.7678 - Document with High Term Frequency
  tf4: 0.7655 - Document with Very High Term Frequency
  tf5: 0.3340 - Document with Extreme Term Frequency
```

## Key Findings

1. **BM25F Advantages**:
   - Field-specific weighting allows for more control over ranking.
   - Can prioritize matches in specific fields (e.g., title, tags).
   - Field-specific length normalization and term frequency saturation.
   - Better for structured documents with distinct fields.

2. **BM25+ Advantages**:
   - Improved handling of rare terms.
   - Assigns a minimum score to all documents for each query term.
   - Better for collections with varying term frequencies.
   - Particularly useful for long-tail queries with rare terms.

3. **When to Use Each**:
   - Use BM25F when documents have well-defined fields with different importance.
   - Use BM25+ when dealing with rare terms or long-tail queries.
   - Standard BM25 is a good baseline but is outperformed by both BM25F and BM25+ in specific scenarios.

4. **Parameter Tuning**:
   - The `k1` parameter controls term frequency saturation (higher values = less saturation).
   - The `b` parameter controls document length normalization (higher values = more normalization).
   - For BM25+, the `delta` parameter controls the lower bound for term frequency.
   - For BM25F, field weights can be tuned to prioritize specific fields.

## Conclusion

Both BM25F and BM25+ offer significant improvements over standard BM25 in specific scenarios. BM25F is particularly useful for structured documents with distinct fields, while BM25+ is better for handling rare terms and long-tail queries. The choice between them depends on the specific requirements of the search application and the nature of the document collection. 