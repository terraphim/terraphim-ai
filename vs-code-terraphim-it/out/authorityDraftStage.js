"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.AuthorityDraftStage = void 0;
const authorityRecovery_1 = require("./authorityRecovery");
/** Bounded raw-draft transport independent of native revision/ACK timing. */
class AuthorityDraftStage {
    constructor(timeout = 60000) {
        this.timeout = timeout;
        this.completed = 0;
    }
    clear() { if (this.pending)
        clearTimeout(this.pending.timer); this.pending = undefined; }
    receive(value, identity, capacity) {
        const m = value;
        if (!m || m.resource !== identity.resource || m.session !== identity.session || m.authorityEpoch !== identity.authorityEpoch)
            throw new Error('Stale draft identity');
        if (!Number.isSafeInteger(m.draftSequence) || m.draftSequence <= this.completed)
            throw new Error('Stale draft sequence');
        try {
            if (typeof m.token !== 'string' || !m.token || m.token.length > 64 || !Number.isSafeInteger(m.index) || !Number.isSafeInteger(m.total) || !Number.isSafeInteger(m.bytes) || m.index < 0 || m.total < 1 || m.index >= m.total || m.bytes < 0 || m.bytes > authorityRecovery_1.MAX_DRAFT_BYTES || m.bytes > capacity || typeof m.text !== 'string' || m.text.length > 256 * 1024 || m.total > Math.ceil(m.bytes / (256 * 1024)) + 1)
                throw new Error('Invalid draft chunk or capacity');
            if (m.index === 0) {
                if (this.pending && m.draftSequence <= this.pending.sequence)
                    throw new Error('Repeated draft start');
                this.clear();
                this.pending = { token: m.token, sequence: m.draftSequence, bytes: m.bytes, total: m.total, parts: [], size: 0, timer: setTimeout(() => this.clear(), this.timeout) };
            }
            const stage = this.pending;
            if (!stage || stage.token !== m.token || stage.sequence !== m.draftSequence || stage.parts.length !== m.index || stage.total !== m.total || stage.bytes !== m.bytes)
                throw new Error('Draft chunk sequence mismatch');
            stage.size += Buffer.byteLength(m.text, 'utf8');
            if (stage.size > stage.bytes + 2 * stage.total)
                throw new Error('Draft byte capacity exceeded');
            stage.parts.push(m.text);
            if (stage.parts.length !== stage.total)
                return;
            const draft = stage.parts.join('');
            if (Buffer.byteLength(draft, 'utf8') !== stage.bytes)
                throw new Error('Draft byte count differs');
            this.completed = stage.sequence;
            this.clear();
            return { viewSession: identity.session, sequence: m.draftSequence, draft };
        }
        catch (error) {
            this.clear();
            throw error;
        }
    }
}
exports.AuthorityDraftStage = AuthorityDraftStage;
//# sourceMappingURL=authorityDraftStage.js.map