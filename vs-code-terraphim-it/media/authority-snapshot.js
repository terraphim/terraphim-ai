/* Async mirror admission; this grants no native mutation or receipt authority. */
(function(root,factory){
 'use strict';
 const api=Object.freeze(factory());
 if(typeof module==='object'&&module.exports)module.exports=api;
 else root.TerraphimAuthoritySnapshot=api;
})(globalThis,()=>{
 'use strict';
 const digest=async source=>[...new Uint8Array(await crypto.subtle.digest('SHA-256',new TextEncoder().encode(source)))].map(b=>b.toString(16).padStart(2,'0')).join('');
 function admitted(candidate,current,epoch){
  return current.authorityEpoch===epoch&&candidate.authorityEpoch===epoch
   &&candidate.resource===current.resource&&candidate.session===current.session
   &&Number.isSafeInteger(candidate.revision)&&candidate.revision>=current.revision
   &&Number.isSafeInteger(candidate.historyGeneration)&&candidate.historyGeneration>=current.historyGeneration
   &&(candidate.revision!==current.revision||candidate.historyGeneration!==current.historyGeneration||candidate.sourceHash===current.sourceHash);
 }
 /** Revalidate after every asynchronous boundary, before the caller's synchronous adoption. */
 async function settleSnapshot(message,current,settle,hasLocalDraft=()=>false){
  const candidate={...message},epoch=current().authorityEpoch;
  await settle();if(!admitted(candidate,current(),epoch))return null;
  if(typeof candidate.source!=='string'||typeof candidate.sourceHash!=='string')throw new Error('Invalid mirror source hash');
  const actual=await digest(candidate.source);
  if(!admitted(candidate,current(),epoch))return null;
  if(actual!==candidate.sourceHash)throw new Error('Mirror source hash mismatch');
  if(!sameAccepted(candidate,current())&&hasLocalDraft())throw Object.assign(new Error('Another view changed the document while this view retains a raw draft'),{mirrorDraftConflict:true,candidate});
  return candidate;
 }
 /** An own acknowledged broadcast must not repaint an independently typed raw draft. */
 function sameAccepted(candidate,current){
  return admitted(candidate,current,current.authorityEpoch)&&candidate.revision===current.revision
   &&candidate.historyGeneration===current.historyGeneration&&candidate.sourceHash===current.sourceHash
   &&candidate.source===current.source;
 }
 /** Hash only the captured accepted object; a retired continuation cannot change a new base. */
 async function refreshHash(current){
  const captured=current(),source=captured.source,epoch=captured.authorityEpoch;
  const actual=await digest(source);
  if(current()!==captured||captured.source!==source||captured.authorityEpoch!==epoch)return false;
  captured.sourceHash=actual;return captured;
 }
 /** Refresh recovery transport metadata without replacing the held projection or UI history. */
 function rebaseConflict(message,current){
  if(!message.externalAdoptionConflict||!admitted(message,current,current.authorityEpoch)
   ||typeof message.source!=='string'||typeof message.sourceHash!=='string'||!/^[0-9a-f]{64}$/.test(message.sourceHash))return current;
  return {...current,source:message.source,sourceHash:message.sourceHash,revision:message.revision,historyGeneration:message.historyGeneration};
 }
 /** Classify persisted view input without replaying it or inferring that stale metadata is empty. */
 function retainedDraft(state,resource){
  if(state==null||typeof state==='object'&&!Object.prototype.hasOwnProperty.call(state,'draft'))return {kind:'none'};
  const object=value=>value!==null&&typeof value==='object'&&!Array.isArray(value);
  const keys=(value,allowed)=>object(value)&&Object.keys(value).every(key=>allowed.includes(key));
  if(!keys(state,['resource','sequence','draft'])||typeof state.resource!=='string'||!state.resource
   ||!Number.isSafeInteger(state.sequence)||state.sequence<1||typeof state.draft!=='string'
   ||new TextEncoder().encode(state.draft).length>128*1024*1024)return {kind:'invalid'};
  if(state.resource!==resource)return {kind:'foreign',resource:state.resource};
  let value;try{value=JSON.parse(state.draft);}catch{return {kind:'invalid'};}
  function valid(value,depth=0){
   if(!object(value)||depth>1)return false;
   if(value.kind==='external-adoption-conflict'||value.kind==='remote-view-conflict')return keys(value,['kind','previousAcceptedSource','rawDraft'])&&typeof value.previousAcceptedSource==='string'&&valid(value.rawDraft,depth+1);
   return keys(value,['baseSourceHash','pending','body','alternatives','overflow'])&&typeof value.baseSourceHash==='string'&&/^[a-f0-9]{64}$/.test(value.baseSourceHash)
    &&(value.body===undefined||typeof value.body==='string')&&(value.overflow===undefined||typeof value.overflow==='string')
    &&(value.pending===undefined||keys(value.pending,['sequence','intents'])&&Number.isSafeInteger(value.pending.sequence)&&value.pending.sequence>=0&&Array.isArray(value.pending.intents)&&value.pending.intents.every(object))
    &&(value.alternatives===undefined||Array.isArray(value.alternatives)&&value.alternatives.every(item=>keys(item,['value','committed'])&&typeof item.value==='string'&&typeof item.committed==='string'));
  }
  return valid(value)?{kind:'retained',draft:state.draft,sequence:state.sequence}:{kind:'invalid'};
 }
 /** Let browser text selection finish synchronously without forwarding it twice. */
 function consumeEditableSelectAll(event,container){
  if(event.defaultPrevented||event.isComposing||event.altKey||event.shiftKey||event.ctrlKey===event.metaKey||event.key?.toLowerCase()!=='a')return false;
  const route=event.composedPath();if(!route.includes(container))return false;
  const target=route.find(node=>node instanceof HTMLTextAreaElement||node instanceof HTMLInputElement);
  if(!target||target.disabled||target.readOnly||target instanceof HTMLInputElement&&!['text','search','url','tel','email','password'].includes(target.type))return false;
  event.stopPropagation();return true;
 }
 return {retainedDraft,settleSnapshot,refreshHash,isAdmissible:admitted,rebaseConflict,isSameAccepted:sameAccepted,consumeEditableSelectAll};
});
