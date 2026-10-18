import type { AuthorityBase } from './resourceAuthority';

/** An acknowledged view boundary, without its resource-local routing fields. */
export type FlushSnapshot=Pick<AuthorityBase,'authorityEpoch'|'revision'|'sourceHash'|'historyGeneration'>;
interface FlushRound { snapshots:readonly FlushSnapshot[]; participantsUnchanged:boolean; }
/** Wait for every view to agree with one current accepted snapshot; never accept stale replies. */
export async function flushStableSnapshots(current:()=>FlushSnapshot,flushViews:()=>Promise<FlushRound>,synchronize:()=>Promise<void>):Promise<void> {
 for(let attempt=0;attempt<3;attempt++) {
  const round=await flushViews(),accepted=current();
  if(round.participantsUnchanged && round.snapshots.every(snapshot=>snapshot.authorityEpoch===accepted.authorityEpoch && snapshot.revision===accepted.revision && snapshot.sourceHash===accepted.sourceHash && snapshot.historyGeneration===accepted.historyGeneration))return;
  if(attempt<2)await synchronize();
 }
 throw new Error('Editor snapshots kept changing; drafts retained. Save again after editing settles');
}
