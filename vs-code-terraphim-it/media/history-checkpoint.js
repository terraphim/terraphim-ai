/* Transport of the EXISTING EditorSurface history, never another replay engine. */
(function(root) {
 'use strict';
 const LIMIT=16*1024*1024;
 const HISTORY_LIMIT=500;
 const CHUNK_UNITS=256*1024;
 function encodedSize(value,limit=LIMIT) {
  let bytes=0;const seen=new WeakSet();
  const add=n=>{bytes+=n;if(bytes>limit)throw new Error('Checkpoint exceeds source/message limit');};
  function text(value) {
   add(2);
   for(let i=0;i<value.length;i++) {
    const c=value.charCodeAt(i);
    if(c===34||c===92||c===8||c===9||c===10||c===12||c===13)add(2);
    else if(c<32)add(6);
    else if(c<128)add(1);
    else if(c<2048)add(2);
    else if(c>=0xd800&&c<=0xdbff&&value.charCodeAt(i+1)>=0xdc00&&value.charCodeAt(i+1)<=0xdfff){add(4);i++;}
    else if(c>=0xd800&&c<=0xdfff)add(6);
    else add(3);
   }
  }
  function visit(item,depth=0) {
   if(depth>64)throw new Error('Checkpoint JSON nesting limit');
   if(item===null){add(4);return;}
   if(typeof item==='string'){text(item);return;}
   if(typeof item==='number'&&Number.isFinite(item)){add(JSON.stringify(item).length);return;}
   if(typeof item==='boolean'){add(item?4:5);return;}
   if(!item||typeof item!=='object')throw new Error('Checkpoint must be finite JSON data');
   if(seen.has(item))throw new Error('Checkpoint JSON cycle');seen.add(item);add(2);
   const entries=Array.isArray(item)?item.map((value,index)=>[index,value]):Object.entries(item);
   entries.forEach(([key,child],index)=>{if(index)add(1);if(!Array.isArray(item)){text(key);add(1);}visit(child,depth+1);});
   seen.delete(item);
  }
  visit(value);return bytes;
 }
 function aggregateLimit(value) {
  if(!Number.isSafeInteger(value?.historyLimit)||value.historyLimit<1||value.historyLimit>HISTORY_LIMIT)throw new Error('Invalid history limit');
  if(!Array.isArray(value.history)||value.history.length>value.historyLimit+1)throw new Error('Existing history entry limit exceeded');
  return (value.history.length+2)*LIMIT;
 }
 function copy(value,limit) {encodedSize(value,limit);return JSON.parse(JSON.stringify(value));}
 function validateCheckpoint(value,presentationSource,body,large=false) {
  if (!value || value.schema!==1) throw new Error('Invalid checkpoint schema');
  if (!Number.isSafeInteger(value.historyLimit) || value.historyLimit<1) throw new Error('Invalid history limit');
  const saved=copy(value,large?aggregateLimit(value):LIMIT);
  if (saved.presentationSource!==presentationSource) throw new Error('Checkpoint source mismatch');
  if (!Array.isArray(saved.history) || !saved.history.length || !Number.isSafeInteger(saved.historyIndex) || saved.historyIndex<0 || saved.historyIndex>=saved.history.length) throw new Error('Invalid history index');
  const coordinate=value=>Number.isSafeInteger(value)&&value>=0&&value<=0xffffffff;
  const fields={ghost:['change','undo'],alt:['span','before','after'],move:['start','end','to'],swap:['span','from','to'],overflow:['before','after']};
  for (const entry of saved.history) {
   if (!entry || typeof entry.text!=='string' || typeof entry.source!=='string' || !Number.isFinite(entry.time)
    || !Number.isSafeInteger(entry.start) || !Number.isSafeInteger(entry.end) || entry.start<0 || entry.end<entry.start || entry.end>entry.text.length
    || !(entry.edits===null || Array.isArray(entry.edits))) throw new Error('Invalid existing history entry');
   for(const step of entry.edits||[]) {
    if(!step || typeof step!=='object' || !coordinate(step.start) || typeof step.deletedText!=='string' || typeof step.insertedText!=='string'
      || Object.keys(step).some(key=>!['start','deletedText','insertedText',...Object.keys(fields)].includes(key))
      || Object.keys(fields).filter(key=>step[key]).length>1)throw new Error('Invalid existing history step');
    for(const [key,required] of Object.entries(fields)) if(step[key]) {
     const meta=step[key];if(typeof meta!=='object'||Object.keys(meta).length!==required.length||required.some(field=>!Object.hasOwn(meta,field)))throw new Error('Invalid existing history metadata');
     if(key==='move' && !required.every(field=>coordinate(meta[field])))throw new Error('Invalid history move');
     if((key==='alt'||key==='swap')&&typeof meta.span!=='string')throw new Error('Invalid history span');
     if(key==='swap'&&(!Number.isSafeInteger(meta.from)||!Number.isSafeInteger(meta.to)||meta.from<0||meta.to<0))throw new Error('Invalid history swap');
     if(key==='overflow'&&(typeof meta.before!=='string'||typeof meta.after!=='string'))throw new Error('Invalid history overflow');
     if(key==='ghost'&&(typeof meta.undo!=='boolean'||!meta.change||typeof meta.change!=='object'||step.deletedText||step.insertedText))throw new Error('Invalid history ghost receipt');
     if(key==='alt'&&[meta.before,meta.after].some(value=>value!==null && (typeof value!=='object'||Array.isArray(value))))throw new Error('Invalid history alternative');
    }
   }
  }
  if (saved.history[saved.historyIndex].text!==body) throw new Error('Checkpoint body mismatch');
  return saved;
 }
 function capture(surface,presentationSource,panel) {
  let history=structuredClone(surface.history), index=surface.historyIndex;
  // A live addition's transported suffix is one action. The origin retains
  // its own base reference and continues to fold/cancel in the existing UI.
  if (panel?.live) {
   const base=surface.history.indexOf(panel.live.base), count=index-base;
   if (base<0) throw new Error('Live history base is unavailable');
   if (count>1) {
    const group=history.slice(base+1,index+1);
    if (group.some(entry=>!entry.edits)) throw new Error('Live history cannot be transported losslessly');
    const merged={...group.at(-1),source:'alternatives',edits:group.flatMap(entry=>entry.edits)};
    if (group.some(entry=>entry.ghostHistory || entry.edits.some(step=>step.ghost))) merged.ghostHistory=true;
    history.splice(base+1,count,merged); index=base+1;
   }
  }
  return validateCheckpoint({schema:1,presentationSource,history,historyIndex:index,historyLimit:panel?.live?.limit || surface.historyLimit},presentationSource,surface.getText(),true);
 }
 function restore(surface,saved,presentationSource,owner) {
  const valid=validateCheckpoint(saved,presentationSource,surface.getText(),true);
  if(valid.history.some(entry=>entry.ghostHistory || entry.edits?.some(step=>step.ghost))) {
   if(!owner || typeof owner.replayGhostHistory!=='function' || typeof owner.ghostHistoryNotice!=='function')throw new Error('Existing MarkdownEditor ghost history owner is required');
   surface.ghostHistoryAdapter=owner;
  }
  surface.history=valid.history;surface.historyIndex=valid.historyIndex;surface.historyLimit=valid.historyLimit;
 }
 function chunks(value) {
  const bytes=encodedSize(value,aggregateLimit(value));const json=JSON.stringify(value),parts=[];
  for(let i=0;i<json.length;i+=CHUNK_UNITS)parts.push(json.slice(i,i+CHUNK_UNITS));
  return {bytes,parts};
 }
 const api={validateCheckpoint,capture,restore,encodedSize,chunks,LIMIT,HISTORY_LIMIT,CHUNK_UNITS};
 if (typeof module==='object' && module.exports) module.exports=api;
 else root.TerraphimHistoryCheckpoint=api;
})(typeof window==='object'?window:globalThis);
