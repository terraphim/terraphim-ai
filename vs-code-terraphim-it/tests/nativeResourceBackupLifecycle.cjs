/* Official extensionTestsPath integration: real provider, VSCode APIs and disk-backed gated filesystem. */
'use strict';
const vscode=require('vscode'),assert=require('node:assert/strict'),fs=require('node:fs/promises'),nativeFs=require('node:fs');
const path=require('node:path'),{randomUUID,createHash}=require('node:crypto');
const hash=text=>createHash('sha256').update(text).digest('hex');
exports.run=async()=>{
 const output=process.env.TERRAPHIM_BACKUP_OUTPUT,input=process.env.TERRAPHIM_BACKUP_INPUT;
 assert.ok(output?.startsWith('/tmp/'));await fs.mkdir(output,{recursive:true});
 const extension=vscode.extensions.getExtension('applied-knowledge-systems.terraphim-it');assert.ok(extension);
 const {AlternativeControlProvider}=require(path.join(extension.extensionPath,'out/alternativeControl'));
 const {encodeResourceBackup,decodeResourceBackup,resourceDirectoryKey}=require(path.join(extension.extensionPath,'out/resourceBackup'));
 const source=await fs.readFile(input,'utf8'),initial='Risk needs mitigation.\n\nThe editor is basically the only target here today.\n';
 const root=path.join(output,'real-fs'),storage=vscode.Uri.parse('terraphim-backup-verification:/storage'),events=new vscode.EventEmitter();await fs.mkdir(root,{recursive:true});
 const providers=[],documents=[];let gate;
 const resolve=uri=>{const result=path.resolve(root,'.'+uri.path);assert.ok(result===root||result.startsWith(root+path.sep));return result;};
 const convert=(error,uri)=>{if(error.code==='ENOENT')throw vscode.FileSystemError.FileNotFound(uri);if(error.code==='EEXIST')throw vscode.FileSystemError.FileExists(uri);throw error;};
 const adapter={onDidChangeFile:events.event,
  watch(uri){const target=resolve(uri);let watcher;try{watcher=nativeFs.watch(target,()=>events.fire([{type:vscode.FileChangeType.Changed,uri}]));}catch{watcher=nativeFs.watch(root,()=>events.fire([{type:vscode.FileChangeType.Changed,uri}]));}return new vscode.Disposable(()=>watcher.close());},
  async stat(uri){try{const s=await fs.stat(resolve(uri));return {type:s.isDirectory()?vscode.FileType.Directory:vscode.FileType.File,ctime:s.ctimeMs,mtime:s.mtimeMs,size:s.size};}catch(e){convert(e,uri);}},
  async readDirectory(uri){try{return(await fs.readdir(resolve(uri),{withFileTypes:true})).map(e=>[e.name,e.isDirectory()?vscode.FileType.Directory:e.isSymbolicLink()?vscode.FileType.SymbolicLink:vscode.FileType.File]);}catch(e){convert(e,uri);}},
  async readFile(uri){try{return new Uint8Array(await fs.readFile(resolve(uri)));}catch(e){convert(e,uri);}},
  async createDirectory(uri){await fs.mkdir(resolve(uri),{recursive:true});},
  async writeFile(uri,data,options){try{if(!options.create)await fs.access(resolve(uri));await fs.writeFile(resolve(uri),data,{flag:options.overwrite?'w':'wx'});}catch(e){convert(e,uri);}},
  async delete(uri,options){try{await fs.rm(resolve(uri),{recursive:options.recursive,force:false});}catch(e){convert(e,uri);}},
  async rename(from,to,options){try{if(!options.overwrite){try{await fs.access(resolve(to));throw vscode.FileSystemError.FileExists(to);}catch(e){if(e.code!=='ENOENT')throw e;}}await fs.rename(resolve(from),resolve(to));
   if(gate){const held=gate;gate=undefined;held.reached();await held.wait;}
  }catch(e){convert(e,to);}},
 };
 const registration=vscode.workspace.registerFileSystemProvider('terraphim-backup-verification',adapter,{isCaseSensitive:true});
 const receipt={status:'RUNNING',scope:'Real official VSCode extension-host provider lifecycle with actual disk IO; not a UI authoring/marketplace-install claim',checks:[],extensionPath:extension.extensionPath};
 const neverCancelled=new vscode.CancellationTokenSource(),none=neverCancelled.token;
 function holdRename(){let reached,release;const signal=new Promise(r=>reached=r),wait=new Promise(r=>release=r);gate={reached,wait};return {signal,release};}
 async function open(name,drafts=[]){const file=path.join(output,name+'.md');await fs.writeFile(file,initial);const uri=vscode.Uri.file(file),record={kind:'terraphim-resource-backup',version:1,resource:uri.toString(),authorityEpoch:randomUUID(),token:randomUUID(),baselineHash:hash(initial),source,drafts};
  const fixture=path.join(output,name+'-INPUT-backup.json');await fs.writeFile(fixture,encodeResourceBackup(record));const provider=new AlternativeControlProvider(extension.extensionUri,storage);providers.push(provider);
  const doc=await provider.openCustomDocument(uri,{backupId:vscode.Uri.file(fixture).toString()},none);documents.push(doc);assert.equal(doc.source,source);return {provider,doc,file,uri};}
 async function files(uri){const directory=path.join(root,'storage/native-authority-backups',resourceDirectoryKey(uri.toString()));let names=[];try{names=await fs.readdir(directory);}catch{}return Promise.all(names.filter(n=>n.endsWith('.json')).map(async name=>({file:path.join(directory,name),record:decodeResourceBackup(await fs.readFile(path.join(directory,name),'utf8'),uri.toString())})));}
 try{
  const raw=await fs.readFile(process.env.TERRAPHIM_BACKUP_NORMALIZATION_INPUT,'utf8');
  const normalizationFile=path.join(output,'normalization-INPUT.md');await fs.writeFile(normalizationFile,raw);
  const normalizer=new AlternativeControlProvider(extension.extensionUri,storage);providers.push(normalizer);const normalizedDoc=await normalizer.openCustomDocument(vscode.Uri.file(normalizationFile),{},none);documents.push(normalizedDoc);
  const [firstWorker,secondWorker]=await Promise.all([normalizer.native(normalizedDoc),normalizer.native(normalizedDoc)]);assert.equal(firstWorker,secondWorker,'concurrent initialization must resolve to one actual native worker');
  const initialized=await firstWorker.snapshot();assert.notEqual(initialized.source,raw,'actual annotation context canonicalizes');assert.equal(normalizedDoc.source,initialized.source);assert.equal(hash(normalizedDoc.source),initialized.sourceHash);assert.equal(normalizedDoc.diskSource,raw);assert.equal(normalizedDoc.recoveryDirty,true);assert.equal(await fs.readFile(normalizationFile,'utf8'),raw);
  await firstWorker.attachView('official-initialization-fixture');const request={...normalizer.authorityBase(normalizedDoc),protocol:1,viewSession:'official-initialization-fixture',requestId:1,dispatchId:1};
  const prepared=await firstWorker.prepareAction(request,{expectedBody:initialized.projection.body,meta:{kind:'overflow_set',textRef:0},textArgs:['Actual prepared then aborted scratch']});assert.equal(prepared.kind,'prepared');
  const aborted=await firstWorker.abort(request,prepared.token);assert.equal(aborted.kind,'guarded-aborted');await firstWorker.ackOutcome(request,aborted,normalizer.authorityBase(normalizedDoc));assert.equal((await firstWorker.snapshot()).source,normalizedDoc.source);
  receipt.checks.push({id:'one-real-worker-canonical-initialization-matches-editable-headers',status:'PASS',workerThreadId:firstWorker.worker.threadId});
  const headerFile=path.join(output,'sync-normalization-INPUT.md');await fs.writeFile(headerFile,raw);const headerDoc=await normalizer.openCustomDocument(vscode.Uri.file(headerFile),{},none);documents.push(headerDoc);
  const headerPanel=vscode.window.createWebviewPanel('terraphim-backup-verification','Actual canonical initialization sync',vscode.ViewColumn.One,{enableScripts:true});
  try{await normalizer.resolveCustomEditor(headerDoc,headerPanel,none);const view=[...headerDoc.views][0];let loaded=false,observed;
   const observer=headerPanel.webview.onDidReceiveMessage(message=>{if(message.type==='fixture-loaded')loaded=true;if(message.type==='fixture-sync')observed=message.snapshot;});
   try{headerPanel.webview.html='<html><body>Actual canonical sync verification<script>const api=acquireVsCodeApi();window.addEventListener("message",event=>{if(event.data.type==="sync")api.postMessage({type:"fixture-sync",snapshot:event.data});});api.postMessage({type:"fixture-loaded"});</script></body></html>';
    let deadline=Date.now()+10000;while(!loaded&&Date.now()<deadline)await new Promise(resolve=>setTimeout(resolve,25));assert.ok(loaded);await normalizer.sync(headerDoc,view,'sync',{authoritySnapshot:{source:'obsolete supplied metadata'}});
    deadline=Date.now()+10000;while(!observed&&Date.now()<deadline)await new Promise(resolve=>setTimeout(resolve,25));assert.ok(observed);assert.equal(observed.source,observed.authoritySnapshot.source);assert.equal(observed.sourceHash,observed.authoritySnapshot.sourceHash);assert.equal(observed.revision,observed.authoritySnapshot.revision);assert.equal(observed.historyGeneration,observed.authoritySnapshot.historyGeneration);assert.equal(observed.authorityEpoch,observed.authoritySnapshot.authorityEpoch);assert.equal(await fs.readFile(headerFile,'utf8'),raw);
    await fs.writeFile(path.join(output,'00-actual-canonical-sync.json'),JSON.stringify(observed,null,2));receipt.checks.push({id:'actual-webview-initial-sync-has-coherent-canonical-native-tuple',status:'PASS'});
   }finally{observer.dispose();}
  }finally{headerPanel.dispose();}
  const retiredFile=path.join(output,'retired-initialization.md');await fs.writeFile(retiredFile,raw);const retiredDoc=await normalizer.openCustomDocument(vscode.Uri.file(retiredFile),{},none);documents.push(retiredDoc);
  const oldInitialization=normalizer.native(retiredDoc).then(()=>null,error=>error),oldEpoch=retiredDoc.authorityEpoch;
  const replacement=raw.replace('EXTERNAL','NEW-EXTERNAL');assert.notEqual(replacement,raw);
  const resetting=normalizer.resetAuthority(retiredDoc,async()=>{await fs.writeFile(retiredFile,replacement);const actual=await fs.readFile(retiredFile,'utf8');retiredDoc.source=actual;retiredDoc.diskSource=actual;retiredDoc.checkpoint=undefined;retiredDoc.revision++;retiredDoc.historyGeneration++;});
  const newInitialization=normalizer.native(retiredDoc);await resetting;const oldError=await oldInitialization;assert.ok(oldError instanceof Error);assert.match(oldError.message,/retired/);const newWorker=await newInitialization,newSnapshot=await newWorker.snapshot();assert.notEqual(retiredDoc.authorityEpoch,oldEpoch);assert.equal(newSnapshot.authorityEpoch,retiredDoc.authorityEpoch);assert.equal(newSnapshot.source,retiredDoc.source);assert.equal(newSnapshot.sourceHash,hash(retiredDoc.source));assert.equal(await normalizer.native(retiredDoc),newWorker);assert.equal(await fs.readFile(retiredFile,'utf8'),replacement);
  receipt.checks.push({id:'retired-real-initializer-cannot-clear-new-worker-or-start-before-reset-source',status:'PASS'});
  const a=await open('ordered-save'),held=holdRename();
  const backup=a.provider.backupCustomDocument(a.doc,{destination:vscode.Uri.joinPath(storage,'unused')},none);await held.signal;
  let saved=false;const saving=a.provider.saveCustomDocument(a.doc,none).then(()=>{saved=true;});await new Promise(r=>setImmediate(r));assert.equal(saved,false);assert.equal(await fs.readFile(a.file,'utf8'),initial);
  held.release();const completed=await backup;await saving;assert.equal(await fs.readFile(a.file,'utf8'),source);assert.deepEqual(await files(a.uri),[]);completed.delete();await a.doc.queue;
  await fs.copyFile(a.file,path.join(output,'01-actual-saved-after-backup.md'));receipt.checks.push({id:'backup-write-before-save-and-token-cleanup',status:'PASS'});
  const c=await open('cancelled-after-rename'),cancel=new vscode.CancellationTokenSource(),pause=holdRename();
  const interrupted=c.provider.backupCustomDocument(c.doc,{destination:vscode.Uri.joinPath(storage,'unused')},cancel.token);await pause.signal;cancel.cancel();pause.release();await assert.rejects(interrupted,/Cancel/i);
  const durable=await files(c.uri);assert.equal(durable.length,1);assert.equal(durable[0].record.source,source);assert.equal(await fs.readFile(c.file,'utf8'),initial);await fs.copyFile(durable[0].file,path.join(output,'02-actual-post-rename-cancelled-backup.json'));cancel.dispose();receipt.checks.push({id:'post-rename-cancellation-retains-completed-private-copy',status:'PASS'});
  const one=await open('two-windows'),foreign=new AlternativeControlProvider(extension.extensionUri,storage);providers.push(foreign);
  const inputRecord={kind:'terraphim-resource-backup',version:1,resource:one.uri.toString(),authorityEpoch:randomUUID(),token:randomUUID(),baselineHash:hash(initial),source,drafts:[{viewSession:'other-window',sequence:1,draft:'other window raw draft'}]};const inputPath=path.join(output,'foreign-INPUT-backup.json');await fs.writeFile(inputPath,encodeResourceBackup(inputRecord));
  const two=await foreign.openCustomDocument(one.uri,{backupId:vscode.Uri.file(inputPath).toString()},none);documents.push(two);
  const old=await one.provider.backupCustomDocument(one.doc,{destination:storage},none),other=await foreign.backupCustomDocument(two,{destination:storage},none),newer=await one.provider.backupCustomDocument(one.doc,{destination:storage},none);
  old.delete();await one.doc.queue;assert.equal((await files(one.uri)).length,2);const otherBytes=await vscode.workspace.fs.readFile(vscode.Uri.parse(other.id));
  await one.provider.saveCustomDocument(one.doc,none);newer.delete();await one.doc.queue;assert.equal((await files(one.uri)).length,1);assert.deepEqual(await vscode.workspace.fs.readFile(vscode.Uri.parse(other.id)),otherBytes);
  await fs.writeFile(path.join(output,'03-preserved-other-window.json'),otherBytes);receipt.checks.push({id:'same-uri-other-authority-and-newer-snapshot-survive-old-delete',status:'PASS'});
  if(process.env.TERRAPHIM_BACKUP_CASE!=='accepted-archive'){
  const stale=await open('stale-conflict-notification'),panel=vscode.window.createWebviewPanel('terraphim-backup-verification','Actual host protocol fixture',vscode.ViewColumn.One,{enableScripts:true});
  try{await stale.provider.resolveCustomEditor(stale.doc,panel,none);const view=[...stale.doc.views][0],rawDraft='Retained raw input while another accepted view advanced';
   const identity={protocol:1,resource:stale.uri.toString(),session:view.session,authorityEpoch:stale.doc.authorityEpoch};let observed;
   const observer=panel.webview.onDidReceiveMessage(message=>{if(message.type==='fixture-observed-current-conflict')observed=message.snapshot;});
   try{const draft={...identity,type:'authority-draft-chunk',token:'real-protocol-fixture',draftSequence:1,index:0,total:1,bytes:Buffer.byteLength(rawDraft),text:rawDraft};
    const notification={...identity,type:'external-adoption-conflict',revision:0,sourceHash:'0'.repeat(64),historyGeneration:-1};
    panel.webview.html='<html><body>Actual host protocol verification<script>const api=acquireVsCodeApi();window.addEventListener("message",event=>{const m=event.data;if(m.type==="authority-draft-received")api.postMessage('+JSON.stringify(notification)+');if(m.type==="resource-conflict"&&m.externalAdoptionConflict)api.postMessage({type:"fixture-observed-current-conflict",snapshot:m});});api.postMessage('+JSON.stringify(draft)+');</script></body></html>';
    const deadline=Date.now()+15000;while(!observed&&Date.now()<deadline)await new Promise(resolve=>setTimeout(resolve,25));assert.ok(observed,'actual webview received preservation sync despite stale revision');
    assert.equal(stale.doc.blocked,true);assert.equal(stale.doc.source,source);assert.equal(stale.doc.diskSource,initial);assert.equal(observed.revision,stale.doc.revision);assert.equal(observed.sourceHash,hash(source));assert.equal(observed.historyGeneration,stale.doc.historyGeneration);
    const archive=JSON.parse(await fs.readFile(vscode.Uri.parse(stale.doc.recoveryArchive).fsPath,'utf8').catch(()=>adapter.readFile(vscode.Uri.parse(stale.doc.recoveryArchive)).then(value=>Buffer.from(value).toString('utf8'))));assert.equal(archive.source,source);assert.equal(archive.drafts[0].draft,rawDraft);
    await fs.writeFile(path.join(output,'05-actual-stale-conflict-archive.json'),JSON.stringify(archive,null,2));await fs.writeFile(path.join(output,'05-actual-current-conflict-sync.json'),JSON.stringify(observed,null,2));receipt.checks.push({id:'actual-webview-stale-notification-preserves-raw-and-rebases-current-conflict',status:'PASS'});
   }finally{observer.dispose();}
  }finally{panel.dispose();}
  }
  const acceptedOnly=await open('accepted-unsaved-archive'),archivePanel=vscode.window.createWebviewPanel('terraphim-backup-verification','Actual accepted-unsaved recovery fixture',vscode.ViewColumn.One,{enableScripts:true});
  try{await acceptedOnly.provider.resolveCustomEditor(acceptedOnly.doc,archivePanel,none);const view=[...acceptedOnly.doc.views][0];assert.equal(acceptedOnly.doc.drafts.size,0);assert.equal(acceptedOnly.doc.pendingAuthority,undefined);
   const request={protocol:1,type:'recover-archive',resource:acceptedOnly.uri.toString(),session:view.session,revision:acceptedOnly.doc.revision,sourceHash:hash(source)};
   archivePanel.webview.html='<html><body>Actual accepted-unsaved recovery request<script>acquireVsCodeApi().postMessage('+JSON.stringify(request)+');</script></body></html>';
   const deadline=Date.now()+10000;while(!acceptedOnly.doc.recoveryArchive&&Date.now()<deadline)await new Promise(resolve=>setTimeout(resolve,25));assert.ok(acceptedOnly.doc.recoveryArchive,'actual accepted unsaved source must produce a recovery archive without a raw draft');
   const archive=JSON.parse(Buffer.from(await adapter.readFile(vscode.Uri.parse(acceptedOnly.doc.recoveryArchive))).toString('utf8'));assert.equal(archive.source,source);assert.deepEqual(archive.drafts,[]);assert.equal(await fs.readFile(acceptedOnly.file,'utf8'),initial);
   await fs.writeFile(path.join(output,'06-actual-accepted-unsaved-archive.json'),JSON.stringify(archive,null,2));receipt.checks.push({id:'actual-open-recovery-request-archives-accepted-unsaved-source-without-raw-draft',status:'PASS'});
  }finally{archivePanel.dispose();}
  const externalFile=path.join(output,'idle-external.md');await fs.writeFile(externalFile,initial);
  const idleProvider=new AlternativeControlProvider(extension.extensionUri,storage);providers.push(idleProvider);
  const idleDoc=await idleProvider.openCustomDocument(vscode.Uri.file(externalFile),{},none);documents.push(idleDoc);const previousEpoch=idleDoc.authorityEpoch;
  const externalSource='Risk changed externally.\n';await fs.writeFile(externalFile,externalSource);await idleProvider.external(idleDoc);
  assert.equal(idleDoc.source,externalSource);assert.equal(idleDoc.diskSource,externalSource);assert.notEqual(idleDoc.authorityEpoch,previousEpoch);assert.equal(idleDoc.blocked,false);
  await fs.copyFile(externalFile,path.join(output,'04-actual-idle-external.md'));receipt.checks.push({id:'idle-real-external-file-adoption-resets-native-authority',status:'PASS'});
  const {exportKnowledgeGraph}=require(path.join(extension.extensionPath,'media/knowledge-graph.cjs'));const graph=JSON.parse(await exportKnowledgeGraph(await fs.readFile(a.file,'utf8'),path.join(extension.extensionPath,'media/editor')));assert.equal(graph.data.risk.id,graph.data.concern.id);await fs.writeFile(path.join(output,'01-actual-saved-graph.json'),JSON.stringify(graph,null,2));
  receipt.status='PASS';
 }catch(e){receipt.status='FAIL';receipt.error=String(e.stack||e);throw e;}
 finally{for(const doc of documents)doc.dispose();for(const provider of providers)provider.dispose();registration.dispose();events.dispose();neverCancelled.dispose();await fs.writeFile(path.join(output,'receipt.json'),JSON.stringify(receipt,null,2));}
};
