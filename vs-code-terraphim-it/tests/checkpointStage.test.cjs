const {test}=require('node:test');
const assert=require('node:assert/strict');
const fs=require('node:fs/promises');const path=require('node:path');
const {CheckpointStage}=require('../out/checkpointStage');
const {chunks}=require('../media/history-checkpoint');
let bindings;
async function setup(timeout=60000) {
 if(!bindings){const glue=await fs.readFile(path.join(__dirname,'../media/editor/terraphim_editor.js'));
  bindings=await import('data:text/javascript;base64,'+glue.toString('base64'));
  await bindings.default({module_or_path:await fs.readFile(path.join(__dirname,'../media/editor/terraphim_editor_bg.wasm'))});}
 const source=await fs.readFile(path.join(__dirname,'fixtures/native-three-alternatives.md'),'utf8');
 const engine=new bindings.TerraphimHostSession('file:///actual.md','actual',1,source);
 const state=JSON.parse(engine.snapshot());engine.free();
 const snapshot={resource:state.resource,session:state.session,revision:state.revision,sourceHash:state.sourceHash,historyGeneration:3};
 const presentationSource=bindings.host_presentation_source(source);
 const checkpoint={schema:1,presentationSource,history:[{text:state.projection.body,start:0,end:0,source:'init',time:0,edits:null}],historyIndex:0,historyLimit:500};
 const encoded=chunks(checkpoint),token='issued-stage',messages=encoded.parts.map((text,index)=>({protocol:1,type:'checkpoint-chunk',...snapshot,token,index,total:encoded.parts.length,bytes:encoded.bytes,text}));
 return {stage:new CheckpointStage(timeout),snapshot,checkpoint,messages};
}
test('actual staged checkpoint consumes once at exact real WASM resource/source/history CAS',async()=>{
 const {stage,snapshot,checkpoint,messages}=await setup();for(const message of messages)stage.receive(message,snapshot,64*1024*1024);
 assert.deepEqual(stage.consume('issued-stage',snapshot),checkpoint);assert.throws(()=>stage.consume('issued-stage',snapshot),/Missing/);
});
test('stale actor, source and generation cannot stage or consume data',async()=>{
 for(const key of ['resource','session','revision','sourceHash','historyGeneration']){
  const {stage,snapshot,messages}=await setup();const changed={...snapshot,[key]:typeof snapshot[key]==='number'?snapshot[key]+1:'foreign'};
  assert.throws(()=>stage.receive(messages[0],changed,64*1024*1024),/Stale/);stage.clear();
  for(const message of messages)stage.receive(message,snapshot,64*1024*1024);
  assert.throws(()=>stage.consume('issued-stage',changed),/stale/);stage.clear();
 }
});
test('sequence, capacity, byte corruption and disposal refuse without accepted-source mutation',async()=>{
 for(const mutate of [m=>({...m,index:1}),m=>({...m,bytes:m.bytes+1}),m=>({...m,text:m.text+'x'})]){
  const {stage,snapshot,messages}=await setup();const before=structuredClone(snapshot);
  assert.throws(()=>{stage.receive(mutate(messages[0]),snapshot,64*1024*1024);stage.consume('issued-stage',snapshot);});
  assert.deepEqual(snapshot,before);stage.clear();
 }
 const {stage,snapshot,messages}=await setup();assert.throws(()=>stage.receive(messages[0],snapshot,1),/capacity/);
 for(const message of messages)stage.receive(message,snapshot,64*1024*1024);stage.clear();assert.throws(()=>stage.consume('issued-stage',snapshot),/Missing/);
});
test('real timer expires unaccepted staged data and permits a fresh distinct stage',async()=>{
 const {stage,snapshot,messages}=await setup(5);for(const message of messages)stage.receive(message,snapshot,64*1024*1024);
 await new Promise(resolve=>setTimeout(resolve,15));assert.throws(()=>stage.consume('issued-stage',snapshot),/Missing/);
 for(const message of messages)stage.receive({...message,token:'fresh'},snapshot,64*1024*1024);assert.ok(stage.consume('fresh',snapshot));
});
test('native authority epoch refuses stale chunks without retiring current staged checkpoint',async()=>{
 const {stage,snapshot,checkpoint,messages}=await setup();const current={...snapshot,authorityEpoch:'current-epoch'};
 for(const message of messages)stage.receive({...message,authorityEpoch:'current-epoch'},current,64*1024*1024);
 assert.throws(()=>stage.receive({...messages[0],authorityEpoch:'retired-epoch'},current,64*1024*1024),/Stale/);
 assert.deepEqual(stage.consume('issued-stage',current),checkpoint);
});
