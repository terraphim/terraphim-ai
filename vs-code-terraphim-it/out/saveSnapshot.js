"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.assertSaveSnapshotCurrent = assertSaveSnapshotCurrent;
/** Refuse successful Save completion while newer accepted or raw changes remain. */
function assertSaveSnapshotCurrent(doc, source, revision) {
    if (doc.pendingAuthority || doc.drafts.size)
        throw new Error('Newer draft remains unsaved');
    if (doc.revision !== revision || doc.source !== source)
        throw new Error('Newer source remains unsaved');
}
//# sourceMappingURL=saveSnapshot.js.map