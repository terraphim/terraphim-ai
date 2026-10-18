import {Worker} from 'worker_threads';
import * as path from 'path';

/** Existing Rust JSON data; never executable callbacks or arbitrary WASM RPC. */
export type NativeJson = null|boolean|number|string|NativeJson[]|{[key:string]:NativeJson};
export interface AuthorityBase {resource:string;authorityEpoch:string;revision:number;sourceHash:string;historyGeneration:number;}
export interface AuthorityRequest extends AuthorityBase {protocol:1;viewSession:string;requestId:number;dispatchId:number;}
export interface DocumentProjection {body:string;annotations:NativeJson;counts:NativeJson;overflow:string;export:string;notice:NativeJson;}
export interface AuthoritySnapshot extends AuthorityBase {source:string;presentationSource:string;projection:DocumentProjection;roleGeneration:string;providerGeneration:string;}
export interface EditMeta {start:number;deleteUtf16:number;insertedRef:number;beforeBodyHash:string;afterBodyHash:string;}
export type NativeActionMeta =
 | ({kind:'text_edit'}&EditMeta)|{kind:'text_edits';edits:EditMeta[]}
 |{kind:'ghost_range'|'revive_range';start:number;end:number}
 |{kind:'alt_create';spanKind:'word'|'sentence'|'paragraph';start:number;end:number;textRef:number;source:'human'|'ai'}
 |{kind:'alt_add';spanId:string;textRef:number;source:'human'|'ai'}
 |{kind:'alt_edit';spanId:string;index:number;textRef:number}
 |{kind:'alt_remove';spanId:string;index:number}|{kind:'alt_move';spanId:string;from:number;to:number}
 |{kind:'alt_active';spanId:string;index:number}|{kind:'kg_swap';spanId:string;index:number;roleGeneration:string}
 |{kind:'append_alternatives';start:number;end:number;suppliedJsonRef:number;roleGeneration:string;providerGeneration:string}
 |{kind:'stash';start:number;end:number}|{kind:'pull';at:number;from:number;to:number}|{kind:'overflow_set';textRef:number};
