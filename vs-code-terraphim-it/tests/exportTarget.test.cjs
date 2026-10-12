const { test } = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs/promises');
const os = require('node:os');
const path = require('node:path');
const { pathToFileURL } = require('node:url');
const { assertSeparateExportTarget } = require('../media/export-target.cjs');

test('real source and dirty-document aliases are refused while separate output is allowed', async () => {
  const root = await fs.mkdtemp(path.join(os.tmpdir(), 'terraphim-export-target-'));
  const uri = name => pathToFileURL(path.join(root, name)).href;
  try {
    await fs.writeFile(path.join(root, 'source.md'), 'Original source');
    await fs.writeFile(path.join(root, 'dirty.json'), 'Saved dirty document version');
    await fs.symlink(path.join(root, 'source.md'), path.join(root, 'source-alias.json'));
    await fs.link(path.join(root, 'source.md'), path.join(root, 'source-hardlink.json'));
    await fs.symlink(path.join(root, 'dirty.json'), path.join(root, 'dirty-alias.json'));
    for (const target of ['source.md', 'source-alias.json', 'source-hardlink.json', 'dirty.json', 'dirty-alias.json']) {
      await assert.rejects(assertSeparateExportTarget(uri('source.md'), uri(target), [uri('dirty.json')]), /separate output/);
    }
    await assertSeparateExportTarget(uri('source.md'), uri('new.json'), [uri('dirty.json')]);
    await fs.writeFile(path.join(root, 'existing.json'), '{}');
    await assertSeparateExportTarget(uri('source.md'), uri('existing.json'), []);
    assert.equal(await fs.readFile(path.join(root, 'source.md'), 'utf8'), 'Original source');
    assert.equal(await fs.readFile(path.join(root, 'dirty.json'), 'utf8'), 'Saved dirty document version');
  } finally { await fs.rm(root, { recursive: true }); }
});

test('non-file URI source and dirty destinations are still protected exactly', async () => {
  await assert.rejects(assertSeparateExportTarget('memfs:/source.md', 'memfs:/source.md', []));
  await assert.rejects(assertSeparateExportTarget('memfs:/source.md', 'memfs:/dirty.json', ['memfs:/dirty.json']));
  await assertSeparateExportTarget('memfs:/source.md', 'memfs:/graph.json', []);
});
