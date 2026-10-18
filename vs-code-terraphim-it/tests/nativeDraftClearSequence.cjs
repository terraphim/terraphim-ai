/* Actual official VSCode provider, webview adapter and Rust worker. Gate only a real ACK. */
'use strict';
const vscode=require('vscode'),fs=require('node:fs/promises'),path=require('node:path'),assert=require('node:assert/strict'),{createHash}=require('node:crypto');
exports.run=async()=>{
 const output=process.env.TERRAPHIM_DRAFT_CLEAR_OUTPUT;assert(output?.startsWith('/tmp/'));await fs.mkdir(output,{recursive:true});
 const extension=vscode.extensions.getExtension('applied-knowledge-systems.terraphim-it');assert(extension);
 const {AlternativeControlProvider}=require(path.join(extension.extensionPath,'out/alternativeControl'));
 const file=path.join(output,'actual.md');await fs.writeFile(file,'Risk needs mitigation.\n');
 const provider=new AlternativeControlProvider(extension.extensionUri,vscode.Uri.file(path.join(output,'storage'))),cancellation=new vscode.CancellationTokenSource();
 const doc=await provider.openCustomDocument(vscode.Uri.file(file),{},cancellation.token);
 const panel=vscode.window.createWebviewPanel('terraphim-draft-clear-verification','Actual draft-clear sequence regression',vscode.ViewColumn.One,{enableScripts:true});
 const receipt={status:'RUNNING',scope:'Actual official VSCode provider/real webview adapter/real Rust worker ACK timing regression; not native keyboard or release qualification',checks:[]};let release,gateReached,held=false;const reached=new Promise(r=>gateReached=r),gate=new Promise(r=>release=r);
 const originalReply=provider.authorityReply.bind(provider);provider.authorityReply=async function(document,view,message,value){if(message.type==='authority-clear-draft'&&!held){held=true;gateReached();await gate;}return originalReply(document,view,message,value);};
 let observer;const observations=[];const wait=async(check,label)=>{const end=Date.now()+15000;while(!check()){if(Date.now()>end)throw new Error('Timeout: '+label);await new Promise(r=>setTimeout(r,25));}};
 try{
  await provider.resolveCustomEditor(doc,panel,cancellation.token);const view=[...doc.views][0];
  observer=panel.webview.onDidReceiveMessage(message=>{if(message.type?.startsWith('fixture-'))observations.push(message);});
  const nativeSource=await fs.readFile(path.join(extension.extensionPath,'media/native-authority.js'),'utf8');receipt.adapterSha256=createHash('sha256').update(nativeSource).digest('hex');
  const hook=`
window.addEventListener('message',event=>{const m=event.data;if(m.type==='fixture-start')void(async()=>{try{recordDraft(rawDraft());await adapter.historyChanged();vscode.postMessage({type:'fixture-first-complete',draftSequence,clearedDraft,state:vscode.getState()});}catch(error){vscode.postMessage({type:'fixture-error',error:String(error)});}})();if(m.type==='fixture-later')recordDraft(JSON.stringify({publicFixture:'Later raw draft which must remain retained'}));if(m.type==='fixture-second')void(async()=>{try{await adapter.historyChanged();vscode.postMessage({type:'fixture-second-complete',draftSequence,clearedDraft,state:vscode.getState()});}catch(error){vscode.postMessage({type:'fixture-error',error:String(error)});}})();});
`;
  const instrumented=nativeSource.replace('md().setAuthority(adapter);ready=true;','md().setAuthority(adapter);ready=true;vscode.postMessage({type:"fixture-ready"});').replace(' vscode.postMessage({protocol:1,type:\'ready\'});',hook+' vscode.postMessage({protocol:1,type:\'ready\'});');assert.notEqual(instrumented,nativeSource);
  const html=panel.webview.html;const tag=/<script nonce="([^"]+)" src="[^"]*native-authority\.js"><\/script>/;assert(tag.test(html),'actual provider native adapter script');panel.webview.html=html.replace(tag,(_,nonce)=>`<script nonce="${nonce}">${instrumented.replace(/<\/script/gi,'<\\/script')}</script>`);
  await wait(()=>observations.some(x=>x.type==='fixture-ready'),'real adapter and native worker initialization');const source=(await provider.native(doc)).snapshot();await source;
  await panel.webview.postMessage({type:'fixture-start'});await reached;
  assert.equal(doc.drafts.size,0,'actual provider removed only first submitted draft before gating real ACK');
  await panel.webview.postMessage({type:'fixture-later'});await wait(()=>doc.drafts.get(view.session)?.sequence===2,'real later raw draft reached provider');
  release();await wait(()=>observations.some(x=>x.type==='fixture-first-complete'||x.type==='fixture-error'),'actual ACK received');
  const first=observations.find(x=>x.type==='fixture-first-complete');assert(first,JSON.stringify(observations));receipt.first=first;assert.equal(doc.drafts.get(view.session)?.sequence,2);assert.equal(first.draftSequence,2);assert(first.clearedDraft<first.draftSequence,'newer draft was locally acknowledged by an older real ACK');assert.equal(first.state.sequence,2);
  receipt.checks.push({id:'older-real-ack-does-not-retire-later-draft',status:'PASS'});
  await panel.webview.postMessage({type:'fixture-second'});await wait(()=>observations.some(x=>x.type==='fixture-second-complete'),'actual second clear acknowledgement');const second=observations.find(x=>x.type==='fixture-second-complete');assert.equal(doc.drafts.size,0);assert.equal(second.draftSequence,second.clearedDraft);assert.equal(second.state,undefined);
  const final=await (await provider.native(doc)).snapshot();assert.equal(final.source,'Risk needs mitigation.\n');assert.equal(await fs.readFile(file,'utf8'),final.source);receipt.workerThreadId=(await provider.native(doc)).worker.threadId;receipt.checks.push({id:'next-real-clear-retires-exact-draft-without-source-change',status:'PASS'});receipt.status='PASS';
 }catch(error){receipt.status='FAIL';receipt.error=String(error);throw error;}finally{release?.();receipt.observations=observations;await fs.writeFile(path.join(output,'receipt.json'),JSON.stringify(receipt,null,2)+'\n');observer?.dispose();panel.dispose();doc.dispose();provider.dispose();cancellation.dispose();}
};
