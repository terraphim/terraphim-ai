"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.flushStableSnapshots = flushStableSnapshots;
/** Wait for every view to agree with one current accepted snapshot; never accept stale replies. */
async function flushStableSnapshots(current, flushViews, synchronize) {
    for (let attempt = 0; attempt < 3; attempt++) {
        const round = await flushViews(), accepted = current();
        if (round.participantsUnchanged && round.snapshots.every(snapshot => snapshot.authorityEpoch === accepted.authorityEpoch && snapshot.revision === accepted.revision && snapshot.sourceHash === accepted.sourceHash && snapshot.historyGeneration === accepted.historyGeneration))
            return;
        if (attempt < 2)
            await synchronize();
    }
    throw new Error('Editor snapshots kept changing; drafts retained. Save again after editing settles');
}
//# sourceMappingURL=flushSnapshots.js.map