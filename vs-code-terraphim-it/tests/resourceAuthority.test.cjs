// Actual Node workers and real Rust/WASM. Timing faults drop real replies, never domain results.
const {test,before,after}=require('node:test');
const assert=require('node:assert/strict');
const fs=require('node:fs/promises');const path=require('node:path');const os=require('node:os');
const {createHash}=require('node:crypto');
const {ResourceAuthority}=require('../out/resourceAuthority.js');
const hash=value=>createHash('sha256').update(value).digest('hex');
const root=path.resolve(__dirname,'..');let fixture;const owned=[];let dispatch=0;
const role=JSON.stringify({name:'Worker fixture',data:{risk:{id:1,nterm:'risk',display_value:'Risk',pinned:false}}});
before(async()=>{
 fixture=await fs.mkdtemp(path.join(os.tmpdir(),'terraphim-authority-real-'));
 await fs.mkdir(path.join(fixture,'media','editor','roles'),{recursive:true});
 await fs.copyFile(path.join(root,'media','resource-authority-worker.cjs'),path.join(fixture,'media','resource-authority-worker.cjs'));
 const wasm=process.env.TERRAPHIM_AUTHORITY_WASM_DIR||path.join(root,'media','editor');const assets={};
 for(const name of ['terraphim_editor.js','terraphim_editor_bg.wasm']){const bytes=await fs.readFile(path.join(wasm,name));assets[name]=hash(bytes);await fs.writeFile(path.join(fixture,'media','editor',name),bytes);}
 const bytes=await fs.readFile(path.join(root,'media','editor','roles','default.json'));assets['roles/default.json']=hash(bytes);await fs.writeFile(path.join(fixture,'media','editor','roles','default.json'),bytes);
 await fs.writeFile(path.join(fixture,'media','editor-build.json'),JSON.stringify({protocol:1,assets}));
});
after(async()=>{for(const a of owned)await a.dispose();if(fixture)await fs.rm(fixture,{recursive:true,force:true});});
async function authority(source='Risk needs a decision.\n',suffix=String(owned.length)){
 const a=await ResourceAuthority.create({extensionPath:fixture,resource:'file:///authority-'+suffix+'.md',authorityEpoch:'epoch-'+suffix,revision:1,source,historyGeneration:0,roleJson:role});owned.push(a);await a.attachView('view-a');await a.attachView('view-b');return a;
}
function base(s){return {resource:s.resource,authorityEpoch:s.authorityEpoch,revision:s.revision,sourceHash:s.sourceHash,historyGeneration:s.historyGeneration};}
async function request(a,id=1,viewSession='view-a'){return {...base(await a.snapshot()),protocol:1,viewSession,requestId:id,dispatchId:++dispatch};}
async function commit(a,p){assert.equal(p.kind,'prepared');const before=await a.snapshot();const terminal=await a.finish(p.request,p.token,p.afterHash);assert.equal(terminal.kind,'committed');
 assert.deepEqual(await a.finish(p.request,p.token,p.afterHash),terminal,'finish retry uses recorded terminal outcome');
 assert.equal((await a.status(p.request)).state,'committed-unreconciled');assert.equal((await a.snapshot()).source,before.source,'tentative source never becomes accepted before reconciliation');
 const next={...base(before),sourceHash:p.afterHash,revision:before.revision+1,historyGeneration:before.historyGeneration+1};await a.ackOutcome(p.request,terminal,next);return a.snapshot();}
