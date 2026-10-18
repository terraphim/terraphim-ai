'use strict';
// Observe only the actual active archive in this run's disposable native profile.
const fs = require('node:fs/promises');
const path = require('node:path');
const assert = require('node:assert/strict');
const {createHash} = require('node:crypto');

async function readOwnedRecoveryArchive(profile, tabName, installedRoot) {
  assert.ok(path.isAbsolute(profile) && path.isAbsolute(installedRoot), 'Use explicit owned profile and installed package paths');
  assert.match(tabName, /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}\.json$/, 'Observe the exact native archive tab name');
  const owned = await fs.realpath(profile);
  const directory = path.join(owned, 'User', 'globalStorage', 'applied-knowledge-systems.terraphim-it', 'native-authority-recovery');
  assert.equal(await fs.realpath(directory), directory, 'Recovery storage must stay inside the owned profile');
  const archivePath = path.join(directory, tabName);
  const stat = await fs.lstat(archivePath);
  assert.ok(stat.isFile() && !stat.isSymbolicLink(), 'Recovery archive must be a regular owned file');
  assert.ok(stat.size <= 512 * 1024 * 1024, 'Respect the production recovery envelope limit');
  const bytes = await fs.readFile(archivePath);
  const raw = bytes.toString('utf8');
  assert.ok(Buffer.from(raw, 'utf8').equals(bytes), 'Archive UTF-8 bytes must decode exactly');
  const record = JSON.parse(raw);
  assert.equal(record.kind, 'terraphim-native-authority-backup');
  assert.equal(record.version, 1);
  // Validate with the real decoder shipped in the package being exercised.
  const {decodeRecovery} = require(path.join(installedRoot, 'out', 'authorityRecovery.js'));
  const decoded = decodeRecovery(raw);
  return {archivePath, bytes, source:decoded.source, drafts:decoded.drafts,
    sha256:createHash('sha256').update(bytes).digest('hex')};
}
module.exports = {readOwnedRecoveryArchive};
