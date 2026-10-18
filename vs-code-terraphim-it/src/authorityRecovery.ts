/** Private recovery storage; annotated Markdown remains the main-file format. */
export interface AuthorityDraft { viewSession:string; sequence:number; draft:string; }
const KIND='terraphim-native-authority-backup';
const LIMIT=16*1024*1024;
// Raw intents can require six JSON bytes per admitted UTF-8 byte. Individual IPC chunks stay bounded.
export const MAX_DRAFT_BYTES=128*1024*1024;
function text(value:unknown):value is string {return typeof value==='string'&&Buffer.byteLength(value,'utf8')<=LIMIT;}
function validDrafts(value:unknown):value is AuthorityDraft[] {
 if(!Array.isArray(value)||value.length>128)return false;
 const sessions=new Set<string>();let bytes=0;
 for(const item of value){
  if(!item||typeof item!=='object'||Object.keys(item).sort().join(',')!=='draft,sequence,viewSession'
   ||typeof item.viewSession!=='string'||!item.viewSession||item.viewSession.length>256
   ||!Number.isSafeInteger(item.sequence)||item.sequence<1||typeof item.draft!=='string'||Buffer.byteLength(item.draft,'utf8')>MAX_DRAFT_BYTES||sessions.has(item.viewSession))return false;
  sessions.add(item.viewSession);bytes+=Buffer.byteLength(item.draft,'utf8');if(bytes>MAX_DRAFT_BYTES)return false;
 }return true;
}
export function validateRecoveryData(source:unknown,drafts:unknown):void {
 if(!text(source)||!validDrafts(drafts))throw new Error('Recovery snapshot exceeds bounds or has invalid draft identities');
}
/** Encode accepted source and pending drafts, without implying a main-file save. */
export function encodeRecovery(source:string,drafts:AuthorityDraft[]):string {
 validateRecoveryData(source,drafts);
 return JSON.stringify({kind:KIND,version:1,source,drafts});
}
/** Read a private versioned backup or preserve a legacy plain-Markdown backup. */
export function decodeRecovery(raw:string):{source:string;drafts:AuthorityDraft[]} {
 if(typeof raw!=='string'||Buffer.byteLength(raw,'utf8')>32*LIMIT)throw new Error('Recovery snapshot exceeds bounds');
 let value:unknown;try{value=JSON.parse(raw);}catch{return {source:raw,drafts:[]};}
 if(!value||typeof value!=='object'||!('kind' in value)||value.kind!==KIND)return {source:raw,drafts:[]};
 const record=value as Record<string,unknown>;
 if(Object.keys(record).sort().join(',')!=='drafts,kind,source,version'||record.version!==1||!text(record.source)||!validDrafts(record.drafts))throw new Error('Invalid native-authority recovery snapshot');
 return {source:record.source,drafts:record.drafts};
}