export interface NativeAction {expectedBody:string;meta:NativeActionMeta;textArgs:string[];}
export interface HistoryReplay {expectedBody:string;targetBody:string;operationsJson:string;}
export interface ProtectedTrim {expectedBody:string;level:'slight'|'tighten'|'sharper'|'half';kept:number[];}
export interface PreparedAction {kind:'prepared';request:AuthorityRequest;token:string;beforeHash:string;afterHash:string;source:string;presentationSource:string;projection:DocumentProjection;result:NativeJson;protectedRoleGeneration?:string;providerGeneration:string;}
export interface NoChange {kind:'no-change';request:AuthorityRequest;sourceHash:string;}
export type TerminalOutcome = {kind:'committed'|'guarded-aborted'|'refused-with-pending-token';request:AuthorityRequest;token:string;sourceHash:string;source:string;presentationSource:string;projection:DocumentProjection;reason?:string;};
export interface AuthorityStatus {state:'idle'|'pending'|'committed-unreconciled'|'aborted-unreconciled'|'failed';accepted:AuthoritySnapshot;prepared?:PreparedAction;outcome?:TerminalOutcome;reason?:string;}
export type AuthorityControl = {kind:'role-load'|'role-prepare';json:string}|{kind:'role-clear'}|{kind:'role-commit'|'role-discard';token:string}|{kind:'provider-register';id:string;model:string}|{kind:'provider-unregister';id:string};
export type AuthorityQuery = 'kg_role_snapshot_json'|'alternatives_provider_ids'|'trim_plan_json'|'trim_status'|'trim_make_cuts'|'document_synonyms'|'export_knowledge_graph'|'export_document';
export interface AuthorityOptions {extensionPath:string;resource:string;authorityEpoch:string;revision:number;source:string;historyGeneration:number;crlf?:boolean;roleJson?:string;providers?:{id:string;model:string}[];timeoutMs?:number;}
const INPUT_LIMIT=16*1024*1024;
function admitText(value:unknown,label:string):void {if(typeof value!=='string'||Buffer.byteLength(value,'utf8')>INPUT_LIMIT)throw new Error(`${label} exceeds source/argument limit`);}
function admitArguments(args:unknown):void {if(!Array.isArray(args)||args.length>4096)throw new Error('Action argument count limit');let total=0;for(const value of args){admitText(value,'Action argument');total+=Buffer.byteLength(value as string,'utf8');if(total>INPUT_LIMIT)throw new Error('Combined text arguments exceed limit');}}
interface Reply {rpc:number;ok:boolean;value?:unknown;error?:string;fatal?:boolean;}
interface Waiting {resolve:(value:unknown)=>void;reject:(error:Error)=>void;timer:ReturnType<typeof setTimeout>;}
/** One asynchronous resource-owned worker. It never silently restarts lost native state. */
export class ResourceAuthority {
 private readonly worker:Worker;
 private readonly waiting=new Map<number,Waiting>();
 private readonly failureListeners=new Set<(error:Error)=>void>();
 private sequence=0;private disposed=false;private failed?:Error;private uncertain=false;
 private constructor(private readonly options:AuthorityOptions) {
  if(!path.isAbsolute(options.extensionPath))throw new Error('Authority extension path must be absolute');
  for(const value of [options.source,options.roleJson])if(value!==undefined&&(typeof value!=='string'||Buffer.byteLength(value)>16*1024*1024))throw new Error('Authority initial source/role exceeds limit');
  if(options.timeoutMs!==undefined&&(!Number.isSafeInteger(options.timeoutMs)||options.timeoutMs<1||options.timeoutMs>60000))throw new Error('Invalid authority timeout');
  this.worker=new Worker(path.join(options.extensionPath,'media','resource-authority-worker.cjs'),{
   workerData:{...options,timeoutMs:undefined},resourceLimits:{maxOldGenerationSizeMb:256,stackSizeMb:8},
  });
  this.worker.on('message',(reply:Reply)=>{
   if(!reply || !Number.isSafeInteger(reply.rpc))return;
   if(reply.fatal){this.fail(new Error(reply.error||'Native authority requires explicit recovery'));return;}
   const pending=this.waiting.get(reply.rpc);if(!pending)return;
   clearTimeout(pending.timer);this.waiting.delete(reply.rpc);
   if(reply.ok)pending.resolve(reply.value);else pending.reject(new Error(reply.error||'Native authority refused'));
  });
  this.worker.on('error',error=>this.fail(error));
  this.worker.on('messageerror',error=>this.fail(error instanceof Error?error:new Error(String(error))));
  this.worker.on('exit',code=>{if(!this.disposed)this.fail(new Error(`Native authority context exited (${code}); explicit recovery is required`));});
 }
 static async create(options:AuthorityOptions):Promise<ResourceAuthority> {
  const authority=new ResourceAuthority(options);
  try {await authority.call('ready',{});return authority;}catch(error){await authority.dispose();throw error;}
 }
 onFailure(listener:(error:Error)=>void):()=>void {this.failureListeners.add(listener);if(this.failed)listener(this.failed);return()=>this.failureListeners.delete(listener);}
 private fail(error:Error):void {if(this.failed||this.disposed)return;this.failed=error;
  for(const pending of this.waiting.values()){clearTimeout(pending.timer);pending.reject(error);}this.waiting.clear();
  for(const listener of this.failureListeners){try{listener(error);}catch{/* Other recovery subscribers still run. */}}
 }
 private call<T>(operation:string,payload:unknown):Promise<T> {
  if(this.disposed)return Promise.reject(new Error('Native authority disposed'));
  if(this.failed)return Promise.reject(this.failed);
  const rpc=++this.sequence;if(!Number.isSafeInteger(rpc))return Promise.reject(new Error('Authority RPC identities exhausted'));
  return new Promise<T>((resolve,reject)=>{
   const timer=setTimeout(()=>{this.waiting.delete(rpc);this.uncertain=true;reject(new Error('Native authority reply timed out; query status before further mutation'));},this.options.timeoutMs??15000);
   this.waiting.set(rpc,{resolve:value=>resolve(value as T),reject,timer});
   try{this.worker.postMessage({rpc,operation,payload});}catch(error){clearTimeout(timer);this.waiting.delete(rpc);reject(error instanceof Error?error:new Error(String(error)));}
  });
 }
 private mutation<T>(operation:string,payload:unknown):Promise<T>{if(this.uncertain)return Promise.reject(new Error('Native authority outcome is uncertain; status reconciliation required'));return this.call<T>(operation,payload);}
 snapshot():Promise<AuthoritySnapshot>{return this.call('snapshot',{});}
 attachView(viewSession:string):Promise<void>{return this.call('attach',{viewSession});}
 detachView(viewSession:string):Promise<void>{return this.call('detach',{viewSession});}
 prepareAction(request:AuthorityRequest,action:NativeAction):Promise<PreparedAction|NoChange>{try{admitText(action.expectedBody,'Expected body');admitText(JSON.stringify(action.meta),'Action metadata');admitArguments(action.textArgs);}catch(error){return Promise.reject(error);}return this.mutation('prepare-action',{request,action});}
 prepareHistory(request:AuthorityRequest,replay:HistoryReplay):Promise<PreparedAction>{try{admitText(replay.expectedBody,'Expected body');admitText(replay.targetBody,'Target body');admitText(replay.operationsJson,'History operations');}catch(error){return Promise.reject(error);}return this.mutation('prepare-history',{request,replay});}
 prepareTrim(request:AuthorityRequest,trim:ProtectedTrim):Promise<PreparedAction|NoChange>{try{admitText(trim.expectedBody,'Expected body');if(!Array.isArray(trim.kept)||trim.kept.length>4096)throw new Error('Protected trim count limit');}catch(error){return Promise.reject(error);}return this.mutation('prepare-trim',{request,trim});}
 finish(request:AuthorityRequest,token:string,afterHash:string):Promise<TerminalOutcome>{return this.call('finish',{request,token,afterHash});}
 abort(request:AuthorityRequest,token:string):Promise<TerminalOutcome>{return this.call('abort',{request,token});}
 async status(request?:AuthorityRequest):Promise<AuthorityStatus>{const result=await this.call<AuthorityStatus>('status',{request});this.uncertain=false;return result;}
 ackOutcome(request:AuthorityRequest,outcome:TerminalOutcome,accepted:AuthorityBase):Promise<void>{return this.call('ack',{request,outcome,accepted});}
 updateHistory(expectedBase:AuthorityBase,nextHistoryGeneration:number):Promise<AuthoritySnapshot>{return this.mutation('update-history',{expectedBase,nextHistoryGeneration});}
 query(name:AuthorityQuery,args:NativeJson[]=[]):Promise<NativeJson>{try{if(!Array.isArray(args)||args.length>2)throw new Error('Query argument count limit');for(const arg of args)admitText(JSON.stringify(arg),'Query argument');}catch(error){return Promise.reject(error);}return this.call('query',{name,args});}
 control(control:AuthorityControl):Promise<NativeJson>{try{for(const value of Object.values(control))admitText(value,'Control argument');}catch(error){return Promise.reject(error);}return this.mutation('control',control);}
 async dispose():Promise<void>{if(this.disposed)return;this.disposed=true;
  const error=new Error('Native authority disposed');for(const pending of this.waiting.values()){clearTimeout(pending.timer);pending.reject(error);}this.waiting.clear();await this.worker.terminate();this.failureListeners.clear();
 }
}
