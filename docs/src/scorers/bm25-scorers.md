# BM25 Scorers

Terraphim supports multiple BM25-based scoring algorithms for document ranking and relevance calculation. These scorers provide different approaches to information retrieval, from traditional TF-IDF to advanced fielded BM25 variants.

## Available BM25 Scorers

### BM25 (Okapi BM25)
The standard Okapi BM25 ranking function, which is a probabilistic relevance function based on term frequency and inverse document frequency.

**Characteristics:**
- Combines term frequency (TF) with inverse document frequency (IDF)
- Incorporates document length normalization
- Uses parameters k1 (term frequency saturation) and b (length normalization)

**Use Case:** General-purpose document ranking with good balance between precision and recall.

### BM25F (Fielded BM25)
A fielded version of BM25 that applies different weights to different document fields (title, body, description, tags).

**Characteristics:**
- Different weights for different document fields
- Title typically gets higher weight than body text
- Description and tags can be weighted according to importance
- More sophisticated than standard BM25 for structured documents

**Use Case:** When document structure matters and you want to emphasize certain fields over others.

### BM25Plus
An enhanced version of BM25 with additional parameters for fine-tuning the ranking algorithm.

**Characteristics:**
- Additional delta parameter for term frequency adjustment
- More granular control over scoring behavior
- Enhanced parameter sensitivity for specialized use cases

**Use Case:** When you need fine-grained control over the ranking algorithm parameters.

### TFIDF (Term Frequency-Inverse Document Frequency)
The traditional TF-IDF ranking function that does not incorporate document length.

**Characteristics:**
- Simple and well-understood algorithm
- No document length normalization
- Pure statistical approach to term importance

**Use Case:** When you want a simple, interpretable scoring method without length bias.

### Jaccard Similarity
A ranking function determined by computing the similarity of ngrams between the query and document content.

**Characteristics:**
- Based on set intersection over set union
- Scores range from 0.0 to 1.0
- Considers term overlap rather than frequency

**Use Case:** When you want to measure how much query terms overlap with document content.

### QueryRatio
A ranking function that represents the ratio of query terms that matched a document.

**Characteristics:**
- Computes ratio of matched terms to total query terms
- Scores range from 0.0 to 1.0
- Simple and interpretable

**Use Case:** When you want to measure what percentage of query terms appear in a document.

## Configuration

BM25 scorers can be selected via the `QueryScorer` enum in your configuration:

```json
{
  "relevance_function": "title-scorer",
  "query_scorer": "bm25f"
}
```

Available options:
- `okapibm25` - Standard Okapi BM25
- `bm25` - BM25 implementation
- `bm25f` - Fielded BM25
- `bm25plus` - Enhanced BM25
- `tfidf` - Traditional TF-IDF
- `jaccard` - Jaccard similarity
- `queryratio` - Query ratio matching

## Parameters

### BM25 Parameters
- **k1**: Controls term frequency saturation (default: 1.2)
- **b**: Controls length normalization (default: 0.75)
- **delta**: Additional term frequency adjustment (BM25Plus only, default: 1.0)

### Field Weights (BM25F)
- **title**: Weight for document title (default: 2.0)
- **body**: Weight for document body (default: 1.0)
- **description**: Weight for document description (default: 1.5)
- **tags**: Weight for document tags (default: 0.5)

## Usage Examples

### Basic BM25 Usage
```rust
let query = Query::new("rust programming").name_scorer(Some(QueryScorer::BM25));
let documents = score::sort_documents(&query, documents);
```

### BM25F with Custom Field Weights
```rust
let weights = FieldWeights {
    title: 3.0,      // Emphasize title matches
    body: 1.0,       // Standard body weight
    description: 2.0, // Emphasize description
    tags: 0.5,       // Lower tag weight
};
```

### TFIDF for Simple Ranking
```rust
let query = Query::new("search term").name_scorer(Some(QueryScorer::TFIDF));
```

## Performance Considerations

- **BM25F** is typically slower than standard BM25 due to field processing
- **Jaccard** and **QueryRatio** are faster for simple term matching
- **TFIDF** is the fastest but may not capture document length effects
- **BM25Plus** provides the most control but requires parameter tuning

## Best Practices

1. **Start with BM25**: Use standard BM25 as your baseline
2. **Use BM25F for structured content**: When documents have clear field separation
3. **Consider Jaccard for exact matching**: When you care about term presence over frequency
4. **Tune parameters carefully**: BM25Plus requires experimentation with different parameter values
5. **Test with your data**: Different scorers work better for different types of content

## Integration with Terraphim

BM25 scorers integrate seamlessly with Terraphim's existing scoring infrastructure:

- Compatible with existing `QueryScorer` enum
- Works with current document indexing system
- Supports all existing document fields
- Maintains backward compatibility with similarity-based scoring 