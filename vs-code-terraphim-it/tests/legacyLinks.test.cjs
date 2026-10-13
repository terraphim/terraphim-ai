/* Actual packaged Rust/WASM link replacement. No native API or server mocks. */
const test = require('node:test');
const assert = require('node:assert/strict');
const path = require('node:path');
const root = process.env.TERRAPHIM_EXTENSION_ROOT || path.resolve(__dirname, '..');
const { replace_links } = require(path.join(root, 'rust-lib/pkg/terraphim_it.js'));
const thesaurus = JSON.stringify({name: 'Project brief', data: {
  risk: {id: 1, nterm: 'risk', url: 'https://example.com/decisions/risk'},
  concern: {id: 1, nterm: 'risk', url: 'https://example.com/decisions/risk'},
}});

test('packaged Terraphim automata links canonical terms and synonyms', () => {
  assert.equal(replace_links('A concern and risk. 😀', thesaurus),
    'A [risk](https://example.com/decisions/risk) and [risk](https://example.com/decisions/risk). 😀');
});

test('unmatched text and line endings survive the actual WASM call', () => {
  const source = 'Unmatched Café 😀.\r\nRetain this line.\r\n';
  assert.equal(replace_links(source, thesaurus), source);
});

test('malformed graph throws instead of erasing the document and next call works', () => {
  assert.throws(() => replace_links('Preserve this document', '{invalid}'));
  assert.equal(replace_links('concern', thesaurus), '[risk](https://example.com/decisions/risk)');
});

const {replaceLinks} = require(path.join(root, 'out/helpers/linkReplacement.js'));
test('the production command loader resolves the exact packaged module', async () => {
  assert.equal(await replaceLinks('A concern.', thesaurus), 'A [risk](https://example.com/decisions/risk).');
  await assert.rejects(replaceLinks('Preserve me', '{invalid}'));
});
