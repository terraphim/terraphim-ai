const {test} = require('node:test');
const assert = require('node:assert/strict');
const {CompletionCache} = require('../out/helpers/completionCache.js');

test('late old-role response cannot replace results from the new role', () => {
  const cache = new CompletionCache();
  const old = cache.begin('Risk', 1).request;
  const current = cache.begin('Risk', 2).request;
  assert.deepEqual(cache.commit(current, 2, ['new role']), ['new role']);
  assert.equal(cache.isCurrent(old, 2), false);
  assert.deepEqual(cache.commit(old, 2, ['old role']), []);
  assert.deepEqual(cache.begin('Risk', 2).cached, ['new role']);
});

test('query A then B then A rejects the original A response', () => {
  const cache = new CompletionCache();
  const firstA = cache.begin('Risk', 1).request;
  const b = cache.begin('Decision', 1).request;
  const latestA = cache.begin('Risk', 1).request;
  assert.deepEqual(cache.commit(latestA, 1, ['latest risk']), ['latest risk']);
  assert.deepEqual(cache.commit(b, 1, ['decision']), []);
  assert.deepEqual(cache.commit(firstA, 1, ['stale risk']), []);
  assert.deepEqual(cache.begin('Risk', 1).cached, ['latest risk']);
});

test('configuration changes invalidate an in-flight request before another query starts', () => {
  const cache = new CompletionCache();
  const request = cache.begin('Risk', 1).request;
  assert.equal(cache.isCurrent(request, 2), false);
  assert.deepEqual(cache.commit(request, 2, ['old endpoint']), []);
  assert.equal(cache.begin('Risk', 2).cached, undefined);
});

test('changing query clears previous results while the new request is pending', () => {
  const cache = new CompletionCache();
  cache.commit(cache.begin('Risk', 1).request, 1, ['risk']);
  const next = cache.begin('Decision', 1);
  assert.equal(next.cached, undefined);
  assert.deepEqual(cache.begin('Decision', 1).cached, []);
  assert.deepEqual(cache.commit(next.request, 1, ['decision']), ['decision']);
});
