import * as vscode from 'vscode';
import { createHash, randomUUID } from 'crypto';
import { exportKnowledgeGraph, checkpointProjection } from '../media/knowledge-graph.cjs';
import { getHeapStatistics } from 'v8';
import { CheckpointStage } from './checkpointStage';
import { validateCheckpoint, chunks } from '../media/history-checkpoint';
import { assertSeparateExportTarget } from '../media/export-target.cjs';
const VIEW = 'terraphimIt.alternativeControl';
const LIMIT = 16 * 1024 * 1024;
const hash = (source: string) => createHash('sha256').update(source, 'utf8').digest('hex');
interface HostMessage {
  protocol: number; type: string; resource: string; session: string; revision: number;
  id?: number; beforeHash?: string; sourceHash?: string; source?: string;
  format?: 'markdown' | 'knowledge-graph'; checkpoint?: unknown; historyGeneration?: number; nonce?: string; checkpointToken?:string; token?:string; index?:number; total?:number; bytes?:number; text?:string; historyCommandNonce?:string; viewEpoch?:number;
}
interface View {
  session: string; panel: vscode.WebviewPanel; lastRequest: number; exportPending?: number; stage:CheckpointStage; epoch:number; historyCommand?:{nonce:string;epoch:number}; sending?:Promise<boolean>;
  flush?: {nonce:string;resolve:()=>void;reject:(error:Error)=>void;timer:ReturnType<typeof setTimeout>};
}
class AlternativeDocument implements vscode.CustomDocument {
  readonly views = new Set<View>();
  queue: Promise<unknown> = Promise.resolve();
  revision=1; historyGeneration=0; checkpoint:unknown; conflictSource?:string;
  disposed=false; commandSave=false; blocked=false; readonly backups=new Map<string,string>();
  constructor(readonly uri:vscode.Uri, public source:string, public diskSource:string,private readonly onDispose:()=>void) {}
  dispose():void { this.disposed=true;this.onDispose(); for (const view of this.views) {
    view.stage.clear();
    if (view.flush) { clearTimeout(view.flush.timer);view.flush.reject(new Error('Document disposed')); }
  } this.views.clear(); }
}
export function registerAlternativeControl(context:vscode.ExtensionContext):void {
 const provider=new AlternativeControlProvider(context.extensionUri);
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
class AlternativeControlProvider implements vscode.CustomEditorProvider<AlternativeDocument>,vscode.Disposable {
 private readonly resources=new Map<string,AlternativeDocument>();
 private readonly changes=new vscode.EventEmitter<vscode.CustomDocumentContentChangeEvent<AlternativeDocument>>();
 readonly onDidChangeCustomDocument=this.changes.event;
 private readonly subscriptions:vscode.Disposable[]=[];
 constructor(private readonly extensionUri:vscode.Uri) {
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
  const source=doc.source,revision=doc.revision,historyGeneration=doc.historyGeneration,checkpoint=doc.checkpoint;
  const send=async()=>{
   if(['reject','resource-conflict','sync','init'].includes(type))this.clearStage(view);
   const token=checkpoint?randomUUID():undefined;
   if(checkpoint) {
    const encoded=chunks(checkpoint);
    for(let index=0;index<encoded.parts.length;index++) await view.panel.webview.postMessage({protocol:1,type:'checkpoint-chunk',resource:doc.uri.toString(),session:view.session,
     revision,sourceHash:hash(source),historyGeneration,token,index,total:encoded.parts.length,bytes:encoded.bytes,text:encoded.parts[index]});
   }
   return view.panel.webview.postMessage({protocol:1,type,resource:doc.uri.toString(),session:view.session,
    revision,source,checkpointToken:token,historyGeneration,active:view.panel.active,viewEpoch:view.epoch,...extra});
  };
  const result=(view.sending??Promise.resolve(true)).then(send);view.sending=result.catch(()=>false);return result;
 }
 private stageIdentity(doc:AlternativeDocument,view:View) {
  return {resource:doc.uri.toString(),session:view.session,revision:doc.revision,sourceHash:hash(doc.source),historyGeneration:doc.historyGeneration};
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
   if(doc.source!==doc.diskSource || this.normal(doc.uri)?.isDirty) {
    await this.block(doc,'External document changes need an explicit recovery choice',source);return;
   }
   this.checkSize(source);doc.source=source;doc.diskSource=source;doc.checkpoint=undefined;
   doc.revision++;doc.historyGeneration++;await this.broadcast(doc);
  });
 }
 async openCustomDocument(uri:vscode.Uri,options:vscode.CustomDocumentOpenContext,_token:vscode.CancellationToken):Promise<AlternativeDocument> {
  const disk=await this.readDisk(uri);
  const source=options.backupId ? await this.readDisk(vscode.Uri.parse(options.backupId)) : this.normal(uri)?.getText()??disk;
  this.checkSize(source);const doc=new AlternativeDocument(uri,source,disk,()=>{if(this.resources.get(uri.toString())===doc)this.resources.delete(uri.toString());});this.resources.set(uri.toString(),doc);return doc;
 }
 async activeCommand(type:string,command?:string):Promise<void> {
  for (const doc of this.resources.values()) for(const view of doc.views) if(view.panel.active && vscode.window.state.focused) {
   const nonce=type==='request-history'?randomUUID():undefined;
   if(nonce)view.historyCommand={nonce,epoch:view.epoch};
   await view.panel.webview.postMessage({protocol:1,type,resource:doc.uri.toString(),session:view.session,command,historyCommandNonce:nonce,viewEpoch:view.epoch,format:'knowledge-graph'});return;
  }
  await vscode.window.showInformationMessage('Open and focus Terraphim Alternative Control first.');
 }
 private async flush(doc:AlternativeDocument):Promise<void> {
  await Promise.all([...doc.views].map(view=>{
  if(view.flush) return Promise.reject(new Error('A document snapshot is already pending'));
  return new Promise<void>((resolve,reject)=>{
   const nonce=randomUUID();const timer=setTimeout(()=>{view.flush=undefined;reject(new Error('Editor snapshot did not settle; draft retained'));},15000);
   view.flush={nonce,resolve,reject,timer};void view.panel.webview.postMessage({protocol:1,type:'flush-request',resource:doc.uri.toString(),session:view.session,nonce});
  });}));
 }
 private async persist(doc:AlternativeDocument,target:vscode.Uri,token:vscode.CancellationToken,alreadyFlushed=false):Promise<void> {
  const sameTarget=target.toString()===doc.uri.toString();
  let targetBefore:string|undefined;
  if(!sameTarget) { try {targetBefore=await this.readDisk(target);} catch(error) {if(!(error instanceof vscode.FileSystemError && error.code==='FileNotFound'))throw error;} }
  if(!alreadyFlushed) await this.flush(doc);
  await this.enqueue(doc,async()=>{
   if(token.isCancellationRequested) throw new vscode.CancellationError();
   if(doc.blocked) throw new Error('Resolve external document conflict before saving');
   const sibling=this.normal(target);
   if(sibling?.isDirty) throw new Error('Save the unsaved normal Markdown editor before saving this custom document');
   const source=doc.source, revision=doc.revision;
   const same=sameTarget;
   const other=this.resources.get(target.toString());
   if(!same && other && other.source!==other.diskSource) throw new Error('Save As target has unsaved Alternative Control changes');
   if(!same) {
    await assertSeparateExportTarget(doc.uri.toString(),target.toString(),[
     ...vscode.workspace.textDocuments.filter(item=>item.isDirty).map(item=>item.uri.toString()),
     ...[...this.resources.values()].filter(item=>item.source!==item.diskSource||item.blocked).map(item=>item.uri.toString())]);
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
   // A serialized save cannot acknowledge a later revision or clear its draft.
   if(doc.revision!==revision || doc.source!==source) throw new Error('Newer source remains unsaved');
  });
 }
 async saveCustomDocument(doc:AlternativeDocument,token:vscode.CancellationToken):Promise<void> { await this.persist(doc,doc.uri,token,doc.commandSave); }
 async saveCustomDocumentAs(doc:AlternativeDocument,target:vscode.Uri,token:vscode.CancellationToken):Promise<void> { await this.persist(doc,target,token); }
 async revertCustomDocument(doc:AlternativeDocument,token:vscode.CancellationToken):Promise<void> {
  const source=await this.readDisk(doc.uri);if(token.isCancellationRequested) throw new vscode.CancellationError();
  await this.enqueue(doc,async()=>{doc.source=source;doc.diskSource=source;doc.conflictSource=undefined;doc.blocked=false;doc.checkpoint=undefined;
   doc.revision++;doc.historyGeneration++;await this.broadcast(doc,{reverted:true});});
 }
 async backupCustomDocument(doc:AlternativeDocument,context:vscode.CustomDocumentBackupContext,token:vscode.CancellationToken):Promise<vscode.CustomDocumentBackup> {
  await this.flush(doc);
  const source=await this.enqueue(doc,async()=>doc.source);
  if(token.isCancellationRequested) throw new vscode.CancellationError();
  await vscode.workspace.fs.writeFile(context.destination,Buffer.from(source,'utf8'));
  if(await this.readDisk(context.destination)!==source) throw new Error('Recovery snapshot readback differs');
  if(token.isCancellationRequested) throw new vscode.CancellationError();
  const id=context.destination.toString(),epoch=randomUUID();doc.backups.set(id,epoch);
  return {id,delete:()=>{void (async()=>{if(doc.backups.get(id)!==epoch)return;
   if(await this.readDisk(context.destination)!==source)return;
   if(doc.backups.get(id)!==epoch)return;doc.backups.delete(id);await vscode.workspace.fs.delete(context.destination);})().catch(()=>undefined);}};
 }
 async resolveCustomEditor(doc:AlternativeDocument,panel:vscode.WebviewPanel,_token:vscode.CancellationToken):Promise<void> {
  const resource=doc.uri.toString(),view:View={session:randomUUID(),panel,lastRequest:0,epoch:0,stage:new CheckpointStage()};doc.views.add(view);
  const media=this.media();panel.webview.options={enableScripts:true,localResourceRoots:[media]};panel.webview.html=html(panel.webview,media);
  let disposed=false;
  const stateListener=panel.onDidChangeViewState(()=>{view.epoch++;view.historyCommand=undefined;void panel.webview.postMessage({protocol:1,type:'view-state',resource,session:view.session,active:panel.active,viewEpoch:view.epoch});});
  const listener=panel.webview.onDidReceiveMessage((value:unknown)=>{
   if(!value||typeof value!=='object') return;const message=value as HostMessage;
   if(message.type==='ready') {void this.sync(doc,view,'init');return;}
   if(message.protocol!==1||message.resource!==resource||message.session!==view.session) return;
   if(message.type==='checkpoint-chunk') {try{this.stage(doc,view,message);}catch{/* The matching proposal receives the retained stage refusal. */}return;}
   if(message.type==='flush-refused' && message.nonce===view.flush?.nonce) {
    const pending=view.flush!;clearTimeout(pending.timer);view.flush=undefined;pending.reject(new Error('A view retains a conflicting draft; resolve it before saving'));return;
   }
   if(message.type==='flush-complete' && message.nonce===view.flush?.nonce) {
    const pending=view.flush!;clearTimeout(pending.timer);view.flush=undefined;
    if(message.revision===doc.revision&&message.sourceHash===hash(doc.source)&&message.historyGeneration===doc.historyGeneration) pending.resolve();else pending.reject(new Error('Snapshot changed before save'));return;
   }
   if(message.type==='save') {
    // Release no source proposal until the native Save lifecycle has finished.
    // That lifecycle skips a second webview flush because this exact command
    // was already sent behind the acknowledged source/history barrier.
    if(!panel.active || !vscode.window.state.focused || message.revision!==doc.revision || message.sourceHash!==hash(doc.source) || !Number.isSafeInteger(message.id) || (message.id??0)<=view.lastRequest || doc.commandSave) {
     void this.sync(doc,view,'reject',{id:message.id,reason:'Return focus to this exact document before saving'});return;
    }
    view.lastRequest=message.id!;doc.commandSave=true;
    void vscode.commands.executeCommand('workbench.action.files.save').then(()=>this.sync(doc,view,'command-complete',{id:message.id}),error=>this.sync(doc,view,'reject',{id:message.id,reason:String(error)})).then(()=>{doc.commandSave=false;},()=>{doc.commandSave=false;});return;
   }
   void this.enqueue(doc,async()=>{
    if(disposed) return;
    if(message.type==='resolve-conflict') {
     if(message.revision!==doc.revision||message.sourceHash!==hash(doc.source))throw new Error('Stale conflict recovery request');
     if(doc.blocked) {const disk=await this.readDisk(doc.uri);doc.diskSource=disk;doc.source=this.normal(doc.uri)?.isDirty?this.normal(doc.uri)!.getText():disk;doc.conflictSource=undefined;doc.blocked=false;doc.checkpoint=undefined;
      doc.revision++;doc.historyGeneration++;this.changes.fire({document:doc});await this.broadcast(doc,{reverted:true});}return;
    }
    const expected=message.type==='proposal'?message.beforeHash:message.sourceHash;
    if(message.revision!==doc.revision||expected!==hash(doc.source)) {await this.sync(doc,view,'reject',{id:message.id,reason:'Document changed before this operation'});return;}
    if(message.type==='recover-draft') {
     if(typeof message.source!=='string') return;this.checkSize(message.source);
     const recovered=await vscode.workspace.openTextDocument({language:'markdown',content:message.source});
     if(!disposed) await vscode.commands.executeCommand('vscode.openWith',recovered.uri,VIEW);return;
    }
    if(message.type==='proposal'||message.type==='history-update') {
     if(doc.blocked) throw new Error('Resolve external document conflict before editing');
     if(!Number.isSafeInteger(message.id)||(message.id??0)<=view.lastRequest||message.historyGeneration!==doc.historyGeneration) throw new Error('Stale history/source transaction');
     view.lastRequest=message.id!;
     const checkCommand=()=>{if(message.historyCommandNonce && (!panel.active || !vscode.window.state.focused || !view.historyCommand
      ||view.historyCommand.nonce!==message.historyCommandNonce||view.historyCommand.epoch!==message.viewEpoch||view.epoch!==message.viewEpoch))throw new Error('History command no longer targets the active view');};
     checkCommand();
     const source=message.type==='proposal'?message.source:doc.source;
     if(typeof source!=='string') throw new Error('Invalid source proposal');this.checkSize(source);
     if(message.type==='proposal'&&source===doc.source)throw new Error('No document change');
     if(message.type==='proposal'&&hash(source)!==message.sourceHash) throw new Error('Proposed hash mismatch');
     const projection=await checkpointProjection(source,vscode.Uri.joinPath(media,'editor').fsPath);
     const checkpoint=validateCheckpoint(this.consumeStage(doc,view,message),projection.presentationSource,projection.body,true);
     checkCommand();if(message.historyCommandNonce)view.historyCommand=undefined;
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
  panel.onDidDispose(()=>{disposed=true;listener.dispose();stateListener.dispose();this.clearStage(view);if(view.flush){clearTimeout(view.flush.timer);view.flush.reject(new Error('View closed before snapshot completed'));}doc.views.delete(view);});
 }
 private async writeExport(doc:AlternativeDocument,view:View,message:HostMessage,output:string,format:'markdown'|'knowledge-graph'):Promise<void> {
  const resource=doc.uri.toString(),unchanged=()=>!doc.disposed&&doc.revision===message.revision&&hash(doc.source)===message.sourceHash;
  this.checkSize(output);if(!unchanged())throw new Error('Document changed while preparing export');
  const graph=format==='knowledge-graph';const target=await vscode.window.showSaveDialog({filters:graph?{'Knowledge graph':['json']}:{Markdown:['md']},defaultUri:doc.uri.with({path:doc.uri.path.replace(/\.md$/i,'')+(graph?'.knowledge-graph.json':'.clean.md')})});
  if(!target)return;if(!unchanged())throw new Error('Document changed while choosing export destination');
  const dirtyTargets=()=>[...vscode.workspace.textDocuments.filter(d=>d.isDirty).map(d=>d.uri.toString()),
   ...[...this.resources.values()].filter(item=>item.source!==item.diskSource||item.blocked).map(item=>item.uri.toString())];
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
  const nonce = randomUUID();
  return `<!DOCTYPE html><html><head><meta charset="utf-8">
<meta http-equiv="Content-Security-Policy" content="default-src 'none'; img-src ${webview.cspSource} data:; font-src ${webview.cspSource} data:; style-src ${webview.cspSource} 'unsafe-inline'; script-src ${webview.cspSource} 'nonce-${nonce}' 'wasm-unsafe-eval'; connect-src ${webview.cspSource}; worker-src ${webview.cspSource} blob:;">
<link rel="stylesheet" href="${escape(assets)}terraphim-editor.min.css">
<link rel="stylesheet" href="${escape(hostStyle)}">
</head><body data-assets="${escape(assets)}"><div id="host-notice" role="alert" hidden></div><div id="editor"></div>
<script nonce="${nonce}" src="${escape(assets)}terraphim-editor.min.js"></script>
<script nonce="${nonce}" src="${escape(assets)}host-session.js"></script>
<script nonce="${nonce}" src="${escape(intent)}"></script>
<script nonce="${nonce}" src="${escape(hostNotice)}"></script>
<script nonce="${nonce}" src="${escape(historyCheckpoint)}"></script>
<script nonce="${nonce}" src="${escape(script)}"></script></body></html>`;
}
