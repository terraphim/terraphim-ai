const {test}=require('node:test');const assert=require('node:assert/strict');
const {AuthorityDraftStage}=require('../out/authorityDraftStage');
const identity={resource:'file:///risk.md',session:'view-a',authorityEpoch:'epoch-a'};
function packets(text,seq=1){const total=Math.ceil(text.length/(256*1024));return Array.from({length:total},(_,index)=>({...identity,token:'draft-'+seq,draftSequence:seq,index,total,bytes:Buffer.byteLength(text),text:text.slice(index*256*1024,(index+1)*256*1024)}));}
test('real chunk assembly retains UTF-8 raw draft across native revision-independent transport',()=>{
 const text='x'.repeat(256*1024-1)+'😀 concern';const stage=new AuthorityDraftStage();let result;
 for(const packet of packets(text))result=stage.receive({...packet,revision:999},identity,128*1024*1024);
 assert.deepEqual(result,{viewSession:'view-a',sequence:1,draft:text});assert.throws(()=>stage.receive(packets(text)[0],identity,128*1024*1024),/Stale/);
});
test('stale epoch cannot replace current draft and capacity refusal retains no partial result',()=>{
 const text='a'.repeat(256*1024+1),parts=packets(text),stage=new AuthorityDraftStage();assert.equal(stage.receive(parts[0],identity,128*1024*1024),undefined);
 assert.throws(()=>stage.receive({...parts[0],authorityEpoch:'retired'},identity,128*1024*1024),/Stale/);
 assert.equal(stage.receive(parts[1],identity,128*1024*1024).draft,text);
 assert.throws(()=>stage.receive(packets('new',2)[0],identity,1),/capacity/);
});
