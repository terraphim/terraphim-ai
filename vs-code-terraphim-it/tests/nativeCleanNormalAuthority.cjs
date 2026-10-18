/* Real provider/worker/normal-editor/IO regression. Public backup and scheduling gates are INPUTS. */
'use strict';
const vscode=require('vscode'),fs=require('node:fs/promises'),path=require('node:path'),assert=require('node:assert/strict'),{createHash}=require('node:crypto');
const hash=x=>createHash('sha256').update(x).digest('hex');
exports.run=async()=>{
 const extension=vscode.extensions.getExtension('applied-knowledge-systems.terraphim-it');assert(extension);
 const providerFile=path.join(extension.extensionPath,'out/alternativeControl.js'),providerSource=await fs.readFile(providerFile,'utf8');
 const {AlternativeControlProvider}=require(providerFile),fixture=JSON.parse(await fs.readFile(path.join(__dirname,'fixtures/retained-recovery-public-input.json'),'utf8')),output=process.env.TERRAPHIM_RETAINED_OUTPUT;await fs.mkdir(output,{recursive:true});
 const constructorCatchLine=providerSource.split('\n').findIndex(x=>x.includes('this.external(doc, true).catch'))+1;
 const receipt={status:'RUNNING',providerSHA256:hash(providerSource),scope:'Official actual provider, normal editor, real worker and filesystem. Public backup, received raw envelope and real-method scheduling gates are fixture inputs, not native input/full-host qualification.',checks:[]};
 const wait=async(fn,label)=>{const end=Date.now()+15000;while(!await fn()){if(Date.now()>end)throw Error('Timeout: '+label);await new Promise(r=>setTimeout(r,20));}};
 try{for(const kind of ['cleanOwnSave','cleanExternal','dirtyDuringRead','dirtyDuringDispose','rawDuringDispose','missingFileEvent']){
  const dir=path.join(output,kind);await fs.mkdir(dir,{recursive:true});const uri=vscode.Uri.file(path.join(dir,'actual.md'));await fs.writeFile(uri.fsPath,fixture.baseline);
  const normal=await vscode.workspace.openTextDocument(uri);await vscode.window.showTextDocument(normal);assert.equal(normal.isDirty,false);
  const provider=new AlternativeControlProvider(extension.extensionUri,vscode.Uri.file(path.join(dir,'storage'))),cancel=new vscode.CancellationTokenSource();let doc,panel,listener,release=()=>{};
  const calls=[],blocks=[];const originalExternal=provider.external.bind(provider),originalBlock=provider.block.bind(provider);
  const observeCall=(d,fromNormal,operation)=>{const row={ordinal:calls.length,fromNormal,status:'pending'};calls.push(row);return operation().then(value=>{row.status='fulfilled';return value;},error=>{row.status='rejected';row.error=String(error);throw error;});};
  provider.external=(d,fromNormal)=>observeCall(d,fromNormal,()=>originalExternal(d,fromNormal));
  provider.block=(...args)=>{blocks.push({reason:String(args[1]),stack:new Error().stack});return originalBlock(...args);};
  try{
   if(kind==='cleanOwnSave'){
    const backup=path.join(dir,'accepted-backup-INPUT.md');await fs.writeFile(backup,fixture.accepted);doc=await provider.openCustomDocument(uri,{backupId:vscode.Uri.file(backup).toString()},cancel.token);
    const worker=await provider.native(doc),epoch=doc.authorityEpoch,thread=worker.worker.threadId;assert.equal((await worker.snapshot()).source,fixture.accepted);await provider.saveCustomDocument(doc,cancel.token);
    const before={normalVersion:normal.version,normalDirty:normal.isDirty,normalHash:hash(normal.getText()),sourceHash:hash(doc.source),diskBaselineHash:hash(doc.diskSource),epoch};assert.equal(before.normalDirty,false);assert.notEqual(before.normalHash,before.sourceHash,'Observe actual clean stale model; never fabricate one');const pending=provider.external(doc,true);await fs.writeFile(path.join(dir,'actual-before.json'),JSON.stringify(before,null,2));await pending;
    assert.equal(doc.source,fixture.accepted);assert.equal(doc.diskSource,fixture.accepted);assert.equal(doc.authorityEpoch,epoch);assert.equal((await provider.native(doc)).worker.threadId,thread);assert.equal((await worker.snapshot()).source,fixture.accepted);assert.equal(await fs.readFile(uri.fsPath,'utf8'),fixture.accepted);receipt.checks.push({id:'clean-stale-normal-cannot-reset-verified-own-save',status:'PASS'});
   }else{
    doc=await provider.openCustomDocument(uri,{},cancel.token);const worker=await provider.native(doc),epoch=doc.authorityEpoch;let entered=false;const gate=new Promise(resolve=>release=resolve);
    if(kind==='cleanExternal'){
     const thread=worker.worker.threadId;await fs.writeFile(uri.fsPath,'Actual external replacement.\n');await provider.external(doc,false);assert.equal(doc.source,'Actual external replacement.\n');assert.equal(doc.diskSource,doc.source);assert.notEqual(doc.authorityEpoch,epoch);assert.notEqual((await provider.native(doc)).worker.threadId,thread);assert.equal(await fs.readFile(uri.fsPath,'utf8'),doc.source);receipt.checks.push({id:'legitimate-clean-external-file-adopts-fresh-worker',status:'PASS'});
    }else if(kind==='dirtyDuringRead'){
     const original=provider.readDisk.bind(provider);let gated=false;provider.readDisk=async target=>{const actual=await original(target);if(!gated&&target.toString()===uri.toString()){gated=true;entered=true;await gate;}return actual;};
     const pending=provider.external(doc,true);await wait(()=>entered,'real disk read before normal edit');const edit=new vscode.WorkspaceEdit();edit.insert(uri,new vscode.Position(0,0),'REAL-DIRTY-');assert.equal(await vscode.workspace.applyEdit(edit),true);assert.equal(normal.isDirty,true);release();await pending;assert.equal(doc.blocked,true);assert.equal(doc.source,fixture.baseline);assert.equal(doc.authorityEpoch,epoch);assert.ok(normal.getText().startsWith('REAL-DIRTY-'));assert.equal(await fs.readFile(uri.fsPath,'utf8'),fixture.baseline);receipt.checks.push({id:'normal-edit-during-real-read-remains-conflict',status:'PASS'});
    }else if(kind==='dirtyDuringDispose'||kind==='rawDuringDispose'){
     if(kind==='rawDuringDispose'){
      panel=vscode.window.createWebviewPanel('te-normal-authority-raw-test','Received raw adoption guard',vscode.ViewColumn.One,{enableScripts:true});await provider.resolveCustomEditor(doc,panel,cancel.token);const observed=[];listener=panel.webview.onDidReceiveMessage(m=>{if(m.type==='fixture-ready')observed.push(m);});
      const adapter=await fs.readFile(path.join(extension.extensionPath,'media/native-authority.js'),'utf8'),hook=`window.addEventListener('message',e=>{if(e.data.type==='fixture-observe'&&ready)vscode.postMessage({type:'fixture-ready'});if(e.data.type==='fixture-record')recordDraft(e.data.raw);});`,executed=adapter.replace(" vscode.postMessage({protocol:1,type:'ready'});",hook+" vscode.postMessage({protocol:1,type:'ready'});");assert.notEqual(executed,adapter);await fs.writeFile(path.join(dir,'executed-inspected-adapter.js'),executed);
      const tag=/<script nonce="([^"]+)" src="[^"]*native-authority\.js"><\/script>/;assert(tag.test(panel.webview.html));panel.webview.html=panel.webview.html.replace(tag,(_,nonce)=>`<script nonce="${nonce}">${executed.replace(/<\/script/gi,'<\\/script')}</script>`);await wait(async()=>{await panel.webview.postMessage({type:'fixture-observe'});return observed.length>0;},'actual raw view ready');
     }
     const original=worker.dispose.bind(worker);worker.dispose=async()=>{await original();entered=true;await fs.writeFile(path.join(dir,'actual-retirement-call-context.json'),JSON.stringify(calls,null,2));await gate;};
     await fs.writeFile(uri.fsPath,'Actual external replacement.\n');const pending=provider.external(doc,false);pending.catch(()=>undefined);await wait(()=>entered,'actual worker retirement');
     if(kind==='dirtyDuringDispose'){const edit=new vscode.WorkspaceEdit();edit.insert(uri,new vscode.Position(0,0),'LATE-DIRTY-');assert.equal(await vscode.workspace.applyEdit(edit),true);assert.equal(normal.isDirty,true);}else{await fs.writeFile(path.join(dir,'raw-INPUT.json'),fixture.substantive);await panel.webview.postMessage({type:'fixture-record',raw:fixture.substantive});await wait(()=>[...doc.drafts.values()].some(d=>d.draft===fixture.substantive),'actual received raw transport');}
     release();await pending.catch(()=>undefined);await wait(()=>calls.every(c=>c.status!=='pending'),'all observed real calls settle');assert.ok(calls.some(c=>c.status==='rejected'&&/Pending input arrived during external adoption/.test(c.error)),JSON.stringify(calls));assert.equal(doc.source,fixture.baseline);assert.equal(doc.authorityEpoch,epoch);assert.equal(await fs.readFile(uri.fsPath,'utf8'),'Actual external replacement.\n');
     if(kind==='dirtyDuringDispose')assert.ok(normal.getText().startsWith('LATE-DIRTY-'));else{assert.ok([...doc.drafts.values()].some(d=>d.draft===fixture.substantive));await provider.preserveAuthorityRecovery(doc);const archive=Buffer.from(await vscode.workspace.fs.readFile(vscode.Uri.parse(doc.recoveryArchive))).toString();await fs.writeFile(path.join(dir,'actual-recovery-archive.json'),archive);assert.ok(JSON.parse(archive).drafts.some(d=>d.draft===fixture.substantive));}
     receipt.checks.push({id:kind==='dirtyDuringDispose'?'normal-edit-during-real-retirement-prevents-adoption':'received-raw-during-real-retirement-is-retained-and-archived',status:'PASS'});
    }else{
     assert.ok(constructorCatchLine>0);let fromNormalEntered=false;provider.external=(d,fromNormal)=>observeCall(d,fromNormal,async()=>{if(d===doc){if(fromNormal)fromNormalEntered=true;await gate;}return originalExternal(d,fromNormal);});
     await fs.writeFile(uri.fsPath,'Actual clean reload event.\n');await wait(()=>fromNormalEntered,'actual clean normal reload event');assert.equal(normal.isDirty,false);await fs.unlink(uri.fsPath);release();await wait(()=>calls.some(c=>c.fromNormal&&c.status==='rejected')&&blocks.some(b=>b.stack.includes('alternativeControl.js:'+constructorCatchLine+':')),'actual constructor rejection handled by retained block');assert.equal(doc.blocked,true);assert.equal(doc.source,fixture.baseline);assert.equal(doc.authorityEpoch,epoch);await assert.rejects(fs.readFile(uri.fsPath),{code:'ENOENT'});receipt.checks.push({id:'actual-missing-file-normal-event-rejection-is-caught',status:'PASS'});
    }
   }
  }finally{
   release();if(doc){await fs.writeFile(path.join(dir,'actual-accepted-source.md'),doc.source);await fs.writeFile(path.join(dir,'actual-normal-unsaved-or-clean.md'),normal.getText());let disk,diskError;try{disk=await fs.readFile(uri.fsPath,'utf8');await fs.writeFile(path.join(dir,'actual-disk.md'),disk);}catch(error){diskError=String(error);}await fs.writeFile(path.join(dir,'actual-state.json'),JSON.stringify({normal:{version:normal.version,dirty:normal.isDirty,textHash:hash(normal.getText())},sourceHash:hash(doc.source),diskBaselineHash:hash(doc.diskSource),actualDiskHash:disk===undefined?undefined:hash(disk),diskError,epoch:doc.authorityEpoch,blocked:doc.blocked,drafts:[...doc.drafts.values()],calls,blocks},null,2));}
   listener?.dispose();panel?.dispose();doc?.dispose();provider.dispose();cancel.dispose();
  }
 }
 receipt.status='PASS';}catch(error){receipt.status='FAIL';receipt.error=String(error.stack||error);throw error;}finally{await fs.writeFile(path.join(output,'receipt.json'),JSON.stringify(receipt,null,2));}
};
