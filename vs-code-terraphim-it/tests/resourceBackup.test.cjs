'use strict';
const {test}=require('node:test'),assert=require('node:assert/strict');
const fs=require('node:fs/promises'),path=require('node:path'),os=require('node:os'),{randomUUID,createHash}=require('node:crypto');
const {encodeResourceBackup,decodeResourceBackup,resourceDirectoryKey,resourceBackupFileName,canRetireResourceBackup,resourceBackupNames,admitResourceBackupBytes,resourceBackupBaselineMatches}=require('../out/resourceBackup');
const hash=text=>createHash('sha256').update(text).digest('hex');
const make=(extra={})=>({kind:'terraphim-resource-backup',version:1,resource:'file:///project/brief.md',authorityEpoch:randomUUID(),token:randomUUID(),baselineHash:hash('Risk\n'),source:'Concern\n',drafts:[{viewSession:'v',sequence:2,draft:'pending raw alternative'}],...extra});
test('actual private backup retains exact accepted source and separate raw drafts with strict resource identity',async()=>{
 const dir=await fs.mkdtemp(path.join(os.tmpdir(),'terraphim-resource-backup-'));
 try{const data=make(),folder=path.join(dir,resourceDirectoryKey(data.resource));await fs.mkdir(folder);const file=path.join(folder,resourceBackupFileName(data.authorityEpoch,data.token));await fs.writeFile(file,encodeResourceBackup(data));assert.deepEqual(decodeResourceBackup(await fs.readFile(file,'utf8'),data.resource,data.authorityEpoch),data);
 const raw=await fs.readFile(file,'utf8');assert.throws(()=>decodeResourceBackup(raw,'file:///other.md',data.authorityEpoch));
 }finally{await fs.rm(dir,{recursive:true,force:true});}
});
test('strict backup envelope refuses foreign epochs, arbitrary paths, malformed identifiers and source bounds',()=>{
 const good=make();for(const patch of [{path:'/tmp/foreign'},{baselineHash:'x'},{token:'../token'},{authorityEpoch:'../escape'},{resource:''},{source:'x'.repeat(16*1024*1024+1)}])assert.throws(()=>encodeResourceBackup({...good,...patch}));
 assert.throws(()=>decodeResourceBackup(encodeResourceBackup(good),good.resource,randomUUID()));
 assert.throws(()=>resourceBackupFileName('../escape',good.token));
});
test('two real authority backups for the same URI remain separate; stale cleanup cannot delete another window or newer snapshot',async()=>{
 const dir=await fs.mkdtemp(path.join(os.tmpdir(),'terraphim-resource-backup-'));
 try{const a=make(),b=make({source:'Another window source'}),af=path.join(dir,resourceBackupFileName(a.authorityEpoch,a.token)),bf=path.join(dir,resourceBackupFileName(b.authorityEpoch,b.token));
 await fs.writeFile(af,encodeResourceBackup(a));await fs.writeFile(bf,encodeResourceBackup(b));const newer={...a,token:randomUUID(),source:'Newer accepted source'};await fs.writeFile(af,encodeResourceBackup(newer));
 assert.equal(canRetireResourceBackup(decodeResourceBackup(await fs.readFile(af,'utf8'),a.resource),a.resource,a.authorityEpoch,a.token),false);
 assert.equal(canRetireResourceBackup(decodeResourceBackup(await fs.readFile(bf,'utf8'),b.resource),a.resource,a.authorityEpoch,a.token),false);
 assert.equal(await fs.readFile(bf,'utf8'),encodeResourceBackup(b));assert.notEqual(resourceDirectoryKey(a.resource),resourceDirectoryKey('file:///other.md'));
 }finally{await fs.rm(dir,{recursive:true,force:true});}
});
test('foreign recovered snapshot retires only when its accepted bytes are durably saved and it contains no pending drafts',()=>{
 const backup=make();assert.equal(canRetireResourceBackup(backup,backup.resource,backup.authorityEpoch,backup.token,backup.source),false);
 assert.equal(canRetireResourceBackup({...backup,drafts:[]},backup.resource,backup.authorityEpoch,backup.token,'changed'),false);
 assert.equal(canRetireResourceBackup({...backup,drafts:[]},backup.resource,backup.authorityEpoch,backup.token,backup.source),true);
});

test('real incomplete write stays unadmitted, completed snapshots survive it, and aggregate admission uses actual stat sizes',async()=>{
 const dir=await fs.mkdtemp(path.join(os.tmpdir(),'terraphim-resource-backup-'));
 try{const a=make(),b=make(),af=resourceBackupFileName(a.authorityEpoch,a.token),bf=resourceBackupFileName(b.authorityEpoch,b.token);
 await fs.writeFile(path.join(dir,af),encodeResourceBackup(a));await fs.writeFile(path.join(dir,bf+'.partial'),'{incomplete');
 assert.deepEqual(resourceBackupNames(await fs.readdir(dir)),[af]);assert.deepEqual(decodeResourceBackup(await fs.readFile(path.join(dir,af),'utf8'),a.resource),a);
 await fs.writeFile(path.join(dir,bf+'.partial'),encodeResourceBackup(b));await fs.rename(path.join(dir,bf+'.partial'),path.join(dir,bf));
 const sizes=await Promise.all((await fs.readdir(dir)).map(async name=>(await fs.stat(path.join(dir,name))).size));const bytes=sizes.reduce((a,b)=>a+b,0);
 assert.throws(()=>admitResourceBackupBytes(sizes,4*(bytes-1)),/capacity/);admitResourceBackupBytes(sizes,4*bytes);
 assert.throws(()=>resourceBackupNames(Array.from({length:33},()=>resourceBackupFileName(randomUUID(),randomUUID()))),/capacity/);
 }finally{await fs.rm(dir,{recursive:true,force:true});}
});
test('real external file write changes baseline eligibility without changing or deleting the recovery snapshot',async()=>{
 const dir=await fs.mkdtemp(path.join(os.tmpdir(),'terraphim-resource-backup-'));
 try{const data=make(),file=path.join(dir,'brief.md'),snapshot=path.join(dir,'backup');await fs.writeFile(file,'Risk\n');await fs.writeFile(snapshot,encodeResourceBackup(data));
 assert.equal(resourceBackupBaselineMatches(data,await fs.readFile(file,'utf8')),true);await fs.writeFile(file,'External decision\n');
 assert.equal(resourceBackupBaselineMatches(data,await fs.readFile(file,'utf8')),false);assert.equal(await fs.readFile(snapshot,'utf8'),encodeResourceBackup(data));
 }finally{await fs.rm(dir,{recursive:true,force:true});}
});
