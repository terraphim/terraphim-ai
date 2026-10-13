'use strict';
const fs = require('node:fs/promises');
const path = require('node:path');
const { createHash } = require('node:crypto');
const modules = new Map();

async function loadBindings(directory) {
  const [glue, wasm] = await Promise.all([
    fs.readFile(path.join(directory, 'terraphim_editor.js')),
    fs.readFile(path.join(directory, 'terraphim_editor_bg.wasm')),
  ]);
  const identity = createHash('sha256').update(glue).update(wasm).digest('hex');
  if (!modules.has(identity)) {
    // Native import remains intact in this CommonJS helper. The source is the
    // verified extension bundle, never document text or a workspace module.
    const loading = (async () => {
      const bindings = await import('data:text/javascript;base64,' + glue.toString('base64') + '#host-graph-' + identity);
      await bindings.default({ module_or_path: wasm });
      if (typeof bindings.export_knowledge_graph !== 'function') {
        throw new Error('The bundled editor does not support knowledge graph export. Update the extension.');
      }
      return bindings;
    })();
    modules.set(identity, loading);
    loading.catch(() => modules.delete(identity));
  }
  return modules.get(identity);
}

/** Export acknowledged source with the released Terraphim Rust implementation. */
async function exportKnowledgeGraph(source, assetDirectory) {
  if (typeof source !== 'string') throw new TypeError('Document source must be text');
  const bindings = await loadBindings(assetDirectory);
  // These two synchronous calls are one transaction on the host-only WASM
  // instance. No await can interleave another document or a webview's draft.
  bindings.open_document(source);
  return bindings.export_knowledge_graph() + '\n';
}

module.exports = { exportKnowledgeGraph };