async function abort(a,p){const before=await a.snapshot();const terminal=await a.abort(p.request,p.token);assert.equal(terminal.kind,'guarded-aborted');assert.deepEqual(await a.abort(p.request,p.token),terminal);await a.ackOutcome(p.request,terminal,base(before));return a.snapshot();}
async function text(a,id,text='Concern',view='view-a'){
 const s=await a.snapshot();const before=s.projection.body;const after=text+before.slice(4);return a.prepareAction(await request(a,id,view),{expectedBody:before,meta:{kind:'text_edit',start:0,deleteUtf16:4,insertedRef:0,beforeBodyHash:hash(before),afterBodyHash:hash(after)},textArgs:[text]});
}
test('real worker commits once, keeps terminal journal and rejects stale/foreign identities',async()=>{
 const a=await authority();const initial=await a.snapshot();const p=await text(a,1);
 await assert.rejects(a.finish({...p.request,authorityEpoch:'foreign'},p.token,p.afterHash),/Foreign|retired/);
 await assert.rejects(a.finish(p.request,p.token,'0'.repeat(64)),/hash/);
 const accepted=await commit(a,p);assert.equal(accepted.projection.body,'Concern needs a decision.\n');assert.equal(accepted.revision,2);
 await assert.rejects(a.prepareAction(p.request,{expectedBody:initial.projection.body,meta:{kind:'overflow_set',textRef:0},textArgs:['stale']}),/Stale/);
 assert.equal((await a.snapshot()).source,accepted.source);
});
test('real guarded abort restores source and consumes rejected IDs without cancelling pending work',async()=>{
 const a=await authority();const initial=await a.snapshot();const p=await text(a,1);
 await assert.rejects(text(a,2),/reconciliation/);
 assert.equal((await a.status(p.request)).state,'pending');await abort(a,p);assert.equal((await a.snapshot()).source,initial.source);
 await assert.rejects(text(a,2),/Stale/);const later=await text(a,3);await abort(a,later);
});
test('real protected receipt survives issuing-view disposal and second-view Undo/Redo in same worker',async()=>{
 const source='Risk remains. Perhaps we might basically need to possibly review the very complex arrangement because the supplier could in some ways delay the launch.\n';
 const a=await authority(source);await a.query('trim_plan_json');const p=await a.prepareTrim(await request(a),{expectedBody:source,level:'sharper',kept:[]});
 assert.ok(p.result.change,'actual native receipt');const promoted=await commit(a,p);assert.ok(promoted.projection.annotations.ghosts.length>0);
 await a.detachView('view-a');const replay={expectedBody:source,targetBody:source,operationsJson:JSON.stringify([{kind:'ghost',body:source,change:p.result.change,undo:true}])};
 const undo=await a.prepareHistory(await request(a,1,'view-b'),replay);const undone=await commit(a,undo);assert.equal(undone.projection.annotations.ghosts.length,0);
 const redo=await a.prepareHistory(await request(a,2,'view-b'),{...replay,operationsJson:JSON.stringify([{kind:'ghost',body:source,change:p.result.change,undo:false}])});assert.equal((await commit(a,redo)).source,promoted.source);
 const other=await authority(source);await assert.rejects(other.prepareHistory(await request(other),replay),/unknown or stale|receipt/);assert.equal((await other.snapshot()).source,source);
});
test('role changes invalidate pending promotion and guarded abort retains the new actual role',async()=>{
 const source='Risk remains. Perhaps we might basically need to possibly review the arrangement because the supplier could delay the launch.\n';const a=await authority(source);await a.query('trim_plan_json');
 const before=await a.snapshot();const p=await a.prepareTrim(await request(a),{expectedBody:source,level:'sharper',kept:[]});assert.equal(p.kind,'prepared');
 await a.control({kind:'role-load',json:JSON.stringify({name:'Changed',data:{launch:{id:2,nterm:'launch'}}})});
 const failed=await a.finish(p.request,p.token,p.afterHash);assert.equal(failed.kind,'refused-with-pending-token');const restored=await abort(a,p);assert.equal(restored.source,before.source);assert.notEqual(restored.roleGeneration,before.roleGeneration);
});
test('provider generation changes cannot finalize stale actual supplied alternatives',async()=>{
 const a=await authority();await a.control({kind:'provider-register',id:'fixture',model:'local-fixture'});const s=await a.snapshot();
 const p=await a.prepareAction(await request(a),{expectedBody:s.projection.body,meta:{kind:'append_alternatives',start:0,end:4,suppliedJsonRef:0,roleGeneration:s.roleGeneration,providerGeneration:s.providerGeneration},textArgs:[JSON.stringify({fixture:['Concern']})]});assert.equal(p.kind,'prepared');
 await a.control({kind:'provider-unregister',id:'fixture'});assert.equal((await a.finish(p.request,p.token,p.afterHash)).kind,'refused-with-pending-token');assert.equal((await abort(a,p)).source,s.source);
});
test('lost actual finish reply reconciles recorded native commit instead of repeating finish',async()=>{
 const a=await authority();const p=await text(a,1);const worker=a.worker;const listeners=worker.listeners('message');worker.removeAllListeners('message');a.options.timeoutMs=80;
 await assert.rejects(a.finish(p.request,p.token,p.afterHash),/timed out/);for(const listener of listeners)worker.on('message',listener);a.options.timeoutMs=15000;
 const status=await a.status(p.request);assert.equal(status.state,'committed-unreconciled');assert.equal(status.outcome.kind,'committed');
 const before=await a.snapshot();await a.ackOutcome(p.request,status.outcome,{...base(before),sourceHash:p.afterHash,revision:before.revision+1,historyGeneration:before.historyGeneration+1});assert.equal((await a.snapshot()).source,p.source);
});
test('actual worker context death reports failure and never automatically restarts another resource',async()=>{
 const a=await authority();const b=await authority();const stable=await b.snapshot();const failed=new Promise(resolve=>a.onFailure(resolve));await a.worker.terminate();assert.match((await failed).message,/context exited/);
 await assert.rejects(a.snapshot(),/context exited/);assert.deepEqual(await b.snapshot(),stable);await commit(b,await text(b,1));
});
test('history-only generation update leaves native authority and source intact',async()=>{
 const a=await authority();const initial=await a.snapshot();const next=await a.updateHistory(base(initial),1);assert.equal(next.source,initial.source);assert.equal(next.revision,initial.revision);assert.equal(next.historyGeneration,1);
 await assert.rejects(a.updateHistory(base(initial),2),/Stale/);await abort(a,await text(a,1));
});
test('lost actual reconciliation ACK reply is idempotent only for the same accepted identity',async()=>{
 const a=await authority();const p=await text(a,1);const prior=await a.snapshot();const terminal=await a.finish(p.request,p.token,p.afterHash);
 const next={...base(prior),sourceHash:p.afterHash,revision:prior.revision+1,historyGeneration:prior.historyGeneration+1};
 const worker=a.worker,listeners=worker.listeners('message');worker.removeAllListeners('message');a.options.timeoutMs=80;
 await assert.rejects(a.ackOutcome(p.request,terminal,next),/timed out/);for(const listener of listeners)worker.on('message',listener);a.options.timeoutMs=15000;
 const status=await a.status(p.request);assert.equal(status.state,'idle');assert.equal(status.accepted.source,p.source);
 await a.ackOutcome(p.request,terminal,next);assert.equal((await a.snapshot()).revision,next.revision);
 await assert.rejects(a.ackOutcome(p.request,terminal,{...next,revision:next.revision+1}),/mismatch/);
 const later=await text(a,2);await assert.rejects(a.ackOutcome(p.request,terminal,next),/mismatch/);assert.equal((await a.status(later.request)).state,'pending');await abort(a,later);
});

