'use strict';
const {test}=require('node:test'),assert=require('node:assert/strict');
const {retainedDraft}=require('../media/authority-snapshot.js');
const input=require('./fixtures/retained-recovery-public-input.json');
test('actual captured substantive and stale envelopes are retained byte-exact, never inferred empty',()=>{
 for(const draft of [input.substantive,input.staleMetadata])assert.deepEqual(retainedDraft({resource:'file:///actual.md',sequence:132,draft},'file:///actual.md'),{kind:'retained',draft,sequence:132});
 const clearing=JSON.stringify({baseSourceHash:'a'.repeat(64),pending:{sequence:0,intents:[]},alternatives:[],overflow:''});assert.equal(retainedDraft({resource:'file:///actual.md',sequence:1,draft:clearing},'file:///actual.md').kind,'retained','empty overflow can be a substantive clear');
});
test('foreign or malformed persisted input is quarantined without mutating or clearing it',()=>{
 for(const state of [{resource:'file:///other.md',sequence:1,draft:input.substantive},{resource:'file:///actual.md',sequence:1,draft:'not JSON'},{resource:'file:///actual.md',sequence:1,draft:JSON.stringify({baseSourceHash:'a'.repeat(64),unknown:'Preserve unknown input'})},{resource:'file:///actual.md',sequence:-1,draft:input.substantive}]){const before=JSON.stringify(state);assert.ok(['foreign','invalid'].includes(retainedDraft(state,'file:///actual.md').kind));assert.equal(JSON.stringify(state),before);}
 assert.deepEqual(retainedDraft(undefined,'file:///actual.md'),{kind:'none'});
});

test('retained sequence highwater is exact and unsafe counters remain quarantined',()=>{
 const state={resource:'file:///actual.md',sequence:Number.MAX_SAFE_INTEGER,draft:input.substantive};assert.equal(retainedDraft(state,state.resource).sequence,Number.MAX_SAFE_INTEGER);assert.equal(retainedDraft({...state,sequence:Number.MAX_SAFE_INTEGER+1},state.resource).kind,'invalid');
});
