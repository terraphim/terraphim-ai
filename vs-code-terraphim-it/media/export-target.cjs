'use strict';
const fs = require('node:fs/promises');
const { fileURLToPath } = require('node:url');

async function fileIdentity(uri) {
  if (!uri.startsWith('file:')) return null;
  try {
    const info = await fs.stat(fileURLToPath(uri));
    return `${info.dev}:${info.ino}`;
  } catch (error) {
    if (error.code === 'ENOENT') return null;
    throw error;
  }
}

/** Refuse URI aliases and actual filesystem aliases of protected documents. */
async function assertSeparateExportTarget(source, target, dirtyDocuments) {
  const protectedUris = [source, ...dirtyDocuments];
  const refusal = () => new Error('Choose a separate output file with no unsaved changes.');
  if (protectedUris.includes(target)) throw refusal();
  const targetIdentity = await fileIdentity(target);
  if (targetIdentity !== null) {
    const identities = await Promise.all(protectedUris.map(fileIdentity));
    if (identities.includes(targetIdentity)) throw refusal();
  }
}

module.exports = { assertSeparateExportTarget };
