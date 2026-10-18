'use strict';
const {test} = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs/promises');
const os = require('node:os');
const path = require('node:path');
const {createHash} = require('node:crypto');
const {readOwnedRecoveryArchive} = require('./nativeRecoveryArchive.cjs');
const {encodeRecovery} = require('../out/authorityRecovery.js');
const installed = path.resolve(__dirname, '..');
const name = '12345678-1234-1234-1234-123456789abc.json';
async function fixture(run) {
  const profile = await fs.mkdtemp(path.join(os.tmpdir(), 'native-archive-readback-'));
  const directory = path.join(profile, 'User', 'globalStorage', 'applied-knowledge-systems.terraphim-it', 'native-authority-recovery');
  await fs.mkdir(directory, {recursive:true});
  try {await run({profile,directory,file:path.join(directory,name)});} finally {await fs.rm(profile,{recursive:true,force:true});}
}
test('actual versioned archive preserves accepted source and existing opaque drafts without changing bytes', async()=>fixture(async({profile,file})=>{
  const source='Concern changed externally.\r\n';
  const drafts=[{viewSession:'actual-parser-input',sequence:7,draft:JSON.stringify({input:'unaccepted Ω'})}];
  const raw=encodeRecovery(source,drafts);await fs.writeFile(file,raw);
  const result=await readOwnedRecoveryArchive(profile,name,installed);
  assert.equal(result.source,source);assert.deepEqual(result.drafts,drafts);assert.equal(result.bytes.toString(),raw);
  assert.equal(result.sha256,createHash('sha256').update(raw).digest('hex'));assert.equal(await fs.readFile(file,'utf8'),raw);
}));
test('actual accepted-only archive does not invent pending drafts',async()=>fixture(async({profile,file})=>{
  await fs.writeFile(file,encodeRecovery('Concern changed externally.\n',[]));
  assert.deepEqual((await readOwnedRecoveryArchive(profile,name,installed)).drafts,[]);
}));
test('active archive path cannot escape the disposable profile or follow a file symlink',async()=>fixture(async({profile,directory,file})=>{
  await fs.writeFile(file,encodeRecovery('Owned source.\n',[]));
  for(const invalid of ['../'+name,'/tmp/'+name,'other.md'])await assert.rejects(readOwnedRecoveryArchive(profile,invalid,installed));
  const outside=path.join(profile,'outside.json');await fs.writeFile(outside,encodeRecovery('Foreign source.\n',[]));await fs.unlink(file);await fs.symlink(outside,file);
  await assert.rejects(readOwnedRecoveryArchive(profile,name,installed),/regular owned file/);
  await fs.unlink(file);await fs.rmdir(directory);await fs.symlink(path.dirname(outside),directory);
  await assert.rejects(readOwnedRecoveryArchive(profile,name,installed),/inside the owned profile/);
}));
test('real packaged decoder rejects malformed version, foreign kind, invalid draft identity and invalid UTF-8',async()=>fixture(async({profile,file})=>{
  const valid=JSON.parse(encodeRecovery('Concern changed externally.\n',[]));
  for(const input of [{...valid,version:2},{...valid,kind:'other'}, {...valid,drafts:[{viewSession:'v',sequence:0,draft:'raw'}]}]){
    await fs.writeFile(file,JSON.stringify(input));await assert.rejects(readOwnedRecoveryArchive(profile,name,installed));
  }
  await fs.writeFile(file,Buffer.concat([Buffer.from('{"source":"'),Buffer.from([255]),Buffer.from('"}')]));
  await assert.rejects(readOwnedRecoveryArchive(profile,name,installed),/UTF-8/);
}));
