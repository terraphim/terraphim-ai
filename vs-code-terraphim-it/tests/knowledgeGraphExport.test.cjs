const { test } = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs/promises');
const path = require('node:path');
const os = require('node:os');
const { exportKnowledgeGraph } = require('../media/knowledge-graph.cjs');
const { TerraphimExportIntent } = require('../media/export-intent.js');
const { TerraphimHostBridge } = require('../media/editor/host-session.js');
const assets = path.resolve(__dirname, '../media/editor');

async function loadTestBindings() {
  const glue = await fs.readFile(path.join(assets, 'terraphim_editor.js'));
  const bindings = await import('data:text/javascript;base64,' + glue.toString('base64') + '#graph-input-test');
  await bindings.default({ module_or_path: await fs.readFile(path.join(assets, 'terraphim_editor_bg.wasm')) });
  return bindings;
}

test('graph format follows the real WASM ACK barrier while newer input remains queued', async () => {
  const bindings = await loadTestBindings();
  const source = await fs.readFile(path.join(__dirname, 'fixtures/native-three-alternatives.md'), 'utf8');
  const intent = new TerraphimExportIntent();
  const sent = [];
  const bridge = new TerraphimHostBridge({
    engine:new bindings.TerraphimHostSession('file:///brief.md', 'export-barrier', 1, 'Original'),
    send:message => sent.push(intent.attach(message)), adopt:() => {}, conflict:() => intent.clear(),
  });
  try {
    intent.begin('knowledge-graph');
    bridge.submit(source); bridge.command('export');
    assert.equal(sent.length, 1); assert.equal(sent[0].type, 'proposal');
    bridge.receive({ ...sent[0], type:'ack', revision:2 });
    const request = sent[1];
    assert.equal(request.type, 'export'); assert.equal(request.format, 'knowledge-graph');
    const acknowledged = bridge.current.source;
    bridge.submit('New unacknowledged document');
    assert.equal(sent.length, 2);
    assert.deepEqual(Object.keys(JSON.parse(await exportKnowledgeGraph(acknowledged, assets)).data).sort(),
      ['eraser', 'paperclip', 'thumbtack']);
    const completion = { ...request, type:'command-complete', source:acknowledged };
    intent.settle(completion, bridge.receive(completion));
    assert.equal(sent[2].type, 'proposal');
    assert.equal(intent.pending, null);
  } finally { bridge.dispose(); }
});

test('newly created risk/concern synonym exports only committed snapshots and follows undo/redo', async () => {
  const bindings = await loadTestBindings();
  const source = 'Risk: supplier delay. Decision: retain a backup supplier.';
  const engine = new bindings.TerraphimHostSession('file:///brief.md', 'new-synonym', 1, source);
  try {
    const before = JSON.parse(engine.snapshot());
    const proposal = JSON.parse(engine.propose(JSON.stringify({ ...before, id:1, command:'alt_create',
      args:{ kind:'word', start:0, end:4, text:'Concern' } })));
    assert.deepEqual(JSON.parse(await exportKnowledgeGraph(before.source, assets)).data, {});
    engine.acknowledge(1, 2, proposal.source);
    const committed = JSON.parse(engine.snapshot());
    assert.equal(committed.projection.annotations.synonymGroups[0].markdown, 'synonyms:: Risk, Concern\n');
    const expected = JSON.parse(await exportKnowledgeGraph(committed.source, assets));
    assert.deepEqual(Object.keys(expected.data).sort(), ['concern', 'risk']);
    assert.equal(expected.data.risk.id, expected.data.concern.id);
    assert.equal(expected.data.risk.nterm, expected.data.concern.nterm);
    engine.synchronize(3, source);
    assert.deepEqual(JSON.parse(await exportKnowledgeGraph(JSON.parse(engine.snapshot()).source, assets)).data, {});
    engine.synchronize(4, committed.source);
    assert.deepEqual(JSON.parse(await exportKnowledgeGraph(JSON.parse(engine.snapshot()).source, assets)), expected);
    const snapshot = JSON.parse(engine.snapshot());
    const start = source.indexOf('Decision');
    const ambiguous = JSON.parse(engine.propose(JSON.stringify({ ...snapshot, id:2, command:'alt_create',
      args:{ kind:'word', start, end:start + 'Decision'.length, text:'Concern' } })));
    engine.acknowledge(2, 5, ambiguous.source);
    await assert.rejects(exportKnowledgeGraph(JSON.parse(engine.snapshot()).source, assets));
  } finally { engine.free(); }
});

test('actual saved native alternatives export one typed concept through removal, undo, redo and reopen', async () => {
  const original = await fs.readFile(path.join(__dirname, 'fixtures/native-three-alternatives.md'), 'utf8');
  const removed = await fs.readFile(path.join(__dirname, 'fixtures/native-removed-alternative.md'), 'utf8');
  const directory = await fs.mkdtemp(path.join(os.tmpdir(), 'terraphim-graph-export-'));
  try {
    for (const [step, source, terms] of [
      ['create', original, ['eraser', 'paperclip', 'thumbtack']],
      ['remove', removed, ['eraser', 'paperclip']],
      ['undo', original, ['eraser', 'paperclip', 'thumbtack']],
      ['redo', removed, ['eraser', 'paperclip']],
    ]) {
      const document = path.join(directory, step + '.md');
      await fs.writeFile(document, source);
      const reopened = await fs.readFile(document, 'utf8');
      const graphPath = path.join(directory, step + '.knowledge-graph.json');
      await fs.writeFile(graphPath, await exportKnowledgeGraph(reopened, assets));
      const graph = JSON.parse(await fs.readFile(graphPath, 'utf8'));
      assert.deepEqual(Object.keys(graph.data).sort(), terms);
      assert.equal(new Set(Object.values(graph.data).map(value => value.id)).size, 1);
      assert.equal(new Set(Object.values(graph.data).map(value => value.nterm)).size, 1);
      assert.equal(await fs.readFile(document, 'utf8'), source);
    }
  } finally { await fs.rm(directory, { recursive: true }); }
});

test('simultaneous exports do not leak another document graph into an empty document', async () => {
  const source = await fs.readFile(path.join(__dirname, 'fixtures/native-three-alternatives.md'), 'utf8');
  const results = await Promise.all(Array.from({ length: 8 }, (_, i) =>
    exportKnowledgeGraph(i % 2 ? 'Plain document with no alternatives.' : source, assets)));
  for (const [index, result] of results.entries()) {
    assert.equal(Object.keys(JSON.parse(result).data).length, index % 2 ? 0 : 3);
  }
});

test('malformed annotations fail closed through the real Rust exporter', async () => {
  await assert.rejects(exportKnowledgeGraph('Risk\n\n```terraphim-alternatives\n{bad json}\n```\n', assets));
  await assert.rejects(exportKnowledgeGraph(null, assets), /must be text/);
});
