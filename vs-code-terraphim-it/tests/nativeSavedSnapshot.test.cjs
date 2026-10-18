'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs/promises');
const os = require('node:os');
const path = require('node:path');
const { matchesSavedSource } = require('./nativeSavedSnapshot.cjs');

test('actual saved-file transition rejects the old annotation-only snapshot before accepting the exact new source', async () => {
  const directory = await fs.mkdtemp(path.join(os.tmpdir(), 'terraphim-native-snapshot-'));
  try {
    const filename = path.join(directory, 'history.md');
    const old = 'Risk needs mitigation.\n\n```terraphim-alternatives\n{"version":1,"spans":[],"ghosts":[]}\n```\n';
    const expected = 'Risk needs mitigation.\n\n```terraphim-alternatives\n{"version":1,"spans":[],"ghosts":[{"id":"g1","anchor":{"start":0,"end":4,"text":"Risk"}}]}\n```\n';
    await fs.writeFile(filename, old, { flag: 'wx' });
    const before = await fs.readFile(filename, 'utf8');
    assert.equal(before.split('\n')[0], expected.split('\n')[0]);
    assert.equal(matchesSavedSource(before, expected), false);
    await fs.writeFile(filename, expected);
    assert.equal(matchesSavedSource(await fs.readFile(filename, 'utf8'), expected), true);
    await fs.writeFile(filename, old);
    assert.equal(matchesSavedSource(await fs.readFile(filename, 'utf8'), old), true);
  } finally { await fs.rm(directory, { recursive: true, force: true }); }
});

test('existing callers may omit a full snapshot but malformed snapshot arguments refuse', () => {
  assert.equal(matchesSavedSource('actual saved source', undefined), true);
  assert.equal(matchesSavedSource('', ''), true);
  assert.throws(() => matchesSavedSource('source', {}), TypeError);
  assert.throws(() => matchesSavedSource(null, 'source'), TypeError);
});
