# BM25 Test Dataset

This directory contains test datasets and utilities for evaluating and comparing BM25F and BM25+ scoring algorithms in the Terraphim AI search system.

## Overview

The BM25 family of scoring functions are state-of-the-art ranking functions used in information retrieval. This test dataset is designed to evaluate and compare three variants:

1. **Standard BM25** - The classic BM25 algorithm
2. **BM25+** - An extension of BM25 with improved handling of rare terms
3. **BM25F** - A field-weighted version of BM25 that considers different document fields separately

## Directory Contents

- **[documents.json](documents.json)** - Main collection of test documents covering various programming topics
- **[queries.json](queries.json)** - Test queries and expected results for the main document collection
- **[field_weighting_test.json](field_weighting_test.json)** - Documents for testing field weighting in BM25F
- **[rare_terms_test.json](rare_terms_test.json)** - Documents for testing rare term handling in BM25+
- **[document_length_test.json](document_length_test.json)** - Documents of varying lengths for length normalization tests
- **[term_frequency_test.json](term_frequency_test.json)** - Documents for testing term frequency saturation
- **[test_bm25_scorers.py](test_bm25_scorers.py)** - Python utility for testing BM25 scoring algorithms
- **[TEST_SCRIPT_README.md](TEST_SCRIPT_README.md)** - Documentation for the test script
- **[TEST_RESULTS.md](TEST_RESULTS.md)** - Summary of test results and findings
- **[RUST_TEST_RESULTS.md](RUST_TEST_RESULTS.md)** - Summary of Rust implementation test results

## Test Datasets

Each test dataset is designed to evaluate specific aspects of the BM25 scoring algorithms:

1. **Main Documents Dataset** - Tests basic relevance ranking across different topics
2. **Field Weighting Test** - Tests BM25F's ability to weight different document fields differently
3. **Rare Terms Test** - Tests BM25+'s improved handling of rare terms
4. **Document Length Test** - Tests how the algorithms handle documents of varying lengths
5. **Term Frequency Test** - Tests how the algorithms handle term frequency saturation

## Using the Test Script

The `test_bm25_scorers.py` script provides a reference implementation of the BM25 scoring algorithms and allows you to run tests on the provided datasets. See [TEST_SCRIPT_README.md](TEST_SCRIPT_README.md) for detailed usage instructions.

Basic usage:

```bash
# Test all queries on the main documents dataset
./test_bm25_scorers.py

# Test a specific query on a specific dataset
./test_bm25_scorers.py --dataset field_weighting_test.json --query fwq1

# Enable debug output
./test_bm25_scorers.py --dataset rare_terms_test.json --query rtq1 --debug
```

## Test Results

The test results demonstrate the key characteristics and advantages of each algorithm. See [TEST_RESULTS.md](TEST_RESULTS.md) for a detailed analysis of the test results.

Key findings:

- **BM25F** excels at handling structured documents with distinct fields of different importance
- **BM25+** excels at handling rare terms and long-tail queries
- Both offer significant improvements over standard BM25 in specific scenarios

## Rust Implementation Test Results

The Rust implementation of BM25F and BM25+ has been tested against the Python reference implementation. See [RUST_TEST_RESULTS.md](RUST_TEST_RESULTS.md) for a detailed analysis of the Rust test results.

Key findings:

- The Rust implementation successfully matches the key characteristics of the Python reference implementation
- All tests pass, confirming that the Rust implementation can be used with confidence in the Terraphim AI search system
- There are some minor differences in the exact rankings due to differences in tokenization, field handling, and parameter values

## Using with Terraphim

This test dataset can be used to validate the implementation of BM25F and BM25+ in the Terraphim codebase. The Python implementation in the test script provides a reference to compare against the Rust implementation.

To compare results:
1. Run the test script on a dataset
2. Run the equivalent query through Terraphim's search functionality
3. Compare the ranking and scores to ensure consistency

## References

- Robertson, S. E., & Zaragoza, H. (2009). The Probabilistic Relevance Framework: BM25 and Beyond. Foundations and Trends in Information Retrieval, 3(4), 333-389.
- Lv, Y., & Zhai, C. (2011). Lower-bounding term frequency normalization. In Proceedings of the 20th ACM international conference on Information and knowledge management (pp. 7-16).
- Robertson, S., Zaragoza, H., & Taylor, M. (2004). Simple BM25 extension to multiple weighted fields. In Proceedings of the thirteenth ACM international conference on Information and knowledge management (pp. 42-49). 