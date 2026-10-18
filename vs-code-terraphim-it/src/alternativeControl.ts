import * as vscode from 'vscode';
import { createHash, randomUUID } from 'crypto';
import { exportKnowledgeGraph, checkpointProjection } from '../media/knowledge-graph.cjs';
import { getHeapStatistics } from 'v8';
import { CheckpointStage } from './checkpointStage';
import { ResourceAuthority, AuthorityRequest, PreparedAction, TerminalOutcome, AuthorityBase, AuthoritySnapshot } from './resourceAuthority';
import { ResourceBackup, encodeResourceBackup, decodeResourceBackup, resourceDirectoryKey, resourceBackupFileName, canRetireResourceBackup, MAX_RESOURCE_BACKUPS, resourceBackupNames, admitResourceBackupBytes, resourceBackupBaselineMatches } from './resourceBackup';
import { assertSaveSnapshotCurrent } from './saveSnapshot';
import { flushStableSnapshots, FlushSnapshot } from './flushSnapshots';
import { HistoryCommands } from './historyCommands';
import { AuthorityDraftStage } from './authorityDraftStage';
import { AuthorityDraft, encodeRecovery, decodeRecovery } from './authorityRecovery';
import { validateCheckpoint, chunks } from '../media/history-checkpoint';
import { assertSeparateExportTarget } from '../media/export-target.cjs';
const VIEW = 'terraphimIt.alternativeControl';
const LIMIT = 16 * 1024 * 1024;
const hash = (source: string) => createHash('sha256').update(source, 'utf8').digest('hex');
interface HostMessage {
  protocol: number; type: string; resource: string; session: string; revision: number;
  authorityEpoch?:string; operation?:Record<string,unknown>; route?:string; expectedBody?:string; meta?:Record<string,unknown>; textArgs?:string[]; targetBody?:string; operationsJson?:string; level?:string; kept?:number[]; afterHash?:string; draft?:string; draftHash?:string; draftSequence?:number; name?:string; args?:unknown[];
  id?: number; beforeHash?: string; sourceHash?: string; source?: string;
  format?: 'markdown' | 'knowledge-graph'; checkpoint?: unknown; historyGeneration?: number; nonce?: string; checkpointToken?:string; token?:string; index?:number; total?:number; bytes?:number; text?:string; historyCommandNonce?:string; viewEpoch?:number;
}
interface View {
  session: string; nativeAttached?:boolean; draftStage:AuthorityDraftStage; panel: vscode.WebviewPanel; lastRequest: number; exportPending?: number; stage:CheckpointStage; epoch:number; historyCommands:HistoryCommands; sending?:Promise<boolean>;
  flush?: {nonce:string;resolve:(snapshot:FlushSnapshot)=>void;reject:(error:Error)=>void;timer:ReturnType<typeof setTimeout>};
}
class AlternativeDocument implements vscode.CustomDocument {
  readonly views = new Set<View>();
  queue: Promise<unknown> = Promise.resolve();
  authorityEpoch=randomUUID(); dispatchId=0; roleSnapshot?:unknown;recoveryArchive?:string; authority?:ResourceAuthority; authorityInitialization?:Promise<ResourceAuthority>; authorityReset?:Promise<void>; pendingAuthority?:{view:View;request:AuthorityRequest;prepared:PreparedAction;command?:{nonce:string;epoch:number};checkpoint?:unknown;draftSequence?:number;published?:AuthorityBase};
  readonly drafts=new Map<string,AuthorityDraft>();readonly providerConfig=new Map<string,string>();
  revision=1; historyGeneration=0; checkpoint:unknown; conflictSource?:string;
  recoveryDirty=false; recoveredBackup?:{uri:vscode.Uri;record:ResourceBackup;rawHash:string}; readonly backupHashes=new Map<string,string>();
  disposed=false; commandSave=false; commandSaveAttempt?:{failed:boolean;error?:unknown}; blocked=false; readonly backups=new Map<string,string>();
  constructor(readonly uri:vscode.Uri, public source:string, public diskSource:string,private readonly onDispose:()=>void) {}
  dispose():void { this.disposed=true;void this.authority?.dispose();this.onDispose(); for (const view of this.views) {
    view.stage.clear();view.draftStage.clear();
    view.historyCommands.clear();if (view.flush) { clearTimeout(view.flush.timer);view.flush.reject(new Error('Document disposed')); }
  } this.views.clear(); }
}
export function registerAlternativeControl(context:vscode.ExtensionContext):void {
 const provider=new AlternativeControlProvider(context.extensionUri,context.globalStorageUri);
 context.subscriptions.push(provider,vscode.window.registerCustomEditorProvider(VIEW,provider,{
  supportsMultipleEditorsPerDocument:true,webviewOptions:{retainContextWhenHidden:true},
 }));
 context.subscriptions.push(vscode.commands.registerCommand('terraphimIt.openAlternativeControl',async()=>{
  const uri=vscode.window.activeTextEditor?.document.uri;
  if (uri) await vscode.commands.executeCommand('vscode.openWith',uri,VIEW);
  else { const selected=await vscode.window.showOpenDialog({canSelectMany:false,filters:{Markdown:['md']}});
   if (selected?.[0]) await vscode.commands.executeCommand('vscode.openWith',selected[0],VIEW); }
 }));
 context.subscriptions.push(vscode.commands.registerCommand('terraphimIt.exportKnowledgeGraph',()=>provider.activeCommand('request-export')));
 for (const command of ['undo','redo']) context.subscriptions.push(vscode.commands.registerCommand('terraphimIt.'+command,()=>provider.activeCommand('request-history',command)));
}
/** Resource-owned custom-document persistence and native editing coordinator. */
export class AlternativeControlProvider implements vscode.CustomEditorProvider<AlternativeDocument>,vscode.Disposable {
 private readonly resources=new Map<string,AlternativeDocument>();
 private readonly changes=new vscode.EventEmitter<vscode.CustomDocumentContentChangeEvent<AlternativeDocument>>();
 readonly onDidChangeCustomDocument=this.changes.event;
 private readonly subscriptions:vscode.Disposable[]=[];
 constructor(private readonly extensionUri:vscode.Uri,private readonly storageUri:vscode.Uri) {
  this.subscriptions.push(vscode.workspace.onDidChangeTextDocument(event=>{
   if (!event.contentChanges.length) return;
   const doc=this.resources.get(event.document.uri.toString());
   if (doc && event.document.getText()!==doc.source) void this.external(doc,true);
  }));
  const watcher=vscode.workspace.createFileSystemWatcher('**/*.md');
  for (const event of [watcher.onDidChange,watcher.onDidCreate,watcher.onDidDelete]) this.subscriptions.push(event(uri=>{
   const doc=this.resources.get(uri.toString()); if (doc) void this.external(doc).catch(error=>this.block(doc,String(error)));
  }));
  this.subscriptions.push(watcher);
 }
 dispose():void { this.subscriptions.forEach(item=>item.dispose());this.changes.dispose(); }
 private authorityBase(doc:AlternativeDocument) {
  return {resource:doc.uri.toString(),authorityEpoch:doc.authorityEpoch,revision:doc.revision,sourceHash:hash(doc.source),historyGeneration:doc.historyGeneration};
 }
 private async native(doc:AlternativeDocument):Promise<ResourceAuthority> {
  if(doc.authorityReset)await doc.authorityReset;
  if(doc.disposed)throw new Error('Document disposed');
  if(doc.authorityInitialization)return doc.authorityInitialization;
  if(doc.authority)return doc.authority;
  const initial=this.authorityBase(doc),source=doc.source;
  const current=()=>!doc.disposed&&doc.authorityInitialization===initialization&&doc.authorityEpoch===initial.authorityEpoch&&doc.revision===initial.revision&&doc.historyGeneration===initial.historyGeneration;
  const initialization:Promise<ResourceAuthority>=(async()=>{
   let authority:ResourceAuthority|undefined;
   try {
    const role=typeof doc.roleSnapshot==='string'?JSON.parse(doc.roleSnapshot) as {loaded?:boolean;thesaurusText?:string}:undefined;
    authority=await ResourceAuthority.create({extensionPath:this.extensionUri.fsPath,...initial,source,crlf:source.includes('\r\n'),
     ...(role?.loaded&&role.thesaurusText?{roleJson:role.thesaurusText}:{}),providers:[...doc.providerConfig].map(([id,model])=>({id,model}))});
    if(!current()||doc.source!==source)throw new Error('Native initialization was retired; current draft retained');
    if(role?.loaded===false)await authority.control({kind:'role-clear'});
    const snapshot=await authority.snapshot(),roleSnapshot=await authority.query('kg_role_snapshot_json');
    if(!current()||doc.source!==source)throw new Error('Native initialization was retired; current draft retained');
    if(snapshot.resource!==initial.resource||snapshot.authorityEpoch!==initial.authorityEpoch||snapshot.revision!==initial.revision||snapshot.historyGeneration!==initial.historyGeneration||snapshot.sourceHash!==hash(snapshot.source))throw new Error('Invalid initial native snapshot');
    this.checkSize(snapshot.source);
    // Only a fresh native authority may canonicalize accepted source. Keep the
    // actual disk baseline so normalization remains unsaved until verified Save.
    const normalized=snapshot.source!==source;doc.source=snapshot.source;doc.authority=authority;doc.roleSnapshot=roleSnapshot;
    if(normalized){doc.recoveryDirty=true;if(doc.views.size)this.changes.fire({document:doc});}
    for(const item of doc.views){await authority.attachView(item.session);if(!current())throw new Error('Native initialization was retired');item.nativeAttached=true;}
    authority.onFailure(error=>{if(!doc.disposed&&doc.authority===authority&&doc.authorityEpoch===initial.authorityEpoch)void this.block(doc,`Native document context failed. Choose explicit recovery with fresh history. ${error.message}`).catch(()=>undefined);});
    return authority;
   }catch(error){if(authority){if(doc.authority===authority)doc.authority=undefined;await authority.dispose();}throw error;}
  })();
  doc.authorityInitialization=initialization;
  try{return await initialization;}finally{if(doc.authorityInitialization===initialization)doc.authorityInitialization=undefined;}
 }
 private recoveryData(doc:AlternativeDocument):{source:string;drafts:AuthorityDraft[]} {
  const drafts=[...doc.drafts.values()];
  if(doc.pendingAuthority)drafts.push({viewSession:'native-prepared-'+doc.pendingAuthority.request.dispatchId,sequence:1,draft:JSON.stringify({kind:'unaccepted-native-preparation',source:doc.pendingAuthority.prepared.source})});
  return {source:doc.source,drafts};
 }
 private recoverySnapshot(doc:AlternativeDocument):string {const data=this.recoveryData(doc);return encodeRecovery(data.source,data.drafts);}
 private resourceBackupDirectory(uri:vscode.Uri):vscode.Uri {return vscode.Uri.joinPath(this.storageUri,'native-authority-backups',resourceDirectoryKey(uri.toString()));}
 private async resourceBackups(uri:vscode.Uri):Promise<{uri:vscode.Uri;record:ResourceBackup;rawHash:string;mtime:number}[]> {
  const directory=this.resourceBackupDirectory(uri);let entries:[string,vscode.FileType][];
  try{entries=await vscode.workspace.fs.readDirectory(directory);}catch(error){if(error instanceof vscode.FileSystemError&&error.code==='FileNotFound')return [];throw error;}
  const admitted=new Set(resourceBackupNames(entries.map(([name])=>name)));
  const files=entries.filter(([name])=>admitted.has(name));
  const heap=getHeapStatistics(),stats=[];for(const [name,type] of files){if(type!==vscode.FileType.File)throw new Error('Recovery backup must be a regular private file');
   const target=vscode.Uri.joinPath(directory,name),stat=await vscode.workspace.fs.stat(target);stats.push({uri:target,name,mtime:stat.mtime,size:stat.size});}
  admitResourceBackupBytes(stats.map(item=>item.size),heap.heap_size_limit-heap.used_heap_size);
  const result=[];for(const item of stats){const raw=await this.readDisk(item.uri),record=decodeResourceBackup(raw,uri.toString(),item.name.split('.')[0]);if(record.token!==item.name.split('.')[1])throw new Error('Recovery filename/token mismatch');result.push({...item,record,rawHash:hash(raw)});}
  return result.sort((a,b)=>b.mtime-a.mtime);
 }
 private async cleanupResourceBackups(doc:AlternativeDocument,savedSource?:string):Promise<void> {
  const own=[...doc.backups].map(([id,token])=>({uri:vscode.Uri.parse(id),token,rawHash:doc.backupHashes.get(id),foreign:false}));
  if(doc.recoveredBackup)own.push({uri:doc.recoveredBackup.uri,token:doc.recoveredBackup.record.token,rawHash:doc.recoveredBackup.rawHash,foreign:true});
  for(const item of own){let raw:string;try{raw=await this.readDisk(item.uri);}catch(error){if(error instanceof vscode.FileSystemError&&error.code==='FileNotFound')continue;throw error;}
   if(hash(raw)!==item.rawHash)continue;const record=decodeResourceBackup(raw,doc.uri.toString());
   if(!canRetireResourceBackup(record,doc.uri.toString(),record.authorityEpoch,item.token,item.foreign?savedSource:undefined))continue;
   if(doc.pendingAuthority||doc.drafts.size)continue;
   await vscode.workspace.fs.delete(item.uri);doc.backups.delete(item.uri.toString());doc.backupHashes.delete(item.uri.toString());if(item.foreign)doc.recoveredBackup=undefined;
  }
 }
 private async preserveAuthorityRecovery(doc:AlternativeDocument):Promise<void> {
  if(doc.source!==doc.diskSource||doc.blocked||doc.drafts.size||doc.pendingAuthority){
   const directory=vscode.Uri.joinPath(this.storageUri,'native-authority-recovery');await vscode.workspace.fs.createDirectory(directory);
   const target=vscode.Uri.joinPath(directory,randomUUID()+'.json'),source=this.recoverySnapshot(doc);
   await vscode.workspace.fs.writeFile(target,Buffer.from(source,'utf8'));if(await this.readDisk(target)!==source)throw new Error('Recovery archive readback differs; old authority retained');
   doc.recoveryArchive=target.toString();
  }
 }
 private async resetAuthority(doc:AlternativeDocument,replace:()=>void):Promise<void> {
  if(doc.authorityReset)await doc.authorityReset;
  const reset=(async()=>{
   await this.preserveAuthorityRecovery(doc);
   const authority=doc.authority,initialization=doc.authorityInitialization;
   doc.authority=undefined;doc.authorityInitialization=undefined;
   await authority?.dispose();if(initialization)await initialization.then(old=>old.dispose(),()=>undefined);
   if(doc.disposed)throw new Error('Document disposed');replace();
   doc.authorityEpoch=randomUUID();for(const item of doc.views){item.nativeAttached=false;item.historyCommands.clear();}doc.pendingAuthority=undefined;
  })();
  doc.authorityReset=reset;
  try{await reset;}finally{if(doc.authorityReset===reset)doc.authorityReset=undefined;}
 }
 private authorityReply(doc:AlternativeDocument,view:View,message:HostMessage,value:Record<string,unknown>):Promise<boolean> {
  return Promise.resolve(view.panel.webview.postMessage({protocol:1,type:'authority-response',resource:doc.uri.toString(),session:view.session,id:message.id,authorityEpoch:doc.authorityEpoch,...value}));
 }
 private async publishAuthority(doc:AlternativeDocument,pending:NonNullable<AlternativeDocument['pendingAuthority']>,outcome:TerminalOutcome):Promise<void> {
  if(outcome.kind!=='committed'||!pending.checkpoint)throw new Error('Committed native outcome lacks a staged history checkpoint');
  if(!pending.published){
   Object.assign(doc,{source:pending.prepared.source,checkpoint:pending.checkpoint,revision:doc.revision+1,historyGeneration:doc.historyGeneration+1});
   pending.published=this.authorityBase(doc);this.changes.fire({document:doc});
  }
  try {await doc.authority!.ackOutcome(pending.request,outcome,pending.published);}
  catch(error){const status=await doc.authority!.status();
   if(status.state!=='idle'||status.accepted.revision!==pending.published.revision||status.accepted.historyGeneration!==pending.published.historyGeneration||status.accepted.sourceHash!==pending.published.sourceHash)throw error;
  }
  if(pending.draftSequence===doc.drafts.get(pending.view.session)?.sequence)doc.drafts.delete(pending.view.session);
  doc.pendingAuthority=undefined;
 }
 private async authorityMessage(doc:AlternativeDocument,view:View,message:HostMessage):Promise<void> {
  if(!doc.views.has(view)||message.authorityEpoch!==doc.authorityEpoch)throw new Error('Stale native authority view');
  const authority=await this.native(doc);
  if(message.type==='authority-status') {
   const pending=doc.pendingAuthority;
   let status=await authority.status(pending?.view===view?pending.request:undefined);
   if(pending?.view===view&&status.outcome?.kind==='committed'){await this.publishAuthority(doc,pending,status.outcome);status=await authority.status();}
   await this.authorityReply(doc,view,message,{ok:true,value:status,source:doc.source,revision:doc.revision,historyGeneration:doc.historyGeneration,snapshot:status.accepted});return;
  }
  if(message.revision!==doc.revision||message.sourceHash!==hash(doc.source)||message.historyGeneration!==doc.historyGeneration)throw new Error('Native request base changed');
  if(!Number.isSafeInteger(message.id)||(message.id??0)<=view.lastRequest)throw new Error('Stale native request identity');
  view.lastRequest=message.id!;
  if(message.type==='authority-abort') {
   const pending=doc.pendingAuthority;
   if(!pending||pending.view!==view||message.token!==pending.prepared.token)throw new Error('No matching native transaction to abort');
   let outcome:TerminalOutcome;
   try{outcome=await authority.abort(pending.request,pending.prepared.token);}catch(error){const status=await authority.status(pending.request);if(status.outcome?.kind!=='guarded-aborted')throw error;outcome=status.outcome;}
   if(outcome.kind!=='guarded-aborted')throw new Error('Native rollback was not acknowledged; explicit recovery required');
   try{await authority.ackOutcome(pending.request,outcome,this.authorityBase(doc));}catch(error){const status=await authority.status();if(status.state!=='idle'||status.accepted.sourceHash!==hash(doc.source)||status.accepted.revision!==doc.revision||status.accepted.historyGeneration!==doc.historyGeneration)throw error;}doc.pendingAuthority=undefined;
   await this.authorityReply(doc,view,message,{ok:true,rolledBack:true});return;
  }
  if(doc.blocked)throw new Error('Choose explicit recovery before native mutation');
  if(message.type==='authority-finish') {
   const pending=doc.pendingAuthority;
   if(!pending||pending.view!==view||pending.prepared.token!==message.token||pending.prepared.afterHash!==message.afterHash)throw new Error('Native finalization identity mismatch');
   if(pending.command&&(!view.panel.active||!vscode.window.state.focused||view.epoch!==pending.command.epoch||!view.historyCommands.matchesHead(pending.command.nonce,pending.command.epoch,view.epoch)))throw new Error('History command target changed before native finish');
   const prepared=pending.prepared;
   const checkpoint=validateCheckpoint(this.consumeStage(doc,view,message),prepared.presentationSource,prepared.projection.body,true);
   if(!Number.isSafeInteger(doc.revision+1)||!Number.isSafeInteger(doc.historyGeneration+1))throw new Error('Authority revision capacity exhausted');
   pending.checkpoint=checkpoint;pending.draftSequence=message.draftSequence;
   let outcome:TerminalOutcome;
   try {outcome=await authority.finish(pending.request,prepared.token,prepared.afterHash);}
   catch(error){const status=await authority.status(pending.request);if(status.outcome?.kind!=='committed')throw error;outcome=status.outcome;}
   if(outcome.kind!=='committed')throw new Error(outcome.reason||'Native finalization refused; pending transaction retained');
   await this.publishAuthority(doc,pending,outcome);
   await this.authorityReply(doc,view,message,{ok:true,source:doc.source,revision:doc.revision,historyGeneration:doc.historyGeneration,snapshot:await authority.snapshot()});
   await Promise.all([...doc.views].filter(item=>item!==view).map(item=>this.sync(doc,item,'sync')));return;
  }
  if(doc.pendingAuthority)throw new Error('A native transaction must settle before another request');
  if(message.type==='authority-clear-draft'){
   if(message.draftSequence===doc.drafts.get(view.session)?.sequence)doc.drafts.delete(view.session);
   await this.authorityReply(doc,view,message,{ok:true});return;
  }
  if(message.type==='authority-history') {
   const snapshot=await authority.snapshot();
   const checkpoint=validateCheckpoint(this.consumeStage(doc,view,message),snapshot.presentationSource,snapshot.projection.body,true);
   const next=doc.historyGeneration+1;
   await authority.updateHistory(this.authorityBase(doc),next);
   Object.assign(doc,{checkpoint,historyGeneration:next});
   if(message.draftSequence===doc.drafts.get(view.session)?.sequence)doc.drafts.delete(view.session);
   await this.authorityReply(doc,view,message,{ok:true,source:doc.source,revision:doc.revision,historyGeneration:doc.historyGeneration});
   await Promise.all([...doc.views].filter(item=>item!==view).map(item=>this.sync(doc,item,'sync')));return;
  }
  if(message.type==='authority-query') {
   const names=['trim_plan_json','trim_status','trim_make_cuts','kg_role_snapshot_json','alternatives_provider_ids','document_synonyms','export_knowledge_graph','export_document'];
   if(!names.includes(message.name??''))throw new Error('Unsupported native query');
   const value=await authority.query(message.name as Parameters<ResourceAuthority['query']>[0],message.args as Parameters<ResourceAuthority['query']>[1]);
   await this.authorityReply(doc,view,message,{ok:true,value});return;
  }
  if(message.type==='authority-export') {
   const graph=message.format==='knowledge-graph';const output=await authority.query(graph?'export_knowledge_graph':'export_document');
   if(typeof output!=='string')throw new Error('Native export did not return text');
   await this.writeExport(doc,view,message,output+(graph?'\n':''),graph?'knowledge-graph':'markdown');
   await this.authorityReply(doc,view,message,{ok:true});return;
  }
  if(message.type==='authority-control') {
   const operation=message.operation as Parameters<ResourceAuthority['control']>[0];
   const value=await authority.control(operation);
   if(operation.kind==='provider-register')doc.providerConfig.set(operation.id,operation.model);
   else if(operation.kind==='provider-unregister')doc.providerConfig.delete(operation.id);
   doc.roleSnapshot=await authority.query('kg_role_snapshot_json');
   await Promise.all([...doc.views].filter(item=>item!==view).map(item=>this.sync(doc,item,'sync',{roleChanged:true})));
   await this.authorityReply(doc,view,message,{ok:true,value,snapshot:await authority.snapshot()});return;
  }
  if(message.type!=='authority-prepare')throw new Error('Unsupported authority request');
  if(message.historyCommandNonce && (!view.panel.active||!vscode.window.state.focused||!view.historyCommands.matchesHead(message.historyCommandNonce,message.viewEpoch,view.epoch)||view.epoch!==message.viewEpoch))throw new Error('History command no longer targets the active view');
  const request:AuthorityRequest={protocol:1,...this.authorityBase(doc),viewSession:view.session,requestId:message.id!,dispatchId:++doc.dispatchId};
  if(typeof message.expectedBody!=='string')throw new Error('Native expected body missing');
  let prepared;
  try {prepared=message.route==='action'?await authority.prepareAction(request,{expectedBody:message.expectedBody,meta:message.meta as Parameters<ResourceAuthority['prepareAction']>[1]['meta'],textArgs:message.textArgs!})
   :message.route==='history'?await authority.prepareHistory(request,{expectedBody:message.expectedBody,targetBody:message.targetBody!,operationsJson:message.operationsJson!})
   :message.route==='trim'?await authority.prepareTrim(request,{expectedBody:message.expectedBody,level:message.level as Parameters<ResourceAuthority['prepareTrim']>[1]['level'],kept:message.kept!})
   :undefined;}catch(error){
    try{const status=await authority.status();if(status.prepared)doc.pendingAuthority={view,request,prepared:status.prepared};
     else if(status.state==='idle'&&status.accepted.sourceHash===hash(doc.source)&&status.accepted.revision===doc.revision&&status.accepted.historyGeneration===doc.historyGeneration){if(error&&typeof error==='object')Object.assign(error,{authorityUnchanged:true});}
     else await this.block(doc,'Native preparation outcome is uncertain; explicit recovery required');
    }catch{await this.block(doc,'Native preparation outcome is uncertain; explicit recovery required');}
    throw error;
   }
  if(!prepared)throw new Error('Unsupported native mutation route');
  if(prepared.kind==='no-change'){await this.authorityReply(doc,view,message,{ok:true,noChange:true});return;}
  doc.pendingAuthority={view,request,prepared,...(message.historyCommandNonce?{command:{nonce:message.historyCommandNonce,epoch:message.viewEpoch!}}:{})};this.checkSize(prepared.source);
  await this.authorityReply(doc,view,message,{ok:true,prepared});
 }
 private media():vscode.Uri { return vscode.Uri.joinPath(this.extensionUri,'media'); }
 private enqueue<T>(doc:AlternativeDocument,operation:()=>Promise<T>):Promise<T> {
  const next=doc.queue.then(()=>{if(doc.disposed) throw new Error('Document disposed');return operation();});
  doc.queue=next.catch(()=>undefined);return next;
 }
 private async readDisk(uri:vscode.Uri):Promise<string> {
  if (uri.scheme==='untitled') return '';
  return new TextDecoder('utf-8',{fatal:true}).decode(await vscode.workspace.fs.readFile(uri));
 }
 private normal(uri:vscode.Uri):vscode.TextDocument|undefined { return vscode.workspace.textDocuments.find(d=>d.uri.toString()===uri.toString()&&!d.isClosed); }
 private checkSize(source:string):void { if(Buffer.byteLength(source,'utf8')>LIMIT) throw new Error('Document exceeds source/message limit'); }
 private clearStage(view:View):void {view.stage.clear();}
 private sync(doc:AlternativeDocument,view:View,type='sync',extra:Record<string,unknown>={}):Promise<boolean> {
  const resetEpoch=extra.reverted?doc.authorityEpoch:undefined;
  const send=async()=>{
   for(let attempt=0;attempt<3;attempt++){
    if(resetEpoch&&doc.authorityEpoch!==resetEpoch)return false;
    const startingEpoch=doc.authorityEpoch;let authority:ResourceAuthority|undefined,authoritySnapshot:AuthoritySnapshot|undefined;
    try{authority=type==='resource-conflict'||doc.blocked&&type!=='init'?undefined:await this.native(doc);authoritySnapshot=authority?await authority.snapshot():undefined;}
    catch(error){if(doc.disposed||!doc.views.has(view))return false;if(doc.authorityEpoch!==startingEpoch)continue;throw error;}
    if(doc.disposed||!doc.views.has(view))return false;
    const source=doc.source,revision=doc.revision,historyGeneration=doc.historyGeneration,checkpoint=doc.checkpoint,authorityEpoch=doc.authorityEpoch,sourceHash=hash(source);
    if(authority&&(doc.authority!==authority||authoritySnapshot?.authorityEpoch!==authorityEpoch||authoritySnapshot.revision!==revision||authoritySnapshot.historyGeneration!==historyGeneration||authoritySnapshot.sourceHash!==sourceHash||authoritySnapshot.source!==source))continue;
    const metadata={...extra};delete metadata.authoritySnapshot;
    const roleSnapshot=doc.roleSnapshot,recoveryArchive=doc.recoveryArchive;
    if(['reject','resource-conflict','sync','init'].includes(type))this.clearStage(view);
    const token=checkpoint?randomUUID():undefined;
    if(checkpoint){const encoded=chunks(checkpoint);
     for(let index=0;index<encoded.parts.length;index++)await view.panel.webview.postMessage({protocol:1,type:'checkpoint-chunk',resource:doc.uri.toString(),session:view.session,
      revision,sourceHash,historyGeneration,authorityEpoch,token,index,total:encoded.parts.length,bytes:encoded.bytes,text:encoded.parts[index]});
    }
    if(doc.disposed||!doc.views.has(view))return false;
    if(doc.authorityEpoch!==authorityEpoch||doc.source!==source||doc.revision!==revision||doc.historyGeneration!==historyGeneration||doc.checkpoint!==checkpoint||authority&&doc.authority!==authority)continue;
    return view.panel.webview.postMessage({protocol:1,type,resource:doc.uri.toString(),session:view.session,
     revision,source,sourceHash,checkpointToken:token,historyGeneration,authorityEpoch,authority:true,authoritySnapshot,roleSnapshot,recoveryArchive,active:view.panel.active,viewEpoch:view.epoch,...metadata});
   }
   throw new Error('Native snapshot changed before synchronization; current draft retained');
  };
  const result=(view.sending??Promise.resolve(true)).then(send);view.sending=result.catch(()=>false);return result;
 }
 private stageIdentity(doc:AlternativeDocument,view:View) {
  return {resource:doc.uri.toString(),session:view.session,authorityEpoch:doc.authorityEpoch,revision:doc.revision,sourceHash:hash(doc.source),historyGeneration:doc.historyGeneration};
 }
 private stage(doc:AlternativeDocument,view:View,message:HostMessage):void {
  const heap=getHeapStatistics(),capacity=Math.min(503*LIMIT,Math.floor((heap.heap_size_limit-heap.used_heap_size)/4));
  view.stage.receive(message,this.stageIdentity(doc,view),capacity);
 }
 private consumeStage(doc:AlternativeDocument,view:View,message:HostMessage):unknown {
  return view.stage.consume(message.checkpointToken,this.stageIdentity(doc,view));
 }
 private async broadcast(doc:AlternativeDocument,extra:Record<string,unknown>={}):Promise<void> {
  await Promise.all([...doc.views].map(view=>this.sync(doc,view,'sync',extra)));
 }
 private async block(doc:AlternativeDocument,reason:string,externalSource?:string):Promise<void> {
  doc.blocked=true;
  if (externalSource!==undefined) doc.conflictSource=externalSource;
  await Promise.all([...doc.views].map(view=>this.sync(doc,view,'resource-conflict',{reason})));
 }
 private async external(doc:AlternativeDocument,fromNormal=false):Promise<void> {
  await this.enqueue(doc,async()=>{
   const source=fromNormal?this.normal(doc.uri)?.getText()??await this.readDisk(doc.uri):await this.readDisk(doc.uri);
   if(source===doc.diskSource && !this.normal(doc.uri)?.isDirty)return;
   if(doc.blocked){await this.block(doc,'External document changes need an explicit recovery choice',source);return;}
   if(source===doc.source) { if(this.normal(doc.uri)?.isDirty!==true) doc.diskSource=source;return; }
   if(doc.source!==doc.diskSource || doc.drafts.size || doc.pendingAuthority || this.normal(doc.uri)?.isDirty) {
    await this.block(doc,'External document changes need an explicit recovery choice',source);return;
   }
   this.checkSize(source);await this.resetAuthority(doc,()=>{doc.source=source;doc.diskSource=source;doc.checkpoint=undefined;doc.revision++;doc.historyGeneration++;});
   await this.broadcast(doc,{reverted:true,externalAdoption:true});
  });
 }
 async openCustomDocument(uri:vscode.Uri,options:vscode.CustomDocumentOpenContext,_token:vscode.CancellationToken):Promise<AlternativeDocument> {
  const disk=await this.readDisk(uri),normal=this.normal(uri);let recovery:{source:string;drafts:AuthorityDraft[]}|undefined;
  let retained:{uri:vscode.Uri;record:ResourceBackup;rawHash:string}|undefined,recoveredRecord:ResourceBackup|undefined,recoveryLocation:vscode.Uri|undefined;
  if(options.backupId){const target=vscode.Uri.parse(options.backupId),raw=await this.readDisk(target);let tag:unknown;try{tag=JSON.parse(raw);}catch{/* Legacy plain Markdown. */}
   if(tag&&typeof tag==='object'&&'kind' in tag&&tag.kind==='terraphim-resource-backup'){const record=decodeResourceBackup(raw,uri.toString());recovery=record;recoveredRecord=record;recoveryLocation=target;
    const expected=vscode.Uri.joinPath(this.resourceBackupDirectory(uri),resourceBackupFileName(record.authorityEpoch,record.token));if(expected.toString()===target.toString())retained={uri:target,record,rawHash:hash(raw)};
   }else recovery=decodeRecovery(raw);
  }else{
   const candidates=(await this.resourceBackups(uri)).filter(item=>item.record.source!==disk||item.record.drafts.length>0);
   if(candidates.length){
    if(normal?.isDirty){await vscode.window.showWarningMessage('Retained Terraphim backups are preserved. Save or recover the unsaved normal Markdown editor before selecting a backup.');throw new vscode.CancellationError();}
    const choices:{label:string;description:string;detail:string;item?:typeof candidates[number]}[]=candidates.map(item=>({label:'Recover backup',description:new Date(item.mtime).toISOString()+' · '+hash(item.record.source).slice(0,12),detail:`${item.record.drafts.length} pending raw version(s). Fresh Undo history; another window may still own this snapshot.`,item}));
    const picked=candidates.length===1?await vscode.window.showWarningMessage('An unsaved Terraphim backup is available. Recover it with fresh Undo history? The saved file is not changed until Save.',{modal:true},'Recover backup','Open saved file','Inspect backup'):
     await vscode.window.showQuickPick([...choices,{label:'Open saved file',description:'Keep every backup',detail:'',item:undefined}],{title:'Choose an explicit Terraphim recovery version; every unchosen backup is preserved',ignoreFocusOut:true});
    if(!picked)throw new vscode.CancellationError();
    if(picked==='Inspect backup'){await vscode.window.showTextDocument(await vscode.workspace.openTextDocument(candidates[0].uri));throw new vscode.CancellationError();}
    const chosen=typeof picked==='string'?(picked==='Recover backup'?candidates[0]:undefined):picked.item;
    if(chosen){retained=chosen;recovery=chosen.record;recoveredRecord=chosen.record;recoveryLocation=chosen.uri;}
   }
  }
  if(recoveredRecord&&!resourceBackupBaselineMatches(recoveredRecord,disk)){
   const selected=await vscode.window.showWarningMessage('The saved file changed after this backup. Both versions are preserved; inspect the backup before choosing how to combine them.',{modal:true},'Inspect backup','Open saved file');
   if(selected==='Inspect backup'){await vscode.window.showTextDocument(await vscode.workspace.openTextDocument(recoveryLocation!));throw new vscode.CancellationError();}
   if(selected!=='Open saved file')throw new vscode.CancellationError();retained=undefined;recovery=undefined;
  }
  if(recovery&&normal?.isDirty&&normal.getText()!==recovery.source){await vscode.window.showWarningMessage('Recover the unsaved normal Markdown version before restoring a different Terraphim backup. All backup files are preserved.');throw new vscode.CancellationError();}
  if(_token.isCancellationRequested)throw new vscode.CancellationError();
  const source=recovery?.source??(normal?.isDirty?normal.getText():disk);this.checkSize(source);
  const doc=new AlternativeDocument(uri,source,disk,()=>{if(this.resources.get(uri.toString())===doc)this.resources.delete(uri.toString());});
  doc.recoveredBackup=retained;doc.recoveryDirty=source!==disk||Boolean(recovery?.drafts.length);for(const draft of recovery?.drafts??[])doc.drafts.set(draft.viewSession,draft);
  if(recovery?.drafts.length){await this.preserveAuthorityRecovery(doc);doc.blocked=true;}
  this.resources.set(uri.toString(),doc);return doc;
 }
 async activeCommand(type:string,command?:string):Promise<void> {
  for (const doc of this.resources.values()) for(const view of doc.views) if(view.panel.active && vscode.window.state.focused) {
   let nonce:string|undefined;
   if(type==='request-history'){try{nonce=view.historyCommands.issue(view.epoch).nonce;}catch(error){await vscode.window.showWarningMessage(String(error));return;}}
   await view.panel.webview.postMessage({protocol:1,type,resource:doc.uri.toString(),session:view.session,command,historyCommandNonce:nonce,viewEpoch:view.epoch,format:'knowledge-graph'});return;
  }
  await vscode.window.showInformationMessage('Open and focus Terraphim Alternative Control first.');
 }
 private async drainHistory(doc:AlternativeDocument,wait=true):Promise<void> {
  const epoch=doc.authorityEpoch,participants=[...doc.views].map(view=>({view,epoch:view.epoch}));
  if(wait)await Promise.all(participants.map(item=>item.view.historyCommands.waitForIdle(item.epoch)));
  else if(participants.some(item=>item.view.historyCommands.size))throw new Error('Pending title history must settle before saving; draft retained');
  if(doc.disposed||doc.authorityEpoch!==epoch||participants.length!==doc.views.size||participants.some(item=>!doc.views.has(item.view)||item.view.epoch!==item.epoch))throw new Error('History save boundary changed; draft retained');
 }
 private async flush(doc:AlternativeDocument):Promise<void> {
  await flushStableSnapshots(()=>this.authorityBase(doc),async()=>{
   await this.drainHistory(doc);
   const participants=[...doc.views],epochs=participants.map(view=>view.epoch);
   const snapshots=await Promise.all(participants.map(view=>{
    if(view.flush)return Promise.reject(new Error('A document snapshot is already pending'));
    return new Promise<FlushSnapshot>((resolve,reject)=>{
     const nonce=randomUUID();const timer=setTimeout(()=>{view.flush=undefined;reject(new Error('Editor snapshot did not settle; draft retained'));},15000);
     view.flush={nonce,resolve,reject,timer};
     void (view.sending??Promise.resolve(true)).then(()=>view.panel.webview.postMessage({protocol:1,type:'flush-request',resource:doc.uri.toString(),session:view.session,nonce})).catch(error=>{if(view.flush?.nonce===nonce){clearTimeout(timer);view.flush=undefined;reject(error instanceof Error?error:new Error(String(error)));}});
    });
   }));
   return {snapshots,participantsUnchanged:participants.length===doc.views.size&&participants.every((view,index)=>doc.views.has(view)&&view.epoch===epochs[index]&&view.historyCommands.size===0)};
  },()=>this.broadcast(doc));
 }
 private async persist(doc:AlternativeDocument,target:vscode.Uri,token:vscode.CancellationToken,alreadyFlushed=false):Promise<void> {
  const sameTarget=target.toString()===doc.uri.toString();
  let targetBefore:string|undefined;
  if(!sameTarget) { try {targetBefore=await this.readDisk(target);} catch(error) {if(!(error instanceof vscode.FileSystemError && error.code==='FileNotFound'))throw error;} }
  // An acknowledged webview Save holds its sending queue until completion;
  // never await newly issued history behind that same queue. Refuse it safely.
  if(alreadyFlushed)await this.drainHistory(doc,false);else await this.flush(doc);
  await this.enqueue(doc,async()=>{
   if(token.isCancellationRequested) throw new vscode.CancellationError();
   if(doc.blocked || doc.pendingAuthority || doc.drafts.size || [...doc.views].some(view=>view.historyCommands.size)) throw new Error('Settle or recover all pending drafts before saving');
   const sibling=this.normal(target);
   if(sibling?.isDirty) throw new Error('Save the unsaved normal Markdown editor before saving this custom document');
   const source=doc.source, revision=doc.revision;
   const same=sameTarget;
   const other=this.resources.get(target.toString());
   if(!same && other && (other.source!==other.diskSource||other.drafts.size>0||Boolean(other.pendingAuthority))) throw new Error('Save As target has unsaved Alternative Control changes');
   if(!same) {
    await assertSeparateExportTarget(doc.uri.toString(),target.toString(),[
     ...vscode.workspace.textDocuments.filter(item=>item.isDirty).map(item=>item.uri.toString()),
     ...[...this.resources.values()].filter(item=>item.source!==item.diskSource||item.blocked||item.drafts.size>0||Boolean(item.pendingAuthority)).map(item=>item.uri.toString())]);
    let now:string|undefined;try {now=await this.readDisk(target);}catch(error){if(!(error instanceof vscode.FileSystemError && error.code==='FileNotFound'))throw error;}
    if(now!==targetBefore)throw new Error('Save As target changed while preparing the snapshot');}
   if(same && target.scheme!=='untitled' && await this.readDisk(target)!==doc.diskSource) {
    await this.block(doc,'File changed before save',await this.readDisk(target));throw new Error('File changed before save');
   }
   if(token.isCancellationRequested) throw new vscode.CancellationError();
   await vscode.workspace.fs.writeFile(target,Buffer.from(source,'utf8'));
   const actual=await this.readDisk(target);
   if(actual!==source) throw new Error('Saved file readback differs; draft retained');
   if(same && !doc.disposed) doc.diskSource=source;
   if(doc.disposed || token.isCancellationRequested) throw new vscode.CancellationError();
   // Raw draft messages can arrive during asynchronous disk I/O without advancing source/revision.
   // Keep the verified diskSource above, but never acknowledge those newer unsaved changes.
   assertSaveSnapshotCurrent(doc,source,revision);
   if(same)await this.cleanupResourceBackups(doc,source);
   assertSaveSnapshotCurrent(doc,source,revision);
  });
 }
 async saveCustomDocument(doc:AlternativeDocument,token:vscode.CancellationToken):Promise<void> { const attempt=doc.commandSaveAttempt;try {await this.persist(doc,doc.uri,token,doc.commandSave);}catch(error){if(attempt&&doc.commandSaveAttempt===attempt){attempt.failed=true;attempt.error=error;}throw error;} }
 async saveCustomDocumentAs(doc:AlternativeDocument,target:vscode.Uri,token:vscode.CancellationToken):Promise<void> { const attempt=doc.commandSaveAttempt;try {await this.persist(doc,target,token,doc.commandSave);}catch(error){if(attempt&&doc.commandSaveAttempt===attempt){attempt.failed=true;attempt.error=error;}throw error;} }
 async revertCustomDocument(doc:AlternativeDocument,token:vscode.CancellationToken):Promise<void> {
  const source=await this.readDisk(doc.uri);if(token.isCancellationRequested) throw new vscode.CancellationError();
  await this.enqueue(doc,async()=>{await this.resetAuthority(doc,()=>{doc.drafts.clear();doc.source=source;doc.diskSource=source;doc.conflictSource=undefined;doc.blocked=false;doc.checkpoint=undefined;doc.revision++;doc.historyGeneration++;});
   await this.cleanupResourceBackups(doc,source);await this.broadcast(doc,{reverted:true});});
 }
 async backupCustomDocument(doc:AlternativeDocument,_context:vscode.CustomDocumentBackupContext,token:vscode.CancellationToken):Promise<vscode.CustomDocumentBackup> {
  // Flush outside the queue; the exact capture, durable write and cleanup share its ordering.
  try {await this.flush(doc);}catch{/* Keep accepted source and distinguishable received drafts. */}
  return this.enqueue(doc,async()=>{
   if(token.isCancellationRequested||doc.disposed)throw new vscode.CancellationError();
   const data=this.recoveryData(doc);if(data.source===doc.diskSource&&!data.drafts.length)throw new vscode.CancellationError();
   const directory=this.resourceBackupDirectory(doc.uri);await vscode.workspace.fs.createDirectory(directory);
   const entries=await vscode.workspace.fs.readDirectory(directory),epoch=randomUUID(),name=resourceBackupFileName(doc.authorityEpoch,epoch);
   if(entries.filter(([entry])=>/^[0-9a-f-]{36}\.[0-9a-f-]{36}\.json(?:\.partial)?$/.test(entry)).length>=MAX_RESOURCE_BACKUPS)throw new Error('Private resource backup capacity reached; existing versions are retained');
   const target=vscode.Uri.joinPath(directory,name),temporary=vscode.Uri.joinPath(directory,name+'.partial');
   const record:ResourceBackup={kind:'terraphim-resource-backup',version:1,resource:doc.uri.toString(),authorityEpoch:doc.authorityEpoch,token:epoch,baselineHash:hash(doc.diskSource),...data};
   const source=encodeResourceBackup(record),id=target.toString();try {await vscode.workspace.fs.writeFile(temporary,Buffer.from(source,'utf8'));
   if(await this.readDisk(temporary)!==source)throw new Error('Recovery snapshot readback differs');
   if(token.isCancellationRequested)throw new vscode.CancellationError();
   await vscode.workspace.fs.rename(temporary,target,{overwrite:false});
   if(await this.readDisk(target)!==source)throw new Error('Committed recovery snapshot readback differs');doc.backups.set(id,epoch);doc.backupHashes.set(id,hash(source));
   }finally{try{await vscode.workspace.fs.delete(temporary);}catch{/* Uncommitted private partials are never admitted as backups. */}}
   if(token.isCancellationRequested)throw new vscode.CancellationError();
   return {id,delete:()=>{void this.enqueue(doc,async()=>{if(doc.backups.get(id)!==epoch)return;
    if(await this.readDisk(target)!==source||doc.backups.get(id)!==epoch)return;await vscode.workspace.fs.delete(target);doc.backups.delete(id);doc.backupHashes.delete(id);
   }).catch(()=>undefined);}};
  });
 }
 async resolveCustomEditor(doc:AlternativeDocument,panel:vscode.WebviewPanel,_token:vscode.CancellationToken):Promise<void> {
  const resource=doc.uri.toString(),view:View={session:randomUUID(),panel,lastRequest:0,epoch:0,historyCommands:new HistoryCommands(),draftStage:new AuthorityDraftStage(),stage:new CheckpointStage()};doc.views.add(view);
  const media=this.media();panel.webview.options={enableScripts:true,localResourceRoots:[media]};panel.webview.html=html(panel.webview,media);
  let disposed=false;
  const stateListener=panel.onDidChangeViewState(()=>{view.epoch++;view.historyCommands.clear();void panel.webview.postMessage({protocol:1,type:'view-state',resource,session:view.session,active:panel.active,viewEpoch:view.epoch});});
  const listener=panel.webview.onDidReceiveMessage((value:unknown)=>{
   if(!value||typeof value!=='object') return;const message=value as HostMessage;
   if(message.type==='ready') {void this.enqueue(doc,async()=>{if(doc.drafts.has(view.session)){await this.preserveAuthorityRecovery(doc);doc.blocked=true;}const authority=await this.native(doc);if(!view.nativeAttached){await authority.attachView(view.session);view.nativeAttached=true;}const snapshot=await authority.snapshot();await this.sync(doc,view,'init',{authoritySnapshot:snapshot,retainedDraftSequence:doc.drafts.get(view.session)?.sequence??0,recoveryDrafts:doc.blocked?[...doc.drafts.values()]:[]});if(doc.recoveryDirty){doc.recoveryDirty=false;this.changes.fire({document:doc});}}).catch(error=>this.block(doc,String(error)));return;}
   if(message.protocol!==1||message.resource!==resource||message.session!==view.session) return;
   if(message.type==='external-adoption-conflict'||message.type==='retained-draft-conflict') {
    void this.enqueue(doc,async()=>{
     const retainedMatches=()=>{const own=doc.drafts.get(view.session);return Boolean(own&&(message.type!=='retained-draft-conflict'||own.sequence===message.draftSequence&&hash(own.draft)===message.draftHash));};
     if(disposed||doc.disposed||message.authorityEpoch!==doc.authorityEpoch||!retainedMatches())return;
     const epoch=doc.authorityEpoch;await this.preserveAuthorityRecovery(doc);
     if(disposed||doc.disposed||epoch!==doc.authorityEpoch||!retainedMatches())return;
     const reason=message.type==='retained-draft-conflict'?'Retained view input needs explicit recovery. Open the recovery draft before choosing the current file.':'External changes raced an unsaved view draft. Open the recovery draft before choosing the current file.';
     await this.block(doc,reason);
     if(disposed||doc.disposed||epoch!==doc.authorityEpoch||!retainedMatches())return;
     await this.sync(doc,view,'resource-conflict',{reason,externalAdoptionConflict:true,...(message.type==='retained-draft-conflict'?{retainedDraftSequence:message.draftSequence,retainedDraftHash:message.draftHash}:{})});
    }).catch(error=>this.block(doc,String(error)));return;
   }
   if(message.type==='history-command-complete'){
    if(message.authorityEpoch===doc.authorityEpoch)view.historyCommands.completeHead(message.historyCommandNonce,message.viewEpoch,view.epoch);
    return;
   }
   if(message.type==='authority-draft-chunk') {
    try{const heap=getHeapStatistics(),capacity=Math.floor((heap.heap_size_limit-heap.used_heap_size)/4);
     const draft=view.draftStage.receive(message,{resource,session:view.session,authorityEpoch:doc.authorityEpoch},capacity);
     if(draft){const old=doc.drafts.get(view.session);if(old&&old.sequence>=draft.sequence)return;
      encodeRecovery(doc.source,[...[...doc.drafts.values()].filter(item=>item.viewSession!==view.session),draft]);
      doc.drafts.set(view.session,draft);this.changes.fire({document:doc});
      void panel.webview.postMessage({protocol:1,type:'authority-draft-received',resource,session:view.session,authorityEpoch:doc.authorityEpoch,draftSequence:draft.sequence});}
    }catch(error){void this.block(doc,`Pending draft could not be received: ${String(error)}`);}return;
   }
   if(message.type.startsWith('authority-')) {void this.enqueue(doc,()=>this.authorityMessage(doc,view,message)).catch(error=>{
    void panel.webview.postMessage({protocol:1,type:'authority-response',resource,session:view.session,id:message.id,ok:false,error:String(error),authorityUncertain:Boolean(doc.pendingAuthority)||doc.blocked,authorityUnchanged:Boolean(error&&typeof error==='object'&&'authorityUnchanged' in error&&error.authorityUnchanged),authorityEpoch:doc.authorityEpoch});
   });return;}
   if(message.type==='checkpoint-chunk') {try{this.stage(doc,view,message);}catch{/* The matching proposal receives the retained stage refusal. */}return;}
   if(message.type==='flush-refused' && message.nonce===view.flush?.nonce) {
    const pending=view.flush!;clearTimeout(pending.timer);view.flush=undefined;pending.reject(new Error('A view retains a conflicting draft; resolve it before saving'));return;
   }
   if(message.type==='flush-complete' && message.nonce===view.flush?.nonce) {
    const pending=view.flush!;clearTimeout(pending.timer);view.flush=undefined;
    if(message.authorityEpoch!==doc.authorityEpoch||!Number.isSafeInteger(message.revision)||!Number.isSafeInteger(message.historyGeneration)||typeof message.sourceHash!=='string'||!/^[a-f0-9]{64}$/.test(message.sourceHash))pending.reject(new Error('Invalid editor snapshot; draft retained'));else pending.resolve({authorityEpoch:message.authorityEpoch,revision:message.revision,sourceHash:message.sourceHash,historyGeneration:message.historyGeneration!});return;
   }
   if(message.type==='save') {
    // Release no source proposal until the native Save lifecycle has finished.
    // That lifecycle skips a second webview flush because this exact command
    // was already sent behind the acknowledged source/history barrier.
    if(!panel.active || !vscode.window.state.focused || message.revision!==doc.revision || message.sourceHash!==hash(doc.source) || !Number.isSafeInteger(message.id) || (message.id??0)<=view.lastRequest || doc.commandSave) {
     void this.sync(doc,view,'reject',{id:message.id,reason:'Return focus to this exact document before saving'});return;
    }
    view.lastRequest=message.id!;doc.commandSave=true;const attempt={failed:false,error:undefined as unknown};doc.commandSaveAttempt=attempt;
    // VS Code may resolve its Save command after displaying a provider error.
    // A real failed lifecycle must still reject the initiating renderer RPC.
    const finish=()=>{if(doc.commandSaveAttempt===attempt){doc.commandSaveAttempt=undefined;doc.commandSave=false;}};
    void vscode.commands.executeCommand('workbench.action.files.save').then(()=>this.sync(doc,view,attempt.failed?'reject':'command-complete',{id:message.id,...(attempt.failed?{reason:String(attempt.error)}:{})}),error=>this.sync(doc,view,'reject',{id:message.id,reason:String(error)})).then(finish,finish);return;
   }
   void this.enqueue(doc,async()=>{
    if(disposed) return;
    if(message.type==='resolve-conflict') {
     if(message.revision!==doc.revision||message.sourceHash!==hash(doc.source))throw new Error('Stale conflict recovery request');
     if(!doc.blocked)throw new Error('No recovery conflict is pending; current source and drafts are unchanged');
     if(doc.blocked) {const retainedSource=doc.source,retainedDrafts=JSON.stringify([...doc.drafts.values()]),disk=await this.readDisk(doc.uri);await this.resetAuthority(doc,()=>{if(doc.source!==retainedSource||JSON.stringify([...doc.drafts.values()])!==retainedDrafts)throw new Error('Retained input changed during recovery; all drafts remain retained. Retry explicit recovery.');doc.drafts.clear();doc.diskSource=disk;doc.source=this.normal(doc.uri)?.isDirty?this.normal(doc.uri)!.getText():disk;doc.conflictSource=undefined;doc.blocked=false;doc.checkpoint=undefined;doc.revision++;doc.historyGeneration++;});
      this.changes.fire({document:doc});await this.broadcast(doc,{reverted:true});}return;
    }
    const expected=message.type==='proposal'?message.beforeHash:message.sourceHash;
    if(message.revision!==doc.revision||expected!==hash(doc.source)) {await this.sync(doc,view,'reject',{id:message.id,reason:'Document changed before this operation'});return;}
    if(message.type==='recover-archive'){
     await this.preserveAuthorityRecovery(doc);if(doc.recoveryArchive)await vscode.window.showTextDocument(await vscode.workspace.openTextDocument(vscode.Uri.parse(doc.recoveryArchive)));return;
    }
    if(message.type==='recover-draft') {
     if(typeof message.source!=='string') return;this.checkSize(message.source);
     const recovered=await vscode.workspace.openTextDocument({language:'markdown',content:message.source});
     if(!disposed) await vscode.commands.executeCommand('vscode.openWith',recovered.uri,VIEW);return;
    }
    if(message.type==='proposal'||message.type==='history-update') {
     if(doc.authority)throw new Error('Source replacement is forbidden for resource-owned native authority');
     if(doc.blocked) throw new Error('Resolve external document conflict before editing');
     if(!Number.isSafeInteger(message.id)||(message.id??0)<=view.lastRequest||message.historyGeneration!==doc.historyGeneration) throw new Error('Stale history/source transaction');
     view.lastRequest=message.id!;
     const checkCommand=()=>{if(message.historyCommandNonce && (!panel.active || !vscode.window.state.focused || !view.historyCommands.matchesHead(message.historyCommandNonce,message.viewEpoch,view.epoch)||view.epoch!==message.viewEpoch))throw new Error('History command no longer targets the active view');};
     checkCommand();
     const source=message.type==='proposal'?message.source:doc.source;
     if(typeof source!=='string') throw new Error('Invalid source proposal');this.checkSize(source);
     if(message.type==='proposal'&&source===doc.source)throw new Error('No document change');
     if(message.type==='proposal'&&hash(source)!==message.sourceHash) throw new Error('Proposed hash mismatch');
     const projection=await checkpointProjection(source,vscode.Uri.joinPath(media,'editor').fsPath);
     const checkpoint=validateCheckpoint(this.consumeStage(doc,view,message),projection.presentationSource,projection.body,true);
     checkCommand();if(message.historyCommandNonce)view.historyCommands.completeHead(message.historyCommandNonce,message.viewEpoch,view.epoch);
     doc.checkpoint=checkpoint;doc.historyGeneration++;
     if(source!==doc.source) {doc.source=source;doc.revision++;this.changes.fire({document:doc});}
     await this.sync(doc,view,message.type==='proposal'?'ack':'history-ack',{id:message.id});
     await Promise.all([...doc.views].filter(item=>item!==view).map(item=>this.sync(doc,item,'sync')));return;
    }
    if(message.type==='open') {
     const selected=await vscode.window.showOpenDialog({canSelectMany:false,filters:{Markdown:['md']}});
     if(selected?.[0]&&!disposed) await vscode.commands.executeCommand('vscode.openWith',selected[0],VIEW);
    } else if(message.type==='export') {
     if(!Number.isSafeInteger(message.id)||(message.id??0)<=view.lastRequest) throw new Error('Invalid export request');
     view.lastRequest=message.id!;
     if(message.format==='knowledge-graph') await this.writeExport(doc,view,message,await exportKnowledgeGraph(doc.source,vscode.Uri.joinPath(media,'editor').fsPath),'knowledge-graph');
     else {view.exportPending=message.id;await panel.webview.postMessage({protocol:1,type:'export-request',id:message.id,resource,session:view.session,revision:doc.revision,sourceHash:hash(doc.source)});return;}
    } else if(message.type==='export-result'||message.type==='export-refused') {
     if(message.id!==view.exportPending)return;view.exportPending=undefined;
     if(message.type==='export-refused') throw new Error('Export snapshot changed before it could be produced');
     await this.writeExport(doc,view,message,message.source!,'markdown');
    }
    await this.sync(doc,view,'command-complete',{id:message.id});
   }).catch(error=>{void this.sync(doc,view,'reject',{id:message.id,reason:String(error)});});
  });
  panel.onDidDispose(()=>{disposed=true;view.historyCommands.clear();listener.dispose();stateListener.dispose();this.clearStage(view);view.draftStage.clear();if(view.flush){clearTimeout(view.flush.timer);view.flush.reject(new Error('View closed before snapshot completed'));}doc.views.delete(view);void this.enqueue(doc,async()=>{const pending=doc.pendingAuthority;if(pending?.view===view&&doc.authority){const status=await doc.authority.status(pending.request);
    if(status.outcome?.kind==='committed')await this.publishAuthority(doc,pending,status.outcome);
    else {const outcome=status.outcome?.kind==='guarded-aborted'?status.outcome:await doc.authority.abort(pending.request,pending.prepared.token);if(outcome.kind!=='guarded-aborted'){await this.block(doc,'Closed view left an uncertain native transaction; explicit recovery required');return;}try{await doc.authority.ackOutcome(pending.request,outcome,this.authorityBase(doc));}catch(error){const reconciled=await doc.authority.status();if(reconciled.state!=='idle'||reconciled.accepted.sourceHash!==hash(doc.source))throw error;}doc.pendingAuthority=undefined;}}
    if(view.nativeAttached)await doc.authority?.detachView(view.session);}).catch(error=>this.block(doc,String(error)));});
 }
 private async writeExport(doc:AlternativeDocument,view:View,message:HostMessage,output:string,format:'markdown'|'knowledge-graph'):Promise<void> {
  const resource=doc.uri.toString(),unchanged=()=>!doc.disposed&&doc.revision===message.revision&&hash(doc.source)===message.sourceHash;
  this.checkSize(output);if(!unchanged())throw new Error('Document changed while preparing export');
  const graph=format==='knowledge-graph';const target=await vscode.window.showSaveDialog({filters:graph?{'Knowledge graph':['json']}:{Markdown:['md']},defaultUri:doc.uri.with({path:doc.uri.path.replace(/\.md$/i,'')+(graph?'.knowledge-graph.json':'.clean.md')})});
  if(!target)return;if(!unchanged())throw new Error('Document changed while choosing export destination');
  const dirtyTargets=()=>[...vscode.workspace.textDocuments.filter(d=>d.isDirty).map(d=>d.uri.toString()),
   ...[...this.resources.values()].filter(item=>item.source!==item.diskSource||item.blocked||item.drafts.size>0||Boolean(item.pendingAuthority)).map(item=>item.uri.toString())];
  await assertSeparateExportTarget(resource,target.toString(),dirtyTargets());
  if(!unchanged())throw new Error('Document changed while checking export destination');
  await assertSeparateExportTarget(resource,target.toString(),dirtyTargets());
  if(!unchanged())throw new Error('Document changed before export write');
  await vscode.workspace.fs.writeFile(target,Buffer.from(output,'utf8'));
  if(await this.readDisk(target)!==output)throw new Error('Export readback differs');
  await vscode.window.showTextDocument(await vscode.workspace.openTextDocument(target));
 }
}
function escape(value: string): string {
  return value.replace(/[&<>"']/g, c => ({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]!));
}
function html(webview: vscode.Webview, media: vscode.Uri): string {
  const assets = webview.asWebviewUri(vscode.Uri.joinPath(media,'editor')).toString() + '/';
  const script = webview.asWebviewUri(vscode.Uri.joinPath(media,'alternative-control.js')).toString();
  const hostStyle = webview.asWebviewUri(vscode.Uri.joinPath(media,'alternative-control.css')).toString();
  const historyCheckpoint = webview.asWebviewUri(vscode.Uri.joinPath(media,'history-checkpoint.js')).toString();
  const hostNotice = webview.asWebviewUri(vscode.Uri.joinPath(media,'host-notice.js')).toString();
  const intent = webview.asWebviewUri(vscode.Uri.joinPath(media,'export-intent.js')).toString();
  const authoritySnapshot=webview.asWebviewUri(vscode.Uri.joinPath(media,'authority-snapshot.js')).toString();
  const nativeAuthority=webview.asWebviewUri(vscode.Uri.joinPath(media,'native-authority.js')).toString();
  const nonce = randomUUID();
  return `<!DOCTYPE html><html><head><meta charset="utf-8">
<meta http-equiv="Content-Security-Policy" content="default-src 'none'; img-src ${webview.cspSource} data:; font-src ${webview.cspSource} data:; style-src ${webview.cspSource} 'unsafe-inline'; script-src ${webview.cspSource} 'nonce-${nonce}' 'wasm-unsafe-eval'; connect-src ${webview.cspSource}; worker-src ${webview.cspSource} blob:;">
<link rel="stylesheet" href="${escape(assets)}terraphim-editor.min.css">
<link rel="stylesheet" href="${escape(hostStyle)}">
</head><body data-authority="true" data-assets="${escape(assets)}"><div id="host-notice" role="alert" hidden></div><div id="editor"></div>
<script nonce="${nonce}" src="${escape(assets)}terraphim-editor.min.js"></script>
<script nonce="${nonce}" src="${escape(assets)}host-session.js"></script>
<script nonce="${nonce}" src="${escape(intent)}"></script>
<script nonce="${nonce}" src="${escape(hostNotice)}"></script>
<script nonce="${nonce}" src="${escape(historyCheckpoint)}"></script>
<script nonce="${nonce}" src="${escape(authoritySnapshot)}"></script>
<script nonce="${nonce}" src="${escape(nativeAuthority)}"></script><script nonce="${nonce}" src="${escape(script)}"></script></body></html>`;
}