test('oversized input is refused before worker dispatch and does not consume native identities',async()=>{
 const a=await authority();const initial=await a.snapshot(),identity=await request(a);const tooLarge='x'.repeat(16*1024*1024+1),before=a.sequence;
 await assert.rejects(a.prepareAction(identity,{expectedBody:initial.projection.body,meta:{kind:'overflow_set',textRef:0},textArgs:[tooLarge]}),/limit/);
 await assert.rejects(a.prepareHistory(identity,{expectedBody:tooLarge,targetBody:initial.projection.body,operationsJson:'[]'}),/limit/);
 await assert.rejects(a.query('trim_status',[tooLarge,'[]']),/limit/);
 await assert.rejects(a.control({kind:'role-load',json:tooLarge}),/limit/);
 assert.equal(a.sequence,before,'rejected input never posted or structured-cloned to worker');
 const prepared=await a.prepareAction(identity,{expectedBody:initial.projection.body,meta:{kind:'overflow_set',textRef:0},textArgs:['Accepted after refusals']});
 assert.equal(prepared.kind,'prepared');await abort(a,prepared);assert.equal((await a.snapshot()).source,initial.source);
});

test('idle role control refreshes actual accepted KG projection without replacing the document session',async()=>{
 const a=await authority();const before=await a.snapshot();
 await a.control({kind:'role-load',json:JSON.stringify({name:'New role',data:{risk:{id:8,nterm:'risk',display_value:'Risk'},concern:{id:8,nterm:'concern',display_value:'Concern'}}})});
 const controlled=await a.snapshot();assert.notDeepEqual(controlled.projection.annotations.kg,before.projection.annotations.kg,'fixture actually changes matched KG concepts');assert.equal(controlled.revision,before.revision);assert.equal(controlled.historyGeneration,before.historyGeneration);assert.equal(controlled.source,before.source);
 const p=await a.prepareAction(await request(a),{expectedBody:controlled.projection.body,meta:{kind:'overflow_set',textRef:0},textArgs:['Projection probe']});
 assert.deepEqual(controlled.projection.annotations.kg,p.projection.annotations.kg,'idle snapshot reflects native current role just like subsequent actual action');await abort(a,p);
});

