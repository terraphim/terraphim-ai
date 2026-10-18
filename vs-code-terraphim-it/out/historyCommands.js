"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.HistoryCommands = void 0;
const crypto_1 = require("crypto");
/** Bounded FIFO admission; a queued future nonce never authorizes native work. */
class HistoryCommands {
    constructor() {
        this.commands = [];
        this.waiters = new Set();
    }
    get size() { return this.commands.length; }
    /** Issue one cryptographic token without replacing the command already executing. */
    issue(epoch) {
        if (!Number.isSafeInteger(epoch) || epoch < 0)
            throw new Error('Invalid history view epoch');
        if (this.commands.length >= 32)
            throw new Error('Too many pending history commands; wait for editing to settle');
        const command = Object.freeze({ nonce: (0, crypto_1.randomUUID)(), epoch });
        this.commands.push(command);
        return command;
    }
    /** Match only the oldest issued token and the currently activated view. */
    matchesHead(nonce, epoch, currentEpoch) {
        const head = this.commands[0];
        return Boolean(head && typeof nonce === 'string' && Number.isSafeInteger(epoch)
            && epoch === currentEpoch && head.nonce === nonce && head.epoch === epoch);
    }
    /** Retire a terminal command; stale, foreign or future completion is inert. */
    completeHead(nonce, epoch, currentEpoch) {
        if (!this.matchesHead(nonce, epoch, currentEpoch))
            return false;
        this.commands.shift();
        if (!this.commands.length)
            for (const waiter of this.waiters) {
                clearTimeout(waiter.timer);
                this.waiters.delete(waiter);
                waiter.resolve();
            }
        return true;
    }
    /** Await actual terminal completion outside the document mutation queue. */
    waitForIdle(epoch, timeoutMs = 15000) {
        if (!Number.isSafeInteger(epoch) || epoch < 0 || !Number.isSafeInteger(timeoutMs) || timeoutMs < 1 || timeoutMs > 15000)
            return Promise.reject(new Error('Invalid history drain boundary'));
        if (this.commands.some(command => command.epoch !== epoch))
            return Promise.reject(new Error('History view epoch changed; drafts retained'));
        if (!this.commands.length)
            return Promise.resolve();
        return new Promise((resolve, reject) => {
            const waiter = { resolve, reject, timer: setTimeout(() => { this.waiters.delete(waiter); reject(new Error('Pending title history did not settle; drafts retained')); }, timeoutMs) };
            this.waiters.add(waiter);
        });
    }
    /** Retire every token when its view or native authority boundary is replaced. */
    clear() {
        this.commands.length = 0;
        for (const waiter of this.waiters) {
            clearTimeout(waiter.timer);
            this.waiters.delete(waiter);
            waiter.reject(new Error('Pending title history was retired; drafts retained'));
        }
    }
}
exports.HistoryCommands = HistoryCommands;
//# sourceMappingURL=historyCommands.js.map