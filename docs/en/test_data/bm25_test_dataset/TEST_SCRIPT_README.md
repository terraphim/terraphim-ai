# BM25 Test Script

This directory contains a test script (`test_bm25_scorers.py`) that helps evaluate and compare different BM25 scoring algorithms using the provided test datasets.

## Overview

The test script implements three BM25 variants:

1. **Standard BM25** - The classic BM25 algorithm
2. **BM25+** - An extension of BM25 with improved handling of rare terms
3. **BM25F** - A field-weighted version of BM25 that considers different document fields separately

## Usage

You can run the test script with the following command:

```bash
./test_bm25_scorers.py [--dataset DATASET] [--query QUERY_ID]
```

### Options

- `--dataset DATASET`: Specify which dataset file to use (default: documents.json)
- `--query QUERY_ID`: Specify a specific query ID to test (default: all queries)

### Examples

Test all queries using the main documents dataset:
```bash
./test_bm25_scorers.py
```

Test a specific query from the main dataset:
```bash
./test_bm25_scorers.py --query q1
```

Test using the field weighting test dataset:
```bash
./test_bm25_scorers.py --dataset field_weighting_test.json
```

Test a specific query from the rare terms test dataset:
```bash
./test_bm25_scorers.py --dataset rare_terms_test.json --query rtq1
```

## Test Datasets

The script works with all the test datasets in this directory:

- `documents.json` - Main collection of test documents
- `field_weighting_test.json` - Documents for testing field weighting in BM25F
- `rare_terms_test.json` - Documents for testing rare term handling in BM25+
- `document_length_test.json` - Documents of varying lengths for length normalization tests
- `term_frequency_test.json` - Documents for testing term frequency saturation

## Output

For each query, the script outputs:
1. The query ID and text
2. The query description
3. Top 5 ranked documents for each scoring algorithm (Standard BM25, BM25+, BM25F)
4. Expected results (if available in the dataset)

## Customization

You can modify the script to adjust parameters such as:
- `k1` - Controls term frequency saturation (default: 1.2)
- `b` - Controls document length normalization (default: 0.75)
- `delta` - Controls the lower bound for BM25+ (default: 1.0)
- Field weights for BM25F (defaults: title=3.0, body=1.0, description=1.5, tags=2.0)

## Using with Terraphim

This test script and datasets can be used to validate the implementation of BM25F and BM25+ in the Terraphim codebase. The Python implementation provides a reference to compare against the Rust implementation.

To compare results:
1. Run this test script on a dataset
2. Run the equivalent query through Terraphim's search functionality
3. Compare the ranking and scores to ensure consistency

## Notes

- The script uses a simple whitespace tokenizer for demonstration purposes. In a production environment, more sophisticated tokenization would be appropriate.
- Document frequencies are calculated across the entire test dataset, which may differ from a real-world corpus.
- The BM25F implementation in this script follows the approach described in Robertson et al. (2004), where term frequencies are weighted by field before saturation. 