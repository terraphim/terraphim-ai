/** State consulted after the saved snapshot has been written and read back. */
interface SaveSnapshotState {
 readonly source:string;
 readonly revision:number;
 readonly drafts:ReadonlyMap<string,unknown>;
 readonly pendingAuthority?:unknown;
}
/** Refuse successful Save completion while newer accepted or raw changes remain. */
export function assertSaveSnapshotCurrent(doc:SaveSnapshotState,source:string,revision:number):void {
 if(doc.pendingAuthority || doc.drafts.size) throw new Error('Newer draft remains unsaved');
 if(doc.revision!==revision || doc.source!==source) throw new Error('Newer source remains unsaved');
}
