#!/usr/bin/env node
// Loads @terraphim/autocomplete as a consumer would, from the current directory's
// node_modules, and runs one real search through the native binding.

const assert = require('node:assert/strict')

const entry = require.resolve('@terraphim/autocomplete', { paths: [process.cwd()] })
const { autocomplete, buildAutocompleteIndexFromJson, version } = require(entry)

const thesaurus = {
  name: 'Smoke test',
  data: {
    'machine learning': { id: 1, nterm: 'machine learning', url: 'https://example.com/ml' },
    'deep learning': { id: 2, nterm: 'deep learning', url: 'https://example.com/dl' },
  },
}

const index = buildAutocompleteIndexFromJson(JSON.stringify(thesaurus))
const results = autocomplete(Buffer.from(index), 'machine', 10)

assert.deepEqual(
  results.map((result) => result.term),
  ['machine learning'],
)

console.log(`Loaded ${entry}`)
console.log(`Native version ${version()}; found ${results[0].term}`)
