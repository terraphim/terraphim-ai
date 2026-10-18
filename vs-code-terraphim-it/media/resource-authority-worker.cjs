'use strict';
// Fixed packaged entry. No document-controlled imports, function names or receipt imports.
const {parentPort,workerData}=require('node:worker_threads');
const fs=require('node:fs/promises');
const path=require('node:path');
const {createHash}=require('node:crypto');
const LIMIT=16*1024*1024;
const hash=s=>createHash('sha256').update(s,'utf8').digest('hex');
const size=(s,name='text')=>{if(typeof s!=='string'||Buffer.byteLength(s,'utf8')>LIMIT)throw Error(`${name} exceeds source/argument limit`);return s;};
const integer=(v,min=0)=>Number.isSafeInteger(v)&&v>=min;
const decimal=v=>typeof v==='string'&&/^(0|[1-9][0-9]*)$/.test(v);
const same=(a,b)=>JSON.stringify(a)===JSON.stringify(b);
const baseKeys=['resource','authorityEpoch','revision','sourceHash','historyGeneration'];
const requestKeys=[...baseKeys,'protocol','viewSession','requestId','dispatchId'];
const exact=(value,keys)=>{if(!value||typeof value!=='object'||Array.isArray(value)||Object.keys(value).some(k=>!keys.includes(k))||keys.some(k=>!Object.hasOwn(value,k)))throw Error('Invalid authority envelope');};
const viewIds=new Map();let lastDispatch=0,accepted,pending,journal,api,pendingProvider,failure,reconciled,crlf=false,providerGeneration=0n;
function role(){const s=api.kg_role_snapshot_json();const value=typeof s==='string'?JSON.parse(s):s;if(!decimal(value.generation))throw Error('Invalid native role generation');return value.generation;}
function persisted(source){size(source,'Native source');return size(api.host_persistence_source(source,crlf),'Persisted source');}
function projection(native={}){return {body:api.document_body(),annotations:api.document_annotations(),counts:api.document_counts(),overflow:api.document_overflow(),export:api.export_document(),notice:native.notice??native.result?.notice??null};}
function snapshot(source,native={}){return {source:persisted(source),presentationSource:source,projection:projection(native),roleGeneration:role(),providerGeneration:String(providerGeneration)};}
function assertBase(base){for(const key of baseKeys)if(base[key]!==accepted[key])throw Error(`Stale authority ${key}`);}
function assertRequest(request,consume=false){exact(request,requestKeys);
 if(request.protocol!==1||request.resource!==accepted.resource||request.authorityEpoch!==accepted.authorityEpoch||typeof request.viewSession!=='string'||!integer(request.requestId,1)||!integer(request.dispatchId,1)||!integer(request.revision,1)||!integer(request.historyGeneration)||!/^[a-f0-9]{64}$/.test(request.sourceHash))throw Error('Invalid authority request identity');
 if(!viewIds.has(request.viewSession))throw Error('Disposed or unknown authority view');
 if(consume){if(request.requestId<=viewIds.get(request.viewSession)||request.dispatchId<=lastDispatch)throw Error('Stale authority request identity');viewIds.set(request.viewSession,request.requestId);lastDispatch=request.dispatchId;}
 assertBase(request);
}
function sameRequest(a,b){return requestKeys.every(k=>a?.[k]===b?.[k]);}
function reconciledRequest(request){return reconciled&&sameRequest(request,reconciled.request)&&baseKeys.every(key=>accepted[key]===reconciled.accepted[key]);}
function target(request,token){const current=pending||journal?.prepared;if(!current||!sameRequest(request,current.request)||token!==current.token)throw Error('Foreign or retired authority token');return current;}
function requireIdle(){if(failure)throw Error('Native context failed; explicit recovery required');if(pending||journal)throw Error('Native authority requires transaction reconciliation');}
function boundedAction(action){exact(action,['expectedBody','meta','textArgs']);size(action.expectedBody,'Expected body');size(JSON.stringify(action.meta),'Action metadata');if(!Array.isArray(action.textArgs)||action.textArgs.length>4096)throw Error('Action argument count limit');let total=0;for(const text of action.textArgs){size(text,'Action argument');total+=Buffer.byteLength(text);if(total>LIMIT)throw Error('Combined text arguments exceed limit');}}
function prepared(request,native,protectedRoleGeneration){if(native===null||native===undefined)return {kind:'no-change',request,sourceHash:accepted.sourceHash};if(native.ok!==true||typeof native.token!=='string'||!native.token)throw Error('Invalid native begin acknowledgement');
 let state;try{state=snapshot(api.save_document(),native);}catch(error){let aborted;try{aborted=api.abort_document_history_transaction(native.token);}catch{}if(aborted?.ok!==true||aborted.rolledBack!==true||aborted.token!==native.token)failure='Native preparation could not be rolled back';throw error;}const value={kind:'prepared',request,token:native.token,beforeHash:accepted.sourceHash,afterHash:hash(state.source),...state,result:native,providerGeneration:String(providerGeneration)};
 if(protectedRoleGeneration!==undefined)value.protectedRoleGeneration=protectedRoleGeneration;
 pending=value;return value;
}
function outcome(kind,action,state,reason){return {kind,request:action.request,token:action.token,sourceHash:hash(state.source),source:state.source,presentationSource:state.presentationSource,projection:state.projection,...(reason?{reason}:{})};}
async function init(){
 const o=workerData;if(!o||typeof o.resource!=='string'||!o.resource||typeof o.authorityEpoch!=='string'||!o.authorityEpoch||!integer(o.revision,1)||!integer(o.historyGeneration))throw Error('Invalid authority initialization');size(o.source,'Initial source');
 const assets=path.join(o.extensionPath,'media','editor');const manifest=JSON.parse(await fs.readFile(path.join(o.extensionPath,'media','editor-build.json'),'utf8'));
 async function asset(name){const bytes=await fs.readFile(path.join(assets,name));if(typeof manifest.assets?.[name]!=='string'||hash(bytes)!==manifest.assets[name])throw Error(`Pinned authority asset mismatch: ${name}`);return bytes;}
 const glue=await asset('terraphim_editor.js');const wasm=await asset('terraphim_editor_bg.wasm');
 api=await import('data:text/javascript;base64,'+glue.toString('base64'));
 // Admit the whole supported surface before opening any native document. A partial
 // candidate must not initialize successfully and fail only at export or replay.
 const required=['default','open_document','save_document','host_presentation_source','host_persistence_source',
  'document_body','document_annotations','document_counts','document_overflow','document_synonyms','export_document','export_knowledge_graph',
  'begin_document_action','begin_document_history_replay','begin_trim_ghost_transaction','finish_document_history_transaction','abort_document_history_transaction',
  'kg_load_thesaurus','kg_clear_thesaurus','kg_prepare_role','kg_commit_prepared_role','kg_discard_prepared_role','kg_role_snapshot_json',
  'alternatives_provider_register','alternatives_provider_unregister','alternatives_provider_ids','trim_plan_json','trim_status','trim_make_cuts'];
 for(const name of required)if(typeof api[name]!=='function')throw Error(`Native authority capability missing: ${name}`);
 await api.default({module_or_path:wasm});
 crlf=o.crlf===true;api.open_document(api.host_presentation_source(o.source));
 const roleJson=o.roleJson===undefined?(await asset('roles/default.json')).toString('utf8'):size(o.roleJson,'Role data');api.kg_load_thesaurus(roleJson);
 for(const provider of o.providers||[]){if(typeof provider.id!=='string'||typeof provider.model!=='string')throw Error('Invalid provider configuration');api.alternatives_provider_register(provider.id,provider.model);providerGeneration++;}
 const state=snapshot(api.save_document());accepted={resource:o.resource,authorityEpoch:o.authorityEpoch,revision:o.revision,sourceHash:hash(state.source),historyGeneration:o.historyGeneration,...state};
}
const initialized=init();
async function operation(name,p){await initialized;
 switch(name){
 case'ready':case'snapshot':return accepted;
 case'attach':if(typeof p.viewSession!=='string'||!p.viewSession||p.viewSession.length>256||viewIds.has(p.viewSession))throw Error('Invalid or duplicate view session');viewIds.set(p.viewSession,0);return null;
 case'detach':if(pending?.request.viewSession===p.viewSession)throw Error('Settle pending view transaction before disposal');viewIds.delete(p.viewSession);return null;
 case'prepare-action':{assertRequest(p.request,true);requireIdle();boundedAction(p.action);let generation;
  if(p.action.meta.kind==='kg_swap'||p.action.meta.kind==='append_alternatives'){generation=role();if(p.action.meta.roleGeneration!==generation)throw Error('Stale role generation');}
  if(p.action.meta.kind==='append_alternatives'&&p.action.meta.providerGeneration!==String(providerGeneration))throw Error('Stale provider generation');
  reconciled=undefined;const value=prepared(p.request,api.begin_document_action(p.action.expectedBody,JSON.stringify(p.action.meta),p.action.textArgs),generation);if(value.kind==='prepared')pendingProvider=p.action.meta.kind==='append_alternatives'?String(providerGeneration):undefined;return value;}
 case'prepare-history':{assertRequest(p.request,true);requireIdle();exact(p.replay,['expectedBody','targetBody','operationsJson']);size(p.replay.expectedBody);size(p.replay.targetBody);size(p.replay.operationsJson);reconciled=undefined;return prepared(p.request,api.begin_document_history_replay(p.replay.expectedBody,p.replay.targetBody,p.replay.operationsJson));}
 case'prepare-trim':{assertRequest(p.request,true);requireIdle();exact(p.trim,['expectedBody','level','kept']);size(p.trim.expectedBody);if(!['slight','tighten','sharper','half'].includes(p.trim.level)||!Array.isArray(p.trim.kept)||p.trim.kept.length>4096||p.trim.kept.some(n=>!integer(n)))throw Error('Invalid protected trim');reconciled=undefined;return prepared(p.request,api.begin_trim_ghost_transaction(p.trim.expectedBody,p.trim.level,JSON.stringify(p.trim.kept)),role());}
 case'finish':{const action=target(p.request,p.token);if(p.afterHash!==action.afterHash)throw Error('Prepared source hash mismatch');if(journal){if(journal.outcome.kind!=='committed')throw Error('Transaction already aborted');return journal.outcome;}
  if(!viewIds.has(p.request.viewSession))throw Error('Disposed authority view');
  if(pendingProvider!==undefined && pendingProvider!==String(providerGeneration))return outcome('refused-with-pending-token',action,action,'Provider generation changed before finish');
  const value=outcome('committed',action,action);let result;try{result=api.finish_document_history_transaction(p.token);}catch(error){return outcome('refused-with-pending-token',action,action,String(error?.message||JSON.stringify(error)));}
  if(result?.ok!==true||result.finalized!==true||result.token!==p.token){failure='Native finalization outcome is uncertain';throw Error(failure);}
  journal={prepared:action,outcome:value};pending=undefined;pendingProvider=undefined;return value;}
 case'abort':{const action=target(p.request,p.token);if(journal){if(journal.outcome.kind!=='guarded-aborted')throw Error('Transaction already committed');return journal.outcome;}
  const value=outcome('guarded-aborted',action,accepted);let result;try{result=api.abort_document_history_transaction(p.token);}catch(error){return outcome('refused-with-pending-token',action,action,String(error?.message||JSON.stringify(error)));}
  if(result?.ok!==true||result.rolledBack!==true||result.token!==p.token){failure='Native abort outcome is uncertain';throw Error(failure);}
  journal={prepared:action,outcome:value};pending=undefined;pendingProvider=undefined;try{value.projection=projection();}catch(error){failure='Native abort finalized, but projection could not be reconciled';throw error;}return value;}
 case'status':if(p.request){const action=pending||journal?.prepared;if((!action||!sameRequest(action.request,p.request))&&!reconciledRequest(p.request))throw Error('Unknown authority transaction identity');}return {state:failure?'failed':pending?'pending':journal?(journal.outcome.kind==='committed'?'committed-unreconciled':'aborted-unreconciled'):'idle',accepted,...(failure?{reason:failure}:{}),...(pending?{prepared:pending}:{}),...(journal?{outcome:journal.outcome}:{})};
 case'ack':{if(!journal&&reconciledRequest(p.request)&&same(p.accepted,reconciled.accepted)&&hash(JSON.stringify(p.outcome))===reconciled.digest)return null;
  if(!journal||!sameRequest(journal.prepared.request,p.request)||!same(p.outcome,journal.outcome))throw Error('Terminal outcome reconciliation mismatch');exact(p.accepted,baseKeys);const changed=journal.outcome.kind==='committed';
  if(p.accepted.resource!==accepted.resource||p.accepted.authorityEpoch!==accepted.authorityEpoch||p.accepted.sourceHash!==journal.outcome.sourceHash||p.accepted.revision!==accepted.revision+(changed?1:0)||p.accepted.historyGeneration!==accepted.historyGeneration+(changed?1:0))throw Error('Accepted authority tuple mismatch');
  const receipt={request:p.request,accepted:p.accepted,digest:hash(JSON.stringify(p.outcome))};
  accepted={...accepted,...p.accepted,source:journal.outcome.source,presentationSource:journal.outcome.presentationSource,projection:journal.outcome.projection,roleGeneration:role(),providerGeneration:String(providerGeneration)};reconciled=receipt;journal=undefined;return null;}
 case'update-history':requireIdle();exact(p.expectedBase,baseKeys);assertBase(p.expectedBase);if(!integer(p.nextHistoryGeneration)||p.nextHistoryGeneration!==accepted.historyGeneration+1)throw Error('Invalid history generation');accepted={...accepted,historyGeneration:p.nextHistoryGeneration};return accepted;
 case'control':{if(failure)throw Error('Native context requires explicit recovery');if(journal)throw Error('Reconcile native outcome before role/provider control');let result;
  const fields=p.kind==='role-clear'?['kind']:['role-load','role-prepare'].includes(p.kind)?['kind','json']:['role-commit','role-discard'].includes(p.kind)?['kind','token']:p.kind==='provider-register'?['kind','id','model']:['kind','id'];exact(p,fields);
  switch(p.kind){case'role-load':result=api.kg_load_thesaurus(size(p.json));break;case'role-clear':result=api.kg_clear_thesaurus();break;case'role-prepare':result=api.kg_prepare_role(size(p.json));break;case'role-commit':result=api.kg_commit_prepared_role(size(p.token));break;case'role-discard':result=api.kg_discard_prepared_role(size(p.token));break;case'provider-register':result=api.alternatives_provider_register(size(p.id),size(p.model));providerGeneration++;break;case'provider-unregister':result=api.alternatives_provider_unregister(size(p.id));providerGeneration++;break;default:throw Error('Unknown authority control');}
  accepted={...accepted,roleGeneration:role(),providerGeneration:String(providerGeneration),...(!pending?{projection:projection()}: {})};return result??null;}
 case'query':{requireIdle();if(!Array.isArray(p.args))throw Error('Invalid query arguments');for(const arg of p.args)size(JSON.stringify(arg),'Query argument');const arity=['trim_status','trim_make_cuts','document_synonyms'].includes(p.name)?2:0;if(p.args.length!==arity||p.args.some(arg=>typeof arg!=='string'))throw Error('Invalid query argument shape');switch(p.name){
  case'kg_role_snapshot_json':return api.kg_role_snapshot_json();case'alternatives_provider_ids':return api.alternatives_provider_ids();case'trim_plan_json':return api.trim_plan_json();case'trim_status':return api.trim_status(...p.args);case'trim_make_cuts':return api.trim_make_cuts(...p.args);case'document_synonyms':return api.document_synonyms(...p.args);case'export_knowledge_graph':return api.export_knowledge_graph();case'export_document':return api.export_document();default:throw Error('Unknown authority query');}}
 default:throw Error('Unknown authority operation');
 }
}
// Serialize actual WASM calls; no second request can race a native transaction.
let queue=Promise.resolve();
parentPort.on('message',message=>{queue=queue.then(async()=>{if(!message||!integer(message.rpc,1)||typeof message.operation!=='string')return;try{const value=await operation(message.operation,message.payload||{});parentPort.postMessage({rpc:message.rpc,ok:true,value});}catch(error){parentPort.postMessage({rpc:message.rpc,ok:false,error:error instanceof Error?error.message:JSON.stringify(error),fatal:Boolean(failure)});}}).catch(error=>{throw error;});});
