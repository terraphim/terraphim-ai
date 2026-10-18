'use strict';
// Production RPC function, real timers, only the external VS Code messaging boundary is supplied.
// This checks transport deadlines, not native editor or OS-dialog acceptance.
const {test}=require('node:test'),assert=require('node:assert/strict'),fs=require('node:fs'),vm=require('node:vm'),path=require('node:path');
function fixture(){
 const source=fs.readFileSync(path.join(__dirname,'../media/native-authority.js'),'utf8');
 const begin=source.indexOf(' function rpc('),end=source.indexOf(' function accept(',begin);assert.ok(begin>=0&&end>begin);
 const sent=[],requests=new Map(),base={authorityEpoch:'public-test-epoch'};
 const context=vm.createContext({setTimeout,clearTimeout,Error,Promise,requests,base,paused:false,sequence:0,identity:()=>({protocol:1,authorityEpoch:base.authorityEpoch}),vscode:{postMessage:value=>sent.push(value)}});
 vm.runInContext(source.slice(begin,end)+'\nglobalThis.actualRpc=rpc;',context);
 return {rpc:context.actualRpc,requests,sent,base};
}
test('an actual interactive Open request remains pending beyond the mutation deadline', {timeout:25000},async()=>{
 const f=fixture();let settled=false,rejected;const opened=f.rpc('open').then(value=>{settled=true;return value;},error=>{settled=true;rejected=error;});
 await new Promise(resolve=>setTimeout(resolve,16000));
 assert.equal(settled,false,'a user taking time in the OS picker must not invalidate native document authority: '+String(rejected?.message));
 assert.equal(f.requests.size,1);assert.equal(f.sent[0].type,'open');
 const waiting=f.requests.get(f.sent[0].id);waiting.resolve({ok:true});await opened;
 assert.equal(settled,true);
});
test('real native mutation requests retain the existing uncertainty deadline', {timeout:25000},async()=>{
 const f=fixture();await assert.rejects(f.rpc('authority-prepare'),error=>error.authorityUncertain===true&&/timed out/.test(error.message));
 assert.equal(f.requests.size,0);assert.equal(f.sent[0].type,'authority-prepare');
});

test('actual Markdown and knowledge-graph export requests await the interactive destination beyond mutation deadline', {timeout:25000},async()=>{
 const exports=['markdown','knowledge-graph'].map(format=>{const f=fixture();let settled=false,rejected;const result=f.rpc('authority-export',{format}).then(value=>{settled=true;return value;},error=>{settled=true;rejected=error;});return {f,format,result,settled:()=>settled,rejected:()=>rejected};});
 await new Promise(resolve=>setTimeout(resolve,16000));
 for(const entry of exports){assert.equal(entry.settled(),false,'interactive '+entry.format+' destination invalidated authority: '+String(entry.rejected()?.message));assert.equal(entry.f.requests.size,1);const sent=entry.f.sent[0];assert.equal(sent.type,'authority-export');assert.equal(sent.format,entry.format);entry.f.requests.get(sent.id).resolve({ok:true});await entry.result;assert.equal(entry.settled(),true);}
});
test('finish abort status history control query and Save keep their actual bounded deadlines', {timeout:25000},async()=>{
 await Promise.all(['authority-finish','authority-abort','authority-status','authority-history','authority-control','authority-query','save'].map(async type=>{const f=fixture();await assert.rejects(f.rpc(type),error=>error.authorityUncertain===true&&/timed out/.test(error.message));assert.equal(f.requests.size,0);assert.equal(f.sent[0].type,type);}));
});
