const {test}=require('node:test');
const assert=require('node:assert/strict');
const {validateCheckpoint}=require('../media/history-checkpoint.js');
const source='Risk';
const checkpoint={schema:1,presentationSource:source,history:[{text:'Risk',start:0,end:4,source:'init',time:0,edits:null}],historyIndex:0,historyLimit:500};
test('checkpoint transport preserves exact existing entries without interpreting a second history',()=>{
 const actual=validateCheckpoint(checkpoint,source,'Risk');
 assert.deepEqual(actual,checkpoint); assert.notEqual(actual.history,checkpoint.history);
});
test('checkpoint refuses source/body mismatch, invalid index, oversized JSON and executable values',()=>{
 assert.throws(()=>validateCheckpoint(checkpoint,'Concern','Concern'),/source/);
 assert.throws(()=>validateCheckpoint({...checkpoint,historyIndex:1},source,'Risk'),/index/);
 assert.throws(()=>validateCheckpoint({...checkpoint,history:[{...checkpoint.history[0],text:'Concern'}]},source,'Risk'),/body/);
 assert.throws(()=>validateCheckpoint({...checkpoint,historyLimit:Infinity},source,'Risk'),/limit/);
 assert.throws(()=>validateCheckpoint({...checkpoint,presentationSource:'x'.repeat(16*1024*1024)},source,'Risk'),/limit/);
 assert.throws(()=>validateCheckpoint({...checkpoint,execute:()=>{}},source,'Risk'),/JSON/);
});
test('checkpoint transports native text Ghost references without accepting snapshots or conflicting kinds',()=>{
 const entry={text:'Risk-long',start:9,end:9,source:'typing',time:1,edits:[{start:1,deletedText:'isk',insertedText:'isk-long',ghostEdit:{receipt:'text-ghost-19',undo:false}}]};
 const value={...checkpoint,presentationSource:'Risk-long',history:[checkpoint.history[0],entry],historyIndex:1};
 const actual=validateCheckpoint(value,'Risk-long','Risk-long');
 assert.deepEqual(actual.history[1].edits[0].ghostEdit,entry.edits[0].ghostEdit);
 assert.notEqual(actual.history[1].edits[0].ghostEdit,entry.edits[0].ghostEdit);
 for(const meta of [{receipt:'text-ghost-19'},{receipt:'text-ghost-19',undo:0},{receipt:'',undo:false},{receipt:'text-ghost-19',undo:false,snapshot:{}}]){
  const invalid=structuredClone(value);invalid.history[1].edits[0].ghostEdit=meta;
  assert.throws(()=>validateCheckpoint(invalid,'Risk-long','Risk-long'),/metadata|receipt/);
 }
 const conflicting=structuredClone(value);conflicting.history[1].edits[0].move={start:0,end:1,to:3};
 assert.throws(()=>validateCheckpoint(conflicting,'Risk-long','Risk-long'),/step/);
});
