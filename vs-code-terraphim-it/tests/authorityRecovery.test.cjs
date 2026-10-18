'use strict';
const {test}=require('node:test');
const assert=require('node:assert/strict');
const {mkdtemp,writeFile,readFile,rm}=require('node:fs/promises');
const os=require('node:os');const path=require('node:path');
const {encodeRecovery,decodeRecovery}=require('../out/authorityRecovery');
test('real recovery file preserves annotated accepted source and distinguishable pending drafts',async()=>{
 const dir=await mkdtemp(path.join(os.tmpdir(),'terraphim-authority-recovery-'));
 try {const source='Risk matters.\n<!-- terraphim:annotations {"spans":[]} -->\n';
  const drafts=[{viewSession:'view-a',sequence:3,draft:JSON.stringify({alternative:'Concern',body:'Risk matters.\n'})}];
  await writeFile(path.join(dir,'backup'),encodeRecovery(source,drafts));
  assert.deepEqual(decodeRecovery(await readFile(path.join(dir,'backup'),'utf8')),{source,drafts});
 }finally{await rm(dir,{recursive:true,force:true});}
});
test('legacy Markdown backups remain Markdown and malformed tagged bundles refuse',()=>{
 for(const source of ['Risk\n','{"document":"ordinary markdown"}\n',''])assert.deepEqual(decodeRecovery(source),{source,drafts:[]});
 for(const value of [{kind:'terraphim-native-authority-backup',version:2,source:'x',drafts:[]},
  {kind:'terraphim-native-authority-backup',version:1,source:'x',drafts:[{viewSession:'a',sequence:-1,draft:'x'}]},
  {kind:'terraphim-native-authority-backup',version:1,source:'x',drafts:[],path:'/tmp/foreign'}])assert.throws(()=>decodeRecovery(JSON.stringify(value)));
});
test('recovery rejects duplicate sessions and oversized drafts before acknowledging durability',()=>{
 const draft={viewSession:'a',sequence:1,draft:'pending'};
 assert.throws(()=>encodeRecovery('Risk',[draft,draft]));
 assert.throws(()=>encodeRecovery('Risk',[{...draft,draft:'x'.repeat(128*1024*1024+1)}]));
});
test('representative multi-MiB source and pending intent survive real backup without source-budget narrowing',async()=>{
 const dir=await mkdtemp(path.join(os.tmpdir(),'terraphim-authority-large-recovery-'));
 try{const source='a'.repeat(8*1024*1024),draft=JSON.stringify({baseSourceHash:'0'.repeat(64),pending:{sequence:1,intents:[{key:'body',sequence:1,input:{text:'b'.repeat(8*1024*1024)}}]}});
  const expected={source,drafts:[{viewSession:'large',sequence:1,draft}]};const encoded=encodeRecovery(expected.source,expected.drafts);
  await writeFile(path.join(dir,'backup'),encoded);assert.deepEqual(decodeRecovery(await readFile(path.join(dir,'backup'),'utf8')),expected);
 }finally{await rm(dir,{recursive:true,force:true});}
});

test('save completion refuses a new raw draft arriving during real save I/O before delayed readback',async()=>{
 const {assertSaveSnapshotCurrent}=require('../out/saveSnapshot');
 const dir=await mkdtemp(path.join(os.tmpdir(),'terraphim-save-race-'));
 try {
  const source='Risk matters.\n',doc={source,revision:7,pendingAuthority:undefined,drafts:new Map()};
  let release,wrote;const gate=new Promise(resolve=>{release=resolve;});
  const written=new Promise(resolve=>{wrote=resolve;});
  const file=path.join(dir,'brief.md');
  const completion=(async()=>{
   await writeFile(file,source);wrote();await gate;assert.equal(await readFile(file,'utf8'),source);
   assertSaveSnapshotCurrent(doc,source,7);
  })();
  const draft={viewSession:'second-view',sequence:8,draft:JSON.stringify({blocks:{text:'Concern remains unsaved'}})};
  await written;doc.drafts.set(draft.viewSession,draft);release();
  await assert.rejects(completion,/Newer draft remains unsaved/);
  assert.equal(await readFile(file,'utf8'),source);
  assert.deepEqual([...doc.drafts.values()],[draft]);
  await writeFile(path.join(dir,'recovery'),encodeRecovery(source,[...doc.drafts.values()]));
  assert.deepEqual(decodeRecovery(await readFile(path.join(dir,'recovery'),'utf8')),{source,drafts:[draft]});
  doc.drafts.clear();doc.pendingAuthority={dispatchId:8};
  assert.throws(()=>assertSaveSnapshotCurrent(doc,source,7),/Newer draft remains unsaved/);
  doc.pendingAuthority=undefined;assertSaveSnapshotCurrent(doc,source,7);
  doc.source='New accepted source';doc.revision++;
  assert.throws(()=>assertSaveSnapshotCurrent(doc,source,7),/Newer source remains unsaved/);
 }finally{await rm(dir,{recursive:true,force:true});}
});
