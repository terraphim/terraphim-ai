"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.CheckpointStage = void 0;
const LIMIT = 16 * 1024 * 1024, UNITS = 256 * 1024;
const same = (a, b) => a.resource === b.resource && a.session === b.session && a.authorityEpoch === b.authorityEpoch && a.revision === b.revision && a.sourceHash === b.sourceHash && a.historyGeneration === b.historyGeneration;
class CheckpointStage {
    constructor(timeout = 60000) {
        this.timeout = timeout;
    }
    clear() { if (this.pending)
        clearTimeout(this.pending.timer); this.pending = undefined; this.refusal = undefined; }
    receive(value, identity, capacity) {
        const message = value;
        if (!message || message.protocol !== 1 || message.type !== 'checkpoint-chunk' || !same(message, identity))
            throw new Error('Stale checkpoint stage');
        try {
            if (typeof message.token !== 'string' || !message.token || message.token.length > 64 || !Number.isSafeInteger(message.index) || !Number.isSafeInteger(message.total) || !Number.isSafeInteger(message.bytes)
                || message.index < 0 || message.total < 1 || typeof message.text !== 'string' || !message.text.length || message.text.length > UNITS || Buffer.byteLength(JSON.stringify(message), 'utf8') > LIMIT)
                throw new Error('Invalid checkpoint chunk');
            if (message.index === 0) {
                if (this.pending?.token === message.token)
                    throw new Error('Repeated checkpoint chunk');
                this.clear();
                if (message.bytes < 0 || message.bytes > capacity || message.bytes > 503 * LIMIT || message.total > Math.ceil(message.bytes / UNITS) + 1)
                    throw new Error('Checkpoint capacity unavailable; draft retained');
                const timer = setTimeout(() => this.clear(), this.timeout);
                this.pending = { identity: { ...identity }, token: message.token, total: message.total, bytes: message.bytes, parts: [], size: 0, timer };
            }
            const stage = this.pending;
            if (!stage || !same(stage.identity, identity) || stage.token !== message.token || stage.parts.length !== message.index || stage.total !== message.total || stage.bytes !== message.bytes)
                throw new Error('Checkpoint chunk sequence mismatch');
            if (message.index < message.total - 1 && message.text.length !== UNITS)
                throw new Error('Invalid checkpoint chunk length');
            stage.size += Buffer.byteLength(message.text, 'utf8');
            if (stage.size > stage.bytes + stage.total * 2)
                throw new Error('Checkpoint stage exceeds declared bytes');
            stage.parts.push(message.text);
        }
        catch (error) {
            this.clear();
            this.refusal = String(error);
            throw error;
        }
    }
    consume(token, identity) {
        const stage = this.pending, refusal = this.refusal;
        this.clear();
        if (refusal)
            throw new Error(refusal);
        if (!stage || stage.token !== token || stage.parts.length !== stage.total || !same(stage.identity, identity))
            throw new Error('Missing or stale checkpoint stage');
        const json = stage.parts.join('');
        if (Buffer.byteLength(json, 'utf8') !== stage.bytes)
            throw new Error('Checkpoint byte count differs');
        return JSON.parse(json);
    }
}
exports.CheckpointStage = CheckpointStage;
//# sourceMappingURL=checkpointStage.js.map