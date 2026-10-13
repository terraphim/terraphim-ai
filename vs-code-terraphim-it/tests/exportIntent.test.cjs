const { test } = require('node:test');
const assert = require('node:assert/strict');
const { TerraphimExportIntent } = require('../media/export-intent.js');

test('one export retains its format until the exact native completion', () => {
  const intent = new TerraphimExportIntent();
  assert.equal(intent.begin('knowledge-graph'), true);
  assert.equal(intent.begin('markdown'), false);
  assert.deepEqual(intent.attach({ type:'proposal', id:1 }), { type:'proposal', id:1 });
  assert.deepEqual(intent.attach({ type:'export', id:2 }), { type:'export', id:2, format:'knowledge-graph' });
  intent.settle({ type:'command-complete', id:999 }, true);
  assert.equal(intent.begin('markdown'), false);
  intent.settle({ type:'reject', id:999 }, false);
  assert.equal(intent.begin('markdown'), false);
  intent.settle({ type:'command-complete', id:2 }, true);
  assert.equal(intent.begin('markdown'), true);
  assert.equal(intent.attach({ type:'export', id:3 }).format, 'markdown');
});

test('proposal refusal or external conflict clears a queued export format', () => {
  const intent = new TerraphimExportIntent();
  intent.begin('knowledge-graph');
  intent.settle({ type:'reject', id:1 }, true);
  assert.equal(intent.begin('markdown'), true);
  intent.clear();
  assert.equal(intent.begin('knowledge-graph'), true);
  assert.throws(() => intent.begin('unknown'), /Unsupported/);
});