test('real generated module with any missing mandatory capability refuses readiness before native open',async t=>{
 const names=['open_document','save_document','host_presentation_source','host_persistence_source',
  'document_body','document_annotations','document_counts','document_overflow','document_synonyms','export_document','export_knowledge_graph',
  'begin_document_action','begin_document_history_replay','begin_trim_ghost_transaction','finish_document_history_transaction','abort_document_history_transaction',
  'kg_load_thesaurus','kg_clear_thesaurus','kg_prepare_role','kg_commit_prepared_role','kg_discard_prepared_role','kg_role_snapshot_json',
  'alternatives_provider_register','alternatives_provider_unregister','alternatives_provider_ids','trim_plan_json','trim_status','trim_make_cuts'];
 for(const name of names)await t.test(name,async()=>{
  const incomplete=await fs.mkdtemp(path.join(os.tmpdir(),'terraphim-authority-missing-'));
  try{
   await fs.cp(path.join(fixture,'media'),path.join(incomplete,'media'),{recursive:true});
   const gluePath=path.join(incomplete,'media','editor','terraphim_editor.js');let glue=await fs.readFile(gluePath,'utf8');
   const declaration='export function '+name+'(';assert.ok(glue.includes(declaration),'actual generated glue must expose the tested capability');
   glue=glue.replace(declaration,'function '+name+'(');await fs.writeFile(gluePath,glue);
   const manifestPath=path.join(incomplete,'media','editor-build.json'),manifest=JSON.parse(await fs.readFile(manifestPath,'utf8'));
   manifest.assets['terraphim_editor.js']=hash(glue);await fs.writeFile(manifestPath,JSON.stringify(manifest));
   await assert.rejects(ResourceAuthority.create({extensionPath:incomplete,resource:'file:///missing-capability.md',authorityEpoch:'missing-'+name,revision:1,historyGeneration:0,source:'Risk remains.\n'}),new RegExp('Native authority capability missing: '+name));
  }finally{await fs.rm(incomplete,{recursive:true,force:true});}
 });
});
test('actual worker exports authored synonyms after native commit and saved-source reopen',async()=>{
 const a=await authority();const initial=await a.snapshot();
 const p=await a.prepareAction(await request(a),{expectedBody:initial.projection.body,meta:{kind:'alt_create',spanKind:'word',start:0,end:4,textRef:0,source:'human'},textArgs:['Concern']});
 await assert.rejects(a.query('export_knowledge_graph'),/reconciliation/);
 const saved=await commit(a,p);const raw=await a.query('export_knowledge_graph');assert.equal(typeof raw,'string');
 const graph=JSON.parse(raw);assert.ok(graph.data.risk&&graph.data.concern);assert.equal(graph.data.risk.id,graph.data.concern.id);
 const reopened=await authority(saved.source);const reopenedGraph=JSON.parse(await reopened.query('export_knowledge_graph'));
 assert.equal(reopenedGraph.data.risk.id,reopenedGraph.data.concern.id);assert.deepEqual(reopenedGraph,graph);
 assert.equal(await reopened.query('export_document'),'Risk needs a decision.\n');
});

