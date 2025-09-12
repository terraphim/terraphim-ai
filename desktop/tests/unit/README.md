# Unit Tests

This directory contains unit tests for the desktop application components.

## Operator Parsing Tests

### `operator-parsing.test.mjs`
Comprehensive test suite for search operator parsing functionality. Tests include:

- **Basic operator detection**: AND/OR (capitalized and lowercase)
- **Mixed operator handling**: Queries with both AND and OR operators
- **UI operator override**: How UI operator controls interact with text operators
- **Term extraction**: Proper parsing of search terms from complex queries
- **Search query building**: Conversion of parsed input to search API format

### `operator-parsing.test.mjs`
Comprehensive test suite for search operator parsing functionality (ES modules format).

## Running the Tests

```bash
# Run the operator parsing tests
node tests/unit/operator-parsing.test.mjs
```

## Test Coverage

These tests ensure that:
1. AND and OR operators are correctly detected and parsed
2. Mixed operator queries are handled properly
3. UI operator controls properly override text operators
4. Search behavior changes correctly based on operator selection
5. Terms are extracted cleanly without operator artifacts

## Integration with Main Test Suite

These unit tests complement the existing Playwright e2e tests by providing focused testing of the search parsing logic in isolation, making it easier to debug operator-related issues and ensure consistent behavior across different input scenarios.
