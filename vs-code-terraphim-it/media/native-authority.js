/* Resource-owned native transactions; views keep only rendering and existing UI history. */
(() => {
 'use strict';
 if(document.body.dataset.authority!=='true')return;
 const vscode=acquireVsCodeApi(),container=document.getElementById('editor'),notice=document.getElementById('host-notice');
 const hostNotice=new TerraphimHostNotice(notice),assets=document.body.dataset.assets;
 let editor,bindings,base,recoveryPending=false,ready=false,sequence=0,draftSequence=0,receivedDraft=0,clearedDraft=0,acceptedCheckpoint,historyCommand,retainedRegistration,quarantinedDraft,blockingControls;
 let roleGeneration='0',providerGeneration='0',activeView=false,viewEpoch=0,paused=false,adopting=false;
 const requests=new Map(),stages=new Map();let stageTimer;let sending=Promise.resolve(),settling=Promise.resolve(),hostHistory=Promise.resolve(),synchronizing=Promise.resolve();
 const md=()=>editor.markdownEditor;
 const encodeSize=value=>new TextEncoder().encode(value).length;
 const identity=()=>({protocol:1,resource:base.resource,session:base.session,authorityEpoch:base.authorityEpoch,revision:base.revision,sourceHash:base.sourceHash,historyGeneration:base.historyGeneration});
 const rawDraft=()=>{const pending=md().pendingAuthorityDraft?.();const intents=pending?.intents?.length;
  return JSON.stringify({baseSourceHash:base.sourceHash,pending,
   ...(!intents?{body:md().surface.root.textContent===md().surface.getText()?undefined:md().surface.root.textContent,
    alternatives:[...container.querySelectorAll('.te-alt-input')].filter(input=>input.value!==(input.dataset.committed??'')).map(input=>({value:input.value,committed:input.dataset.committed??''})),
    overflow:container.querySelector('.te-overflow-text')?.value}:{} )});};
 function draft(){if(!ready||adopting||paused)return;recordDraft(rawDraft());}
 function recordDraft(value){const bytes=encodeSize(value);
  if(bytes>128*1024*1024){quarantine('Pending draft exceeds recovery capacity',value);return false;}
  if(!Number.isSafeInteger(draftSequence)||draftSequence>=Number.MAX_SAFE_INTEGER){quarantine('Pending draft sequence exceeds recovery capacity',value);return false;}
  draftSequence++;vscode.setState({resource:base.resource,draft:value,sequence:draftSequence});
  const token=crypto.randomUUID(),total=Math.max(1,Math.ceil(value.length/(256*1024)));
  for(let index=0;index<total;index++)vscode.postMessage({...identity(),type:'authority-draft-chunk',token,draftSequence,index,total,bytes,text:value.slice(index*256*1024,(index+1)*256*1024)});
  return true;
 }
 function quarantine(reason,value){quarantinedDraft=value;block(reason,value);}
 function block(reason,localDraft){if(quarantinedDraft!==undefined)localDraft=quarantinedDraft;paused=true;
  const local=localDraft!==undefined;
  // Keep a held pointer's target stable when only the blocking reason changes.
  // A recovery-mode change must still remove the old action and its authority.
  let controls=blockingControls;
  if(notice.dataset.kind!=='blocking'||!controls||controls.local!==local||controls.text.parentNode!==notice||controls.copy.parentNode!==notice||!local&&controls.recover.parentNode!==notice){
   notice.replaceChildren();const text=document.createElement('p'),copy=document.createElement('button'),recover=document.createElement('button');
   controls={text,copy,recover,local,localDraft};blockingControls=controls;
   copy.onclick=()=>{if(controls.local){const retained=document.createElement('textarea');retained.readOnly=true;retained.setAttribute('aria-label','Retained external-change recovery data');retained.value=controls.localDraft;notice.append(retained);retained.focus();retained.select();}else vscode.postMessage({...identity(),type:'recover-archive'});};
   recover.textContent='Recover current file with fresh history';recover.onclick=()=>{void requestRecovery(recover);};
   notice.append(text,copy);if(!local)notice.append(recover);
  }
  controls.localDraft=localDraft;
  controls.text.textContent=local?`${reason}. This version remains in this view; copy the retained data before closing it.`:`${reason}. Accepted source and received drafts are retained. Recovery starts fresh Undo history.`;
  controls.copy.textContent=local?'Select retained recovery data':'Open recovery draft';
  hostNotice.blocking();notice.hidden=false;container.inert=true;
 }
 async function requestRecovery(button){
  if(recoveryPending)return;recoveryPending=true;button.disabled=true;const epoch=base.authorityEpoch;
  try{await rpc('resolve-conflict');}
  catch(error){
   // Only the existing authoritative reverted handler may retire this request.
   // Preserve its recovered or conflict-blocked UI; never adopt state on an error.
   if(!(error?.authorityRetired&&base.authorityEpoch!==epoch))block(`Recovery request refused: ${error?.message??String(error)}`);
  }finally{recoveryPending=false;if(button.isConnected)button.disabled=false;}
 }
 function capture(source=bindings.host_presentation_source(base.source)){return TerraphimHistoryCheckpoint.capture(md().surface,source,md().altPanel);}
 function hasLocalDraft(){if(draftSequence>clearedDraft)return true;try{return JSON.stringify(capture())!==acceptedCheckpoint;}catch{return true;}}
 function stage(checkpoint){const encoded=TerraphimHistoryCheckpoint.chunks(checkpoint),token=crypto.randomUUID();
  for(let index=0;index<encoded.parts.length;index++)vscode.postMessage({...identity(),type:'checkpoint-chunk',token,index,total:encoded.parts.length,bytes:encoded.bytes,text:encoded.parts[index]});return token;
 }
 function rpc(type,fields={}){
  if(paused&&type!=='authority-status'&&type!=='authority-abort'&&type!=='resolve-conflict')return Promise.reject(new Error('Native authority needs explicit recovery'));
  const id=++sequence;return new Promise((resolve,reject)=>{
   // Open/export may wait in an interactive OS picker. Neither mutates this
   // document's authority; actual native mutation/reconciliation deadlines remain.
   const epoch=base.authorityEpoch;const timer=(type==='open'||type==='authority-export')?undefined:setTimeout(()=>{requests.delete(id);const error=new Error('Native reply timed out; document outcome needs reconciliation');if(epoch!==base.authorityEpoch)error.authorityRetired=true;else error.authorityUncertain=true;reject(error);},15000);
   requests.set(id,{resolve,reject,timer});vscode.postMessage({...identity(),type,id,...fields});
  });
 }
 function accept(message){base={...base,source:message.source??base.source,revision:message.revision??base.revision,historyGeneration:message.historyGeneration??base.historyGeneration};
  if(message.snapshot){roleGeneration=message.snapshot.roleGeneration;providerGeneration=message.snapshot.providerGeneration;base.sourceHash=message.snapshot.sourceHash;}
 }
 async function acceptedHash(){return TerraphimAuthoritySnapshot.refreshHash(()=>base);}
 function serialize(operation){const epoch=base.authorityEpoch;const result=sending.then(()=>{if(epoch!==base.authorityEpoch)throw Object.assign(new Error('Authority epoch retired'),{authorityRetired:true});return operation();});sending=result.catch(()=>undefined);return result;}
 function resetTransport(){for(const item of requests.values()){clearTimeout(item.timer);item.reject(Object.assign(new Error('Authority epoch retired by explicit recovery'),{authorityRetired:true}));}requests.clear();sending=Promise.resolve();hostHistory=Promise.resolve();historyCommand=undefined;clearTimeout(stageTimer);stages.clear();}
 const adapter={
  get roleGeneration(){return roleGeneration;},get providerGeneration(){return providerGeneration;},
  reportDraft:draft,
  async transact(command,paint){return serialize(async()=>{
   if(paused)throw new Error('Native authority needs recovery');draft();const submittedDraft=draftSequence;
   let prepared,checkpoint,finished=false;const priorRevision=base.revision;
   try{
    const reply=await rpc('authority-prepare',{...command,...(historyCommand??{})});
    if(reply.noChange)return null;prepared=reply.prepared;
    if(!prepared||prepared.kind!=='prepared')throw new Error('Missing native preparation');
    const started={...prepared.result,source:prepared.presentationSource,presentationSource:prepared.presentationSource};
    const staged=await paint(started);
    checkpoint=staged&&Array.isArray(staged.history)?staged:capture(prepared.presentationSource);
    const response=await rpc('authority-finish',{token:prepared.token,afterHash:prepared.afterHash,checkpointToken:stage(checkpoint),draftSequence:submittedDraft});
    finished=true;accept(response);if(await acceptedHash()!==base)throw Object.assign(new Error('Accepted authority retired during hashing'),{authorityRetired:true});acceptedCheckpoint=JSON.stringify(checkpoint);
    if(submittedDraft===draftSequence){clearedDraft=submittedDraft;vscode.setState(undefined);receivedDraft=Math.max(receivedDraft,submittedDraft);}
    return started;
   }catch(error){
    if(error.authorityRetired||prepared&&prepared.request.authorityEpoch!==base.authorityEpoch)throw error;
    if(error.authorityUncertain){
     try{const status=await rpc('authority-status');
      if(prepared&&status.revision>priorRevision&&status.snapshot?.sourceHash===prepared.afterHash){
       accept(status);if(await acceptedHash()!==base)throw Object.assign(new Error('Accepted authority retired during hashing'),{authorityRetired:true});acceptedCheckpoint=JSON.stringify(checkpoint);
       if(submittedDraft===draftSequence){clearedDraft=submittedDraft;vscode.setState(undefined);}return {...prepared.result,source:prepared.presentationSource,presentationSource:prepared.presentationSource};
      }
      if(!prepared&&status.value?.prepared)prepared=status.value.prepared;
     }catch(statusError){if(statusError.authorityRetired||prepared&&prepared.request.authorityEpoch!==base.authorityEpoch)throw statusError;block(`Native outcome cannot be reconciled: ${String(statusError)}`);throw error;}
    }
    if(prepared&&!finished){try{const abort=await rpc('authority-abort',{token:prepared.token});if(abort.rolledBack)error.authorityRolledBack=true;else throw new Error('Native rollback missing');}
     catch(abortError){error.authorityUncertain=true;block(`Native rollback needs recovery: ${String(abortError)}`);}}
    else if(error.authorityUncertain)block(error.message);
    throw error;
   }
  });},
  historyChanged(){return serialize(async()=>{if(paused)throw new Error('History needs recovery');const checkpoint=capture();
   if(JSON.stringify(checkpoint)===acceptedCheckpoint){if(draftSequence>clearedDraft){const submittedDraft=draftSequence;await rpc('authority-clear-draft',{draftSequence:submittedDraft});if(submittedDraft===draftSequence){clearedDraft=submittedDraft;vscode.setState(undefined);}}return;}
   const submittedDraft=draftSequence;
   const reply=await rpc('authority-history',{checkpointToken:stage(checkpoint),draftSequence:submittedDraft});accept(reply);acceptedCheckpoint=JSON.stringify(checkpoint);
   if(submittedDraft===draftSequence){clearedDraft=submittedDraft;vscode.setState(undefined);}
  });},
  query(name,args=[]){return serialize(async()=>{const reply=await rpc('authority-query',{name,args});return reply.value;});},
  control(operation,args=[]){return serialize(async()=>{
   if(typeof operation==='string'){
    const [first,second]=args;operation=({kg_load_thesaurus:{kind:'role-load',json:first},kg_clear_thesaurus:{kind:'role-clear'},kg_prepare_role:{kind:'role-prepare',json:first},kg_commit_role:{kind:'role-commit',token:first},kg_commit_prepared_role:{kind:'role-commit',token:first},kg_discard_role:{kind:'role-discard',token:first},kg_discard_prepared_role:{kind:'role-discard',token:first},alternatives_provider_register:{kind:'provider-register',id:first,model:second},alternatives_provider_unregister:{kind:'provider-unregister',id:first}})[operation];
   }
   const reply=await rpc('authority-control',{operation});accept(reply);return reply.value;
  });},
 };
 async function settled(){await hostHistory;await md().authoritySettled();await sending;await adapter.historyChanged();await sending;if(paused)throw new Error('Recovery required');}
 async function command(type,format='knowledge-graph'){
  try{await settled();await serialize(()=>type==='export'?rpc('authority-export',{format}):rpc(type));
  }catch(error){if(error.authorityUncertain)block(String(error));else hostNotice.recoverable(`Operation not completed; drafts retained. ${String(error)}`);}
 }
 async function initialize(message){
  if(ready)return;
  base={protocol:1,resource:message.resource,session:message.session,authorityEpoch:message.authorityEpoch,source:message.source,revision:message.revision,historyGeneration:message.historyGeneration};
  try{
   bindings=await TeraphimEditor.loadModule({glueUrl:assets+'terraphim_editor.js',wasmUrl:assets+'terraphim_editor_bg.wasm'});
   editor=await TeraphimEditor.create(container,{bindings,assetBase:assets,allowExternalAssets:true,dirtyTitle:false,config:{hostManaged:true,fileSystemAccess:false,standalone:false,documentKey:message.resource}});
   const role=await fetch(assets+'roles/default.json');if(!role.ok)throw new Error('Bundled local role unavailable');md().loadThesaurus(await role.text());
   editor.openDocument(bindings.host_presentation_source(message.source),message.resource);
   if(message.checkpoint)TerraphimHistoryCheckpoint.restore(md().surface,message.checkpoint,bindings.host_presentation_source(message.source),md());
   if(typeof md().setAuthority!=='function')throw new Error('Packaged editor lacks the required native authority adapter; update its pinned engine assets');
   activeView=message.active;viewEpoch=message.viewEpoch;installRole(message.roleSnapshot);
   roleGeneration=message.authoritySnapshot.roleGeneration;providerGeneration=message.authoritySnapshot.providerGeneration;
   if(await acceptedHash()!==base)return;acceptedCheckpoint=JSON.stringify(capture());md().setAuthority(adapter);ready=true;
   container.addEventListener('input',event=>{if(event.isTrusted)draft();},true);
   container.addEventListener('te:ghosts-change',()=>void adapter.historyChanged());
   for(const [name,type] of [['te:save','save'],['te:open','open'],['te:markdown','export']])document.addEventListener(name,event=>{event.preventDefault();void command(type,'markdown');});
   document.addEventListener('click',event=>{const target=event.composedPath().find(node=>node instanceof Element&&node.matches('.te-files-graph [data-action="download-graph"],.te-files-graph [data-action="download-graph-text"]'));
    if(target){event.preventDefault();event.stopImmediatePropagation();void command('export',target.dataset.action==='download-graph'?'knowledge-graph':'markdown');}},true);
   container.addEventListener('keydown',event=>{if(TerraphimAuthoritySnapshot.consumeEditableSelectAll(event,container))return;if(event.defaultPrevented&&!event.isComposing)event.stopPropagation();});
   const retained=vscode.getState(),decision=TerraphimAuthoritySnapshot.retainedDraft(retained,base.resource);
   const ownSequence=message.retainedDraftSequence??0;
   if(!Number.isSafeInteger(ownSequence)||ownSequence<0){quarantine('Retained host draft sequence could not be validated',retained?.draft??JSON.stringify(retained??null));return;}
   draftSequence=Math.max(draftSequence,ownSequence);
   if(decision.kind==='retained'){
    // Preserve the exact opaque input under this resource's current identity;
    // its older source hash is evidence, never an instruction to replay it.
    draftSequence=Math.max(draftSequence,decision.sequence);
    if(draftSequence>=Number.MAX_SAFE_INTEGER){quarantine('Retained draft sequence exceeds recovery capacity',decision.draft);return;}
    const retainedEpoch=base.authorityEpoch;quarantine('Retained input is awaiting exact resource archival confirmation',decision.draft);
    const draftHash=[...new Uint8Array(await crypto.subtle.digest('SHA-256',new TextEncoder().encode(decision.draft)))].map(x=>x.toString(16).padStart(2,'0')).join('');
    if(base.authorityEpoch!==retainedEpoch)return;retainedRegistration={sequence:draftSequence+1,draftHash,acknowledged:false};
    recordDraft(decision.draft);
   }else if(decision.kind==='foreign'||decision.kind==='invalid'){
    quarantine(decision.kind==='foreign'?'A retained draft belongs to another resource and remains quarantined in this view':'Retained input could not be validated and remains quarantined in this view',typeof retained?.draft==='string'?retained.draft:JSON.stringify(retained));
   }else if(message.recoveryDrafts?.length)block('A retained pending draft is available; inspect it before choosing a recovery source');
  }catch(error){block(String(error));}
 }
 function hostChunk(message){if(base&&(message.resource!==base.resource||message.session!==base.session||message.authorityEpoch!==base.authorityEpoch||message.revision<base.revision||message.revision===base.revision&&message.historyGeneration<base.historyGeneration))return;if(typeof message.token!=='string'||typeof message.text!=='string'||message.text.length>256*1024||!Number.isSafeInteger(message.total)||message.total<1||!Number.isSafeInteger(message.index)||message.index<0||message.index>=message.total||!Number.isSafeInteger(message.bytes)||message.bytes<0||message.bytes>503*16*1024*1024)throw new Error('Invalid authority checkpoint chunk');
  if(message.index===0){clearTimeout(stageTimer);stages.clear();stages.set(message.token,{...message,parts:[],size:0});stageTimer=setTimeout(()=>stages.clear(),60000);}
  const staged=stages.get(message.token);if(!staged||staged.parts.length!==message.index||staged.total!==message.total||staged.bytes!==message.bytes||staged.resource!==message.resource||staged.authorityEpoch!==message.authorityEpoch||staged.sourceHash!==message.sourceHash||staged.session!==message.session||staged.revision!==message.revision||staged.historyGeneration!==message.historyGeneration)throw new Error('Authority checkpoint sequence mismatch');
  staged.size+=encodeSize(message.text);if(staged.size>message.bytes+2*message.total)throw new Error('Checkpoint capacity exceeded');staged.parts.push(message.text);
 }
 function attachCheckpoint(message){if(!message.checkpointToken)return;const staged=stages.get(message.checkpointToken);clearTimeout(stageTimer);stages.clear();if(!staged||staged.parts.length!==staged.total||staged.resource!==message.resource||staged.session!==message.session||staged.authorityEpoch!==message.authorityEpoch||staged.revision!==message.revision||staged.historyGeneration!==message.historyGeneration||staged.sourceHash!==message.sourceHash)throw new Error('Missing authority checkpoint');const raw=staged.parts.join('');if(encodeSize(raw)!==staged.bytes)throw new Error('Checkpoint byte count mismatch');message.checkpoint=JSON.parse(raw);}
 function installRole(value){if(!value)return;const snapshot=typeof value==='string'?JSON.parse(value):value;const local=JSON.parse(bindings.kg_role_snapshot_json());if(snapshot.thesaurusText!==local.thesaurusText){if(snapshot.loaded)bindings.kg_load_thesaurus(snapshot.thesaurusText);else bindings.kg_clear_thesaurus();md().redrawRoleIndicators();}}
 function adoptSource(source){
  const view=md();const painting=view.authorityPainting;view.authorityPainting=true;
  try{editor.openDocument(bindings.host_presentation_source(source));}finally{view.authorityPainting=painting;}
 }
 function adoptRoleMetadata(candidate){
  const prior=adopting;adopting=true;try{installRole(candidate.roleSnapshot);if(candidate.authoritySnapshot){roleGeneration=candidate.authoritySnapshot.roleGeneration;providerGeneration=candidate.authoritySnapshot.providerGeneration;}}finally{adopting=prior;}
 }
 function retainMirrorConflict(candidate){
  const retained=JSON.stringify({kind:'remote-view-conflict',previousAcceptedSource:base.source,rawDraft:JSON.parse(rawDraft())});
  const reason='Another view changed the document while this view retained input; preserve both versions before recovering';
  if(encodeSize(retained)>128*1024*1024){block('Remote-change recovery exceeds durable draft capacity',retained);return;}
  resetTransport();base={...base,source:candidate.source,sourceHash:candidate.sourceHash,revision:candidate.revision,historyGeneration:candidate.historyGeneration};recordDraft(retained);
  vscode.postMessage({...identity(),type:'external-adoption-conflict'});block(reason);
 }
 async function synchronize(message){
  // A broadcast may wait behind its own transaction's ACK. Reopening the same
  // accepted source would erase unrelated input typed since that ACK.
  if(TerraphimAuthoritySnapshot.isSameAccepted(message,base)){adoptRoleMetadata(message);return;}
  let candidate;try{candidate=await TerraphimAuthoritySnapshot.settleSnapshot(message,()=>base,settled,hasLocalDraft);}
  catch(error){if(error.mirrorDraftConflict&&error.candidate&&!paused&&TerraphimAuthoritySnapshot.isAdmissible(error.candidate,base,error.candidate.authorityEpoch)){retainMirrorConflict(error.candidate);return;}throw error;}
  if(!candidate||paused||!TerraphimAuthoritySnapshot.isAdmissible(candidate,base,candidate.authorityEpoch))return;
  if(TerraphimAuthoritySnapshot.isSameAccepted(candidate,base)){adoptRoleMetadata(candidate);return;}
  if(hasLocalDraft()){retainMirrorConflict(candidate);return;}
  adopting=true;try{
   adoptRoleMetadata(candidate);
   base={...base,...candidate};adoptSource(candidate.source);
   if(candidate.checkpoint)TerraphimHistoryCheckpoint.restore(md().surface,candidate.checkpoint,bindings.host_presentation_source(candidate.source),md());
   acceptedCheckpoint=JSON.stringify(capture());
  }finally{adopting=false;}
 }
 window.addEventListener('message',event=>{const message=event.data;if(!message||message.protocol!==1)return;
  if(base&&(message.resource!==base.resource||message.session!==base.session||message.authorityEpoch!==undefined&&message.authorityEpoch!==base.authorityEpoch&&!message.reverted||Number.isSafeInteger(message.revision)&&message.revision<base.revision||message.revision===base.revision&&Number.isSafeInteger(message.historyGeneration)&&message.historyGeneration<base.historyGeneration))return;
  if(message.type==='checkpoint-chunk'){try{hostChunk(message);}catch(error){block(String(error));}return;}
  try{attachCheckpoint(message);}catch(error){block(String(error));return;}
  if(message.type==='init'){void initialize(message);return;}
  if(message.type==='resource-conflict'&&!ready){hostNotice.blocking();notice.hidden=false;notice.textContent=message.reason;return;}
  if(!ready||message.resource!==base.resource||message.session!==base.session)return;
  if(message.type==='command-complete'||message.type==='reject'){
   const waiting=requests.get(message.id);if(waiting){clearTimeout(waiting.timer);requests.delete(message.id);if(message.type==='command-complete')waiting.resolve(message);else waiting.reject(new Error(message.reason||'Host command refused'));}return;
  }
  if(message.type==='authority-response'){
   const waiting=requests.get(message.id);if(!waiting)return;clearTimeout(waiting.timer);requests.delete(message.id);
   if(message.authorityEpoch!==base.authorityEpoch){waiting.reject(new Error('Native authority epoch changed'));return;}
   if(message.ok)waiting.resolve(message);else {const error=new Error(message.error||'Native operation refused');error.authorityUncertain=Boolean(message.authorityUncertain);error.authorityUnchanged=Boolean(message.authorityUnchanged);waiting.reject(error);}return;
  }
  if(message.type==='view-state'){activeView=message.active;viewEpoch=message.viewEpoch;return;}
  if(message.type==='authority-draft-received'){if(!Number.isSafeInteger(message.draftSequence)||message.draftSequence<0)return;receivedDraft=Math.max(receivedDraft,message.draftSequence);
   if(retainedRegistration?.sequence===message.draftSequence&&!retainedRegistration.acknowledged){const retained=retainedRegistration;retained.acknowledged=true;vscode.postMessage({...identity(),type:'retained-draft-conflict',draftSequence:retained.sequence,draftHash:retained.draftHash});}return;}
  if(message.type==='resource-conflict'){if(retainedRegistration?.acknowledged&&message.retainedDraftSequence===retainedRegistration.sequence&&message.retainedDraftHash===retainedRegistration.draftHash){retainedRegistration=undefined;quarantinedDraft=undefined;}base=TerraphimAuthoritySnapshot.rebaseConflict(message,base);block(message.reason);return;}
  if(message.type==='flush-request'){
   settling=settling.then(async()=>{try{await settled();vscode.postMessage({...identity(),type:'flush-complete',nonce:message.nonce});}
    catch{vscode.postMessage({...identity(),type:'flush-refused',nonce:message.nonce});}});return;
  }
  if(message.type==='request-history'){
   if(!activeView||viewEpoch!==message.viewEpoch||document.visibilityState==='hidden')return;
   const epoch=base.authorityEpoch,command={historyCommandNonce:message.historyCommandNonce,viewEpoch:message.viewEpoch};
   hostHistory=hostHistory.then(async()=>{if(epoch!==base.authorityEpoch)return;historyCommand=command;try{await md().altPanel?.finishLive();await md().surface[message.command]();await adapter.historyChanged();}finally{if(historyCommand===command){historyCommand=undefined;if(epoch===base.authorityEpoch&&viewEpoch===command.viewEpoch)vscode.postMessage({...identity(),type:'history-command-complete',...command});}}}).catch(error=>{if(epoch===base.authorityEpoch&&!error.authorityRetired)block(String(error));});return;
  }
  if(message.type==='request-export'){void command('export',message.format);return;}
  if(message.type==='sync'){
   if(message.revision===base.revision&&message.historyGeneration===base.historyGeneration&&!message.roleChanged)return;
   if(message.reverted&&message.externalAdoption&&hasLocalDraft()){
    const retained=JSON.stringify({kind:'external-adoption-conflict',previousAcceptedSource:base.source,rawDraft:JSON.parse(rawDraft())});
    if(encodeSize(retained)>128*1024*1024){block('External-change recovery exceeds durable draft capacity',retained);return;}
    resetTransport();base={...base,...message};recordDraft(retained);
    vscode.postMessage({...identity(),type:'external-adoption-conflict'});block('The saved file changed while this view retained input; both versions require explicit recovery');return;
   }
   if(message.reverted){adopting=true;try{resetTransport();base={...base,...message};installRole(message.roleSnapshot);if(message.authoritySnapshot){roleGeneration=message.authoritySnapshot.roleGeneration;providerGeneration=message.authoritySnapshot.providerGeneration;}adoptSource(message.source);md().surface.resetHistory();md().resetAuthorityAfterRecovery();paused=false;container.inert=false;hostNotice.recoverable(message.externalAdoption?'File updated externally with fresh Undo history.':'Recovered with fresh Undo history. Preserved pending versions remain available in the recovery archive.');acceptedCheckpoint=JSON.stringify(capture());if(quarantinedDraft===undefined)vscode.setState(undefined);else block('Retained input remains quarantined; copy it before closing this view',quarantinedDraft);}finally{adopting=false;}void acceptedHash();return;}
   const epoch=base.authorityEpoch;synchronizing=synchronizing.then(()=>synchronize(message)).catch(error=>{if(epoch===base.authorityEpoch&&!error.authorityRetired)block(String(error));});return;
  }
 });
 window.addEventListener('pagehide',()=>{clearTimeout(stageTimer);stages.clear();for(const request of requests.values()){clearTimeout(request.timer);request.reject(new Error('View disposed; received drafts retained by resource'));}requests.clear();editor?.destroy();});
 vscode.postMessage({protocol:1,type:'ready'});
})();