test('Save flush rejects mixed actual same-body Ghost snapshots and resynchronizes before acceptance',async()=>{
 const {flushStableSnapshots}=require('../out/flushSnapshots');
 const source='Risk remains. Perhaps we might basically need to possibly review the very complex arrangement because the supplier could in some ways delay the launch.\n';
 const a=await authority(source);await a.query('trim_plan_json');const initial=await a.snapshot();let accepted=initial,rounds=0,synchronizations=0;
 const promotion=await a.prepareTrim(await request(a),{expectedBody:source,level:'sharper',kept:[]});
 await flushStableSnapshots(()=>base(accepted),async()=>{
  rounds++;
  if(rounds===1){accepted=await commit(a,promotion);assert.equal(accepted.projection.body,initial.projection.body);assert.notEqual(accepted.sourceHash,initial.sourceHash);return {snapshots:[base(initial),base(accepted)],participantsUnchanged:true};}
  return {snapshots:[base(await a.snapshot()),base(await a.snapshot())],participantsUnchanged:true};
 },async()=>{synchronizations++;assert.equal((await a.snapshot()).source,accepted.source);});
 assert.equal(rounds,2);assert.equal(synchronizations,1);assert.ok(accepted.projection.annotations.ghosts.length);
 const undo=await a.prepareHistory(await request(a,2,'view-b'),{expectedBody:source,targetBody:source,operationsJson:JSON.stringify([{kind:'ghost',body:source,change:promotion.result.change,undo:true}])});
 assert.equal((await commit(a,undo)).source,initial.source,'the exact original native receipt remains valid');
});

