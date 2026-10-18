/* Proposed official-host regression. Real provider/worker/IO; gate only delivery of a real title message. */
'use strict';
const vscode=require('vscode'),fs=require('node:fs/promises'),path=require('node:path'),assert=require('node:assert/strict'),{createHash}=require('node:crypto');
exports.run=async()=>{
 const output=process.env.TERRAPHIM_TITLE_TRANSIT_OUTPUT;assert(output?.startsWith('/tmp/'));await fs.mkdir(output,{recursive:true});
 const extension=vscode.extensions.getExtension('applied-knowledge-systems.terraphim-it');assert(extension);
 const {AlternativeControlProvider}=require(path.join(extension.extensionPath,'out/alternativeControl'));
 const file=path.join(output,'actual.md'),initial='Risk needs mitigation.\n';await fs.writeFile(file,initial);
 const provider=new AlternativeControlProvider(extension.extensionUri,vscode.Uri.file(path.join(output,'storage'))),cancel=new vscode.CancellationTokenSource();
 const doc=await provider.openCustomDocument(vscode.Uri.file(file),{},cancel.token),panel=vscode.window.createWebviewPanel('terraphim-title-transit-verification','Actual title command in transit',vscode.ViewColumn.One,{enableScripts:true});
 const observations=[],receipt={status:'RUNNING',scope:'Actual official VSCode provider/webview/native worker/save IO with delayed real title delivery; not a native keyboard or release claim',checks:[]};let release,held=false,reached;const gate=new Promise(r=>release=r),hit=new Promise(r=>reached=r);let observer,save;
 const wait=async(check,label)=>{const end=Date.now()+15000;while(!check()){if(Date.now()>end)throw new Error('Timeout: '+label);await new Promise(r=>setTimeout(r,25));}};
 try{
  await provider.resolveCustomEditor(doc,panel,cancel.token);const view=[...doc.views][0];observer=panel.webview.onDidReceiveMessage(message=>{if(message.type?.startsWith('fixture-'))observations.push(message);});
  const source=await fs.readFile(path.join(extension.extensionPath,'media/native-authority.js'),'utf8');receipt.adapterSha256=createHash('sha256').update(source).digest('hex');
  const hook=`window.addEventListener('message',event=>{if(event.data.type==='fixture-seed')void(async()=>{try{await md().surface.replaceRange(0,4,'Concern');await adapter.historyChanged();await md().authoritySettled();vscode.postMessage({type:'fixture-seeded'});}catch(error){vscode.postMessage({type:'fixture-error',error:String(error)});}})();});`;
  const instrumented=source.replace('md().setAuthority(adapter);ready=true;','md().setAuthority(adapter);ready=true;vscode.postMessage({type:"fixture-ready"});').replace(' vscode.postMessage({protocol:1,type:\'ready\'});',hook+' vscode.postMessage({protocol:1,type:\'ready\'});');
  const tag=/<script nonce="([^"]+)" src="[^"]*native-authority\.js"><\/script>/;assert(tag.test(panel.webview.html));panel.webview.html=panel.webview.html.replace(tag,(_,nonce)=>`<script nonce="${nonce}">${instrumented.replace(/<\/script/gi,'<\\/script')}</script>`);
  await wait(()=>observations.some(x=>x.type==='fixture-ready'),'actual native adapter ready');await panel.webview.postMessage({type:'fixture-seed'});await wait(()=>observations.some(x=>x.type==='fixture-seeded'||x.type==='fixture-error'),'real native source/history seed');assert(observations.some(x=>x.type==='fixture-seeded'),JSON.stringify(observations));
  await provider.saveCustomDocument(doc,cancel.token);const before=await fs.readFile(file,'utf8');assert.equal(before,'Concern needs mitigation.\n');
  const actualPost=panel.webview.postMessage.bind(panel.webview);panel.webview.postMessage=async message=>{if(message.type==='request-history'&&!held){held=true;receipt.nonceIssuedAt=Date.now();receipt.issuedHistory=view.historyCommands.size;reached();await gate;}return actualPost(message);};
  assert(panel.active&&vscode.window.state.focused,'Official test window must actually be focused');const title=provider.activeCommand('request-history','undo');await hit;assert.equal(view.historyCommands.size,1);
  let completed=false;save=provider.saveCustomDocument(doc,cancel.token).then(()=>{completed=true;});
  // A bounded negative assertion: already-issued title delivery is deliberately
  // in transit. It must not be bypassed by an acknowledged old-source Save.
  await Promise.race([save,new Promise(r=>setTimeout(r,250))]);receipt.saveCompletedBeforeRelease=completed;receipt.diskBeforeReleaseSha256=createHash('sha256').update(await fs.readFile(file)).digest('hex');receipt.pendingHistoryBeforeRelease=view.historyCommands.size;
  assert.equal(completed,false,'Save bypassed an already-issued real title command in transit');
  release();await title;await wait(()=>view.historyCommands.size===0&&doc.source===initial,'real title Undo accepted and retired');await save;assert.equal(await fs.readFile(file,'utf8'),initial);assert.equal((await (await provider.native(doc)).snapshot()).source,initial);receipt.checks.push({id:'issued-title-history-settles-before-real-save',status:'PASS'});
  const head=view.historyCommands.issue(view.epoch);
  await assert.rejects(provider.persist(doc,doc.uri,cancel.token,true),/Pending title history/);
  assert.equal(await fs.readFile(file,'utf8'),initial);assert.equal(view.historyCommands.size,1);
  assert.equal(view.historyCommands.completeHead(head.nonce,view.epoch,view.epoch),true);
  receipt.checks.push({id:'direct-already-flushed-persist-refuses-pending-history',status:'PASS'});
  view.historyCommands.issue(view.epoch);const retired=provider.saveCustomDocument(doc,cancel.token);
  await wait(()=>view.historyCommands.waiters.size===1,'actual Save drain waiter');
  const rejection=assert.rejects(retired,/retired/);view.historyCommands.clear();await rejection;
  assert.equal(await fs.readFile(file,'utf8'),initial);assert.equal(doc.source,initial);
  receipt.checks.push({id:'retired-view-history-rejects-save-without-changing-disk',status:'PASS'});receipt.status='PASS';
 }catch(error){receipt.status='FAIL';receipt.error=String(error);throw error;}finally{release?.();if(save)await save.catch(()=>undefined);receipt.observations=observations;await fs.writeFile(path.join(output,'receipt.json'),JSON.stringify(receipt,null,2)+'\n');observer?.dispose();panel.dispose();doc.dispose();provider.dispose();cancel.dispose();}
};
