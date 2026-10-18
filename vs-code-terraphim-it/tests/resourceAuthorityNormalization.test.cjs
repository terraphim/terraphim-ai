// Actual Rust/WASM workers: normalization must retain strict ready-tuple admission.
'use strict';
const {test}=require('node:test');
const assert=require('node:assert/strict');
const path=require('node:path');
const {createHash}=require('node:crypto');
const {ResourceAuthority}=require('../out/resourceAuthority.js');
const hash=s=>createHash('sha256').update(s).digest('hex');
const base=s=>({resource:s.resource,authorityEpoch:s.authorityEpoch,revision:s.revision,sourceHash:s.sourceHash,historyGeneration:s.historyGeneration});
async function commit(a,p){
 const old=await a.snapshot();const outcome=await a.finish(p.request,p.token,p.afterHash);assert.equal(outcome.kind,'committed');
 await a.ackOutcome(p.request,outcome,{...base(old),revision:old.revision+1,historyGeneration:old.historyGeneration+1,sourceHash:p.afterHash});return a.snapshot();
}
test('real normalized annotation context requires ready canonical tuple, never raw caller hash',async()=>{
 const authorities=[];const root=path.resolve(__dirname,'..');const roleJson=JSON.stringify({name:'Normalization fixture',data:{risk:{id:1,nterm:'risk',display_value:'Risk',pinned:false}}});
 async function open(source,epoch){const a=await ResourceAuthority.create({extensionPath:root,resource:'file:///normalization.md',authorityEpoch:epoch,revision:1,historyGeneration:0,source,roleJson});authorities.push(a);await a.attachView('view');return a;}
 try{
  const original=await open('Risk remains.\n','original');const initial=await original.snapshot();
  const authored=await original.prepareAction({...base(initial),protocol:1,viewSession:'view',requestId:1,dispatchId:1},{expectedBody:initial.projection.body,meta:{kind:'alt_create',spanKind:'word',start:0,end:4,textRef:0,source:'human'},textArgs:['Concern']});
  assert.equal(authored.kind,'prepared');const canonical=await commit(original,authored);assert.ok(canonical.projection.annotations.spans[0].alts.some(x=>x.text==='Concern'));
  const raw=canonical.source.replace('\n\n```terraphim-alternatives','\nEXTERNAL-ADDED\n\n```terraphim-alternatives');assert.notEqual(raw,canonical.source,'real external body input changes the persisted context');
  const reopened=await open(raw,'reopened');const ready=await reopened.snapshot();
  assert.notEqual(ready.source,raw,'real Rust refreshes stored anchor context on opening external source');assert.equal(ready.sourceHash,hash(ready.source));assert.notEqual(ready.sourceHash,hash(raw));assert.ok(ready.projection.body.includes('EXTERNAL-ADDED'));
  const operation={expectedBody:ready.projection.body,meta:{kind:'overflow_set',textRef:0},textArgs:['Public retained note']};
  await assert.rejects(reopened.prepareAction({...base(ready),sourceHash:hash(raw),protocol:1,viewSession:'view',requestId:1,dispatchId:1},operation),/Stale authority sourceHash/);
  assert.equal((await reopened.snapshot()).source,ready.source,'refused raw tuple cannot mutate actual native source');
  const prepared=await reopened.prepareAction({...base(ready),protocol:1,viewSession:'view',requestId:2,dispatchId:2},operation);assert.equal(prepared.kind,'prepared');const accepted=await commit(reopened,prepared);
  assert.equal(accepted.projection.overflow,'Public retained note');assert.equal(accepted.projection.body,ready.projection.body);
 }finally{await Promise.all(authorities.map(a=>a.dispose()));}
});