test('Save flush remains bounded under actual continuing accepted mutations and propagates failure',async()=>{
 const {flushStableSnapshots}=require('../out/flushSnapshots');const a=await authority();let accepted=await a.snapshot(),rounds=0,synchronizations=0;
 await assert.rejects(flushStableSnapshots(()=>base(accepted),async()=>{
  const previous=accepted;rounds++;
  const action=await a.prepareAction(await request(a,rounds),{expectedBody:accepted.projection.body,meta:{kind:'overflow_set',textRef:0},textArgs:['Unsaved note '+rounds]});
  accepted=await commit(a,action);return {snapshots:[base(previous),base(accepted)],participantsUnchanged:true};
 },async()=>{synchronizations++;await a.snapshot();}),/kept changing; drafts retained/);
 assert.equal(rounds,3);assert.equal(synchronizations,2);assert.equal((await a.snapshot()).projection.overflow,'Unsaved note 3');
 await a.worker.terminate();await assert.rejects(flushStableSnapshots(()=>base(accepted),async()=>({snapshots:[base(await a.snapshot())],participantsUnchanged:true}),async()=>{throw Error('Must not retry worker failure');}),/context exited/);
});
test('delayed mirror snapshot cannot rewind an actual committed worker or replace a fresh epoch',async()=>{
 const {settleSnapshot,isAdmissible}=require('../media/authority-snapshot.js');
 const a=await authority();let accepted={...await a.snapshot(),session:'view-a'};
 let release;const gate=new Promise(resolve=>{release=resolve;});
 const delayed=settleSnapshot({...accepted},()=>accepted,()=>gate);
 accepted={...await commit(a,await text(a,1)),session:'view-a'};release();
 assert.equal(await delayed,null);assert.equal(accepted.source,(await a.snapshot()).source);
 const b=await authority('Independent saved source.\n');
 let releaseEpoch;const epochGate=new Promise(resolve=>{releaseEpoch=resolve;});
 const old=settleSnapshot({...accepted},()=>accepted,()=>epochGate);
 accepted={...await b.snapshot(),session:'view-a'};releaseEpoch();assert.equal(await old,null);
 const valid=await settleSnapshot({...accepted},()=>accepted,async()=>{});assert.equal(valid.source,accepted.source);
 accepted={...await commit(b,await text(b,1,'Changed')),session:'view-a'};assert.equal(isAdmissible(valid,accepted,valid.authorityEpoch),false,'caller rechecks at the synchronous adoption edge');
 await assert.rejects(settleSnapshot({...accepted,sourceHash:'0'.repeat(64)},()=>({...accepted,sourceHash:'0'.repeat(64)}),async()=>{}),/source hash/);
});
test('asynchronous hash continuation never assigns an old digest to another accepted worker snapshot',async()=>{
 const {refreshHash}=require('../media/authority-snapshot.js');const a=await authority();let accepted={...await a.snapshot()};
 const original=accepted;const pending=refreshHash(()=>accepted);
 accepted={...original,authorityEpoch:'fresh-epoch',source:'Fresh saved source.',sourceHash:hash('Fresh saved source.')};
 assert.equal(await pending,false);assert.equal(accepted.sourceHash,hash('Fresh saved source.'));
 assert.equal(await refreshHash(()=>accepted),accepted);assert.equal(accepted.sourceHash,hash(accepted.source));
});
test('a retained conflict refreshes only its guarded transport tuple after another actual view commits',async()=>{
 const {rebaseConflict}=require('../media/authority-snapshot.js');const a=await authority();
 const prior={...await a.snapshot(),session:'view-a'};const visibleProjection=prior.projection;
 const current={...await commit(a,await text(a,1,'Concern','view-b')),session:'view-a'};
 const updated=rebaseConflict({...current,externalAdoptionConflict:true},prior);
 assert.equal(updated.source,current.source);assert.equal(updated.sourceHash,current.sourceHash);
 assert.equal(updated.projection,visibleProjection,'transport refresh cannot replace the retained view projection');
 assert.equal(rebaseConflict({...current,session:'foreign',externalAdoptionConflict:true},prior),prior);
 assert.equal(rebaseConflict({...current,authorityEpoch:'retired',externalAdoptionConflict:true},prior),prior);
 assert.equal(rebaseConflict({...prior,externalAdoptionConflict:true},updated),updated);
 assert.equal(rebaseConflict(current,prior),prior,'ordinary conflict message cannot take this recovery-specific path');
});
test('raw Overflow input arriving during a real mirror digest prevents remote projection replacement',async()=>{
 const {settleSnapshot}=require('../media/authority-snapshot.js');const a=await authority();
 let accepted={...await a.snapshot(),session:'view-a'};const candidate={...await commit(a,await text(a,1,'Concern','view-b')),session:'view-a'};
 let rawOverflow='',reads=0;
 const current=()=>{reads++;if(reads===2)queueMicrotask(()=>{rawOverflow='Public rapid Overflow draft, not yet accepted';});return accepted;};
 // The current getter always returns a real worker snapshot. Only the arrival
 // timing of independent raw user input is controlled; crypto and native results are real.
 await assert.rejects(settleSnapshot(candidate,current,async()=>{await a.snapshot();},()=>rawOverflow!==''),/draft/i);
 assert.equal(rawOverflow,'Public rapid Overflow draft, not yet accepted');assert.equal(accepted.projection.overflow,'');
 assert.equal((await a.snapshot()).source,candidate.source,'mirror refusal cannot undo the other real accepted view');
});
test('a queued own broadcast becomes redundant after its actual native ACK without consuming newer raw input',async()=>{
 const {settleSnapshot,isSameAccepted}=require('../media/authority-snapshot.js');const a=await authority();
 let accepted={...await a.snapshot(),session:'view-a'};
 const candidate={...await commit(a,await text(a,1,'Concern')),session:'view-a'};
 assert.equal(isSameAccepted(candidate,accepted),false);
 const rawOverflow='Newer raw input after this actual ACK';
 const result=await settleSnapshot(candidate,()=>accepted,async()=>{accepted={...await a.snapshot(),session:'view-a'};},()=>true);
 assert.equal(isSameAccepted(result,accepted),true,'caller can return without reopening an already accepted projection');
 assert.equal(rawOverflow,'Newer raw input after this actual ACK');assert.equal((await a.snapshot()).projection.overflow,'');
 assert.equal(isSameAccepted({...candidate,session:'another-view'},accepted),false);
 assert.equal(isSameAccepted({...candidate,source:candidate.source+'!'},accepted),false);
});
