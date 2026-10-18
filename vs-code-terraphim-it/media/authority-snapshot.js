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
 /** Let browser text selection finish synchronously without forwarding it twice. */
 function consumeEditableSelectAll(event,container){
  if(event.defaultPrevented||event.isComposing||event.altKey||event.shiftKey||event.ctrlKey===event.metaKey||event.key?.toLowerCase()!=='a')return false;
  const route=event.composedPath();if(!route.includes(container))return false;
  const target=route.find(node=>node instanceof HTMLTextAreaElement||node instanceof HTMLInputElement);
  if(!target||target.disabled||target.readOnly||target instanceof HTMLInputElement&&!['text','search','url','tel','email','password'].includes(target.type))return false;
  event.stopPropagation();return true;
 }
 return {settleSnapshot,refreshHash,isAdmissible:admitted,rebaseConflict,isSameAccepted:sameAccepted,consumeEditableSelectAll};
});
