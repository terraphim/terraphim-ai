import {createHash} from 'crypto';
import {AuthorityDraft,validateRecoveryData} from './authorityRecovery';
/** One resource-private persisted snapshot; never a history or receipt registry. */
export interface ResourceBackup {
 kind:'terraphim-resource-backup';version:1;resource:string;authorityEpoch:string;token:string;baselineHash:string;source:string;drafts:AuthorityDraft[];
}
export const MAX_RESOURCE_BACKUPS=32;
export const MAX_RESOURCE_BACKUP_BYTES=512*1024*1024;
const UUID=/^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/;
/** Derive a private directory without using a resource as a filesystem path. */
export function resourceDirectoryKey(resource:string):string {
 if(!resource || resource.length>1024*1024 || !/^[a-z][a-z0-9+.-]*:/i.test(resource))throw new Error('Invalid backup resource URI');
 return createHash('sha256').update(resource,'utf8').digest('hex');
}
/** Only an adapter-issued authority identifier can select a backup filename. */
export function resourceBackupFileName(authorityEpoch:string,token:string):string {
 if(!UUID.test(authorityEpoch)||!UUID.test(token))throw new Error('Invalid backup authority/token');return authorityEpoch+'.'+token+'.json';
}
function validate(value:unknown,resource?:string,epoch?:string):asserts value is ResourceBackup {
 if(!value||typeof value!=='object'||Array.isArray(value))throw new Error('Invalid resource backup');
 const entry=value as ResourceBackup;
 if(Object.keys(entry).sort().join(',')!=='authorityEpoch,baselineHash,drafts,kind,resource,source,token,version'
  ||entry.kind!=='terraphim-resource-backup'||entry.version!==1||typeof entry.resource!=='string'
  ||typeof entry.authorityEpoch!=='string'||typeof entry.token!=='string'||!UUID.test(entry.authorityEpoch)||!UUID.test(entry.token)
  ||typeof entry.baselineHash!=='string'||!/^[0-9a-f]{64}$/.test(entry.baselineHash))throw new Error('Invalid resource backup identity');
 resourceDirectoryKey(entry.resource);validateRecoveryData(entry.source,entry.drafts);
 if(resource!==undefined&&entry.resource!==resource||epoch!==undefined&&entry.authorityEpoch!==epoch)throw new Error('Foreign resource backup');
}
/** Encode the exact accepted source and distinguishable drafts with bounded metadata. */
export function encodeResourceBackup(value:ResourceBackup):string {
 validate(value);const raw=JSON.stringify(value);if(Buffer.byteLength(raw)>MAX_RESOURCE_BACKUP_BYTES)throw new Error('Resource backup exceeds capacity');return raw;
}
/** Decode only strict metadata; it contains no executable paths or replay authority. */
export function decodeResourceBackup(raw:string,resource:string,epoch?:string):ResourceBackup {
 if(Buffer.byteLength(raw)>MAX_RESOURCE_BACKUP_BYTES)throw new Error('Resource backup exceeds capacity');const value:unknown=JSON.parse(raw);validate(value,resource,epoch);return value;
}
/** An old callback cannot retire a newer snapshot or another window's raw versions. */
export function canRetireResourceBackup(value:ResourceBackup,resource:string,epoch:string,token:string,savedSource?:string):boolean {
 return value.resource===resource&&value.authorityEpoch===epoch&&value.token===token
  &&(savedSource===undefined||value.source===savedSource&&value.drafts.length===0);
}
/** Admit only immutable snapshot filenames; incomplete writes count against capacity. */
export function resourceBackupNames(names:readonly string[]):string[] {
 const retained=names.filter(name=>/^[0-9a-f-]{36}\.[0-9a-f-]{36}\.json(?:\.partial)?$/.test(name));
 if(retained.length>MAX_RESOURCE_BACKUPS)throw new Error('Retained recovery file capacity exceeded; all files are preserved');
 return retained.filter(name=>name.endsWith('.json'));
}
/** Bound aggregate discovery before reading any of the stored payloads. */
export function admitResourceBackupBytes(sizes:readonly number[],heapHeadroom:number):void {
 const capacity=Math.min(MAX_RESOURCE_BACKUP_BYTES,Math.floor(heapHeadroom/4));let total=0;
 for(const bytes of sizes){if(!Number.isSafeInteger(bytes)||bytes<0)throw new Error('Invalid backup file size');total+=bytes;if(total>capacity)throw new Error('Retained recovery data exceeds safe discovery capacity; all files are preserved');}
}
/** An externally changed baseline is never silently overwritten by recovery. */
export function resourceBackupBaselineMatches(backup:ResourceBackup,disk:string):boolean {
 return backup.baselineHash===createHash('sha256').update(disk,'utf8').digest('hex');
}
