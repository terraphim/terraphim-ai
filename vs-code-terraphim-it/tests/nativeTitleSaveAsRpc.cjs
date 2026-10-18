/* Official extensionTestsPath fixture: actual provider/worker/IO and frontend RPC.
 * Disposable test extension package must contribute customEditors viewType
 * terraphim.title-rpc-regression for *.md (priority option). No runtime API shim.
 * Gate forwards the original registered saveCustomDocument; it never fakes results. */
'use strict';
const vscode=require('vscode'),fs=require('node:fs/promises'),path=require('node:path'),assert=require('node:assert/strict'),{createHash}=require('node:crypto');
exports.run=async()=>{
 const output=process.env.TERRAPHIM_TITLE_SAVEAS_OUTPUT;assert(output?.startsWith('/tmp/'));await fs.mkdir(output,{recursive:true});
 const extension=vscode.extensions.getExtension('applied-knowledge-systems.terraphim-it');assert(extension);
 const {AlternativeControlProvider}=require(path.join(extension.extensionPath,'out/alternativeControl'));
 const file=path.join(output,'actual.md'),initial='Risk needs mitigation.\n';await fs.writeFile(file,initial);
 const provider=new AlternativeControlProvider(extension.extensionUri,vscode.Uri.file(path.join(output,'storage'))),cancel=new vscode.CancellationTokenSource();
 let doc,panel,observer,registration;const observations=[],receipt={status:'RUNNING',scope:'Actual registered custom editor/frontend Save RPC; real native Save entry forwards to actual provider SaveAs target. Not native picker/routing qualification. Real worker and filesystem.',checks:[]};let release,reached;const gate=new Promise(r=>release=r),hit=new Promise(r=>reached=r);
 const wait=async(check,label)=>{const end=Date.now()+15000;while(!check()){if(Date.now()>end)throw new Error('Timeout: '+label);await new Promise(r=>setTimeout(r,25));}};
 try{
  const actualOpen=provider.openCustomDocument.bind(provider),actualResolve=provider.resolveCustomEditor.bind(provider);
  provider.openCustomDocument=async(...args)=>{doc=await actualOpen(...args);return doc;};
  provider.resolveCustomEditor=async(document,actualPanel,token)=>{panel=actualPanel;await actualResolve(document,panel,token);observer=panel.webview.onDidReceiveMessage(message=>{if(message.type?.startsWith('fixture-'))observations.push(message);});
  const source=await fs.readFile(path.join(extension.extensionPath,'media/native-authority.js'),'utf8');receipt.adapterSha256=createHash('sha256').update(source).digest('hex');
  const hook=`window.addEventListener('message',event=>{if(event.data.type==='fixture-seed')void(async()=>{try{await md().surface.replaceRange(0,4,'Concern');await adapter.historyChanged();await md().authoritySettled();vscode.postMessage({type:'fixture-seeded'});}catch(error){vscode.postMessage({type:'fixture-error',error:String(error)});}})();if(event.data.type==='fixture-rpc-save')void(async()=>{try{await settled();await serialize(()=>rpc('save'));vscode.postMessage({type:'fixture-rpc-save-ok'});}catch(error){vscode.postMessage({type:'fixture-rpc-save-refused',error:String(error)});}})();});`;
  const instrumented=source.replace('md().setAuthority(adapter);ready=true;','md().setAuthority(adapter);ready=true;vscode.postMessage({type:"fixture-ready"});').replace(' vscode.postMessage({protocol:1,type:\'ready\'});',hook+' vscode.postMessage({protocol:1,type:\'ready\'});');
  const tag=/<script nonce="([^"]+)" src="[^"]*native-authority\.js"><\/script>/;assert(tag.test(panel.webview.html));panel.webview.html=panel.webview.html.replace(tag,(_,nonce)=>`<script nonce="${nonce}">${instrumented.replace(/<\/script/gi,'<\\/script')}</script>`);
  };
  registration=vscode.window.registerCustomEditorProvider('terraphim.title-rpc-regression',provider,{webviewOptions:{retainContextWhenHidden:true},supportsMultipleEditorsPerDocument:true});
  await vscode.commands.executeCommand('vscode.openWith',vscode.Uri.file(file),'terraphim.title-rpc-regression');
  await wait(()=>observations.some(x=>x.type==='fixture-ready'),'registered real native adapter ready');
  await panel.webview.postMessage({type:'fixture-seed'});await wait(()=>observations.some(x=>x.type==='fixture-seeded'||x.type==='fixture-error'),'real native source/history seed');assert(observations.some(x=>x.type==='fixture-seeded'),JSON.stringify(observations));
  const view=[...doc.views][0],actualSave=(document,token)=>provider.saveCustomDocumentAs(document,vscode.Uri.file(path.join(output,'actual-save-as.md')),token);let held=false;
  provider.saveCustomDocument=async(...args)=>{if(args[0]===doc&&!held){held=true;receipt.commandSaveAtEntry=doc.commandSave;receipt.saveEnteredAt=Date.now();reached();await gate;}try{return await actualSave(...args);}catch(error){receipt.actualSaveError=String(error);receipt.actualSaveRejectedAt=Date.now();throw error;}};
  await panel.webview.postMessage({type:'fixture-rpc-save'});await hit;assert.equal(receipt.commandSaveAtEntry,true,'actual frontend RPC must naturally own commandSave');
  await provider.activeCommand('request-history','undo');assert.equal(view.historyCommands.size,1);receipt.issuedHistory=1;
  release();await wait(()=>observations.some(x=>x.type==='fixture-rpc-save-refused'||x.type==='fixture-rpc-save-ok'),'real RPC terminal outcome');assert(observations.some(x=>x.type==='fixture-rpc-save-refused'),'Native workbench swallowed actual Save refusal: '+JSON.stringify(receipt));receipt.refusedAfterMs=Date.now()-receipt.saveEnteredAt;assert(receipt.refusedAfterMs<5000,'must not wait for the 15 second command timeout');
  await wait(()=>view.historyCommands.size===0&&doc.source===initial&&!doc.commandSave,'real rejected save RPC releases renderer queue and Undo completes');
  receipt.checks.push({id:'actual-rpc-saveas-refuses-without-title-self-deadlock',status:'PASS'});
  assert.equal(await fs.readFile(file,'utf8'),initial);
  await panel.webview.postMessage({type:'fixture-rpc-save'});await wait(()=>observations.some(x=>x.type==='fixture-rpc-save-ok'),'fresh actual RPC Save');
  assert.equal(await fs.readFile(file,'utf8'),initial);assert.equal(await fs.readFile(path.join(output,'actual-save-as.md'),'utf8'),initial);assert.equal((await (await provider.native(doc)).snapshot()).source,initial);
  receipt.checks.push({id:'fresh-rpc-saveas-persists-actual-undone-source-to-new-target',status:'PASS'});receipt.status='PASS';
 }catch(error){receipt.status='FAIL';receipt.error=String(error);throw error;}finally{release?.();receipt.observations=observations;await fs.writeFile(path.join(output,'receipt.json'),JSON.stringify(receipt,null,2)+'\n');observer?.dispose();panel?.dispose();doc?.dispose();registration?.dispose();provider.dispose();cancel.dispose();}
};
