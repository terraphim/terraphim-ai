"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.ResourceAuthority = void 0;
const worker_threads_1 = require("worker_threads");
const path = require("path");
const INPUT_LIMIT = 16 * 1024 * 1024;
function admitText(value, label) { if (typeof value !== 'string' || Buffer.byteLength(value, 'utf8') > INPUT_LIMIT)
    throw new Error(`${label} exceeds source/argument limit`); }
function admitArguments(args) { if (!Array.isArray(args) || args.length > 4096)
    throw new Error('Action argument count limit'); let total = 0; for (const value of args) {
    admitText(value, 'Action argument');
    total += Buffer.byteLength(value, 'utf8');
    if (total > INPUT_LIMIT)
        throw new Error('Combined text arguments exceed limit');
} }
/** One asynchronous resource-owned worker. It never silently restarts lost native state. */
class ResourceAuthority {
    constructor(options) {
        this.options = options;
        this.waiting = new Map();
        this.failureListeners = new Set();
        this.sequence = 0;
        this.disposed = false;
        this.uncertain = false;
        if (!path.isAbsolute(options.extensionPath))
            throw new Error('Authority extension path must be absolute');
        for (const value of [options.source, options.roleJson])
            if (value !== undefined && (typeof value !== 'string' || Buffer.byteLength(value) > 16 * 1024 * 1024))
                throw new Error('Authority initial source/role exceeds limit');
        if (options.timeoutMs !== undefined && (!Number.isSafeInteger(options.timeoutMs) || options.timeoutMs < 1 || options.timeoutMs > 60000))
            throw new Error('Invalid authority timeout');
        this.worker = new worker_threads_1.Worker(path.join(options.extensionPath, 'media', 'resource-authority-worker.cjs'), {
            workerData: { ...options, timeoutMs: undefined }, resourceLimits: { maxOldGenerationSizeMb: 256, stackSizeMb: 8 },
        });
        this.worker.on('message', (reply) => {
            if (!reply || !Number.isSafeInteger(reply.rpc))
                return;
            if (reply.fatal) {
                this.fail(new Error(reply.error || 'Native authority requires explicit recovery'));
                return;
            }
            const pending = this.waiting.get(reply.rpc);
            if (!pending)
                return;
            clearTimeout(pending.timer);
            this.waiting.delete(reply.rpc);
            if (reply.ok)
                pending.resolve(reply.value);
            else
                pending.reject(new Error(reply.error || 'Native authority refused'));
        });
        this.worker.on('error', error => this.fail(error));
        this.worker.on('messageerror', error => this.fail(error instanceof Error ? error : new Error(String(error))));
        this.worker.on('exit', code => { if (!this.disposed)
            this.fail(new Error(`Native authority context exited (${code}); explicit recovery is required`)); });
    }
    static async create(options) {
        const authority = new ResourceAuthority(options);
        try {
            await authority.call('ready', {});
            return authority;
        }
        catch (error) {
            await authority.dispose();
            throw error;
        }
    }
    onFailure(listener) { this.failureListeners.add(listener); if (this.failed)
        listener(this.failed); return () => this.failureListeners.delete(listener); }
    fail(error) {
        if (this.failed || this.disposed)
            return;
        this.failed = error;
        for (const pending of this.waiting.values()) {
            clearTimeout(pending.timer);
            pending.reject(error);
        }
        this.waiting.clear();
        for (const listener of this.failureListeners) {
            try {
                listener(error);
            }
            catch { /* Other recovery subscribers still run. */ }
        }
    }
    call(operation, payload) {
        if (this.disposed)
            return Promise.reject(new Error('Native authority disposed'));
        if (this.failed)
            return Promise.reject(this.failed);
        const rpc = ++this.sequence;
        if (!Number.isSafeInteger(rpc))
            return Promise.reject(new Error('Authority RPC identities exhausted'));
        return new Promise((resolve, reject) => {
            const timer = setTimeout(() => { this.waiting.delete(rpc); this.uncertain = true; reject(new Error('Native authority reply timed out; query status before further mutation')); }, this.options.timeoutMs ?? 15000);
            this.waiting.set(rpc, { resolve: value => resolve(value), reject, timer });
            try {
                this.worker.postMessage({ rpc, operation, payload });
            }
            catch (error) {
                clearTimeout(timer);
                this.waiting.delete(rpc);
                reject(error instanceof Error ? error : new Error(String(error)));
            }
        });
    }
    mutation(operation, payload) { if (this.uncertain)
        return Promise.reject(new Error('Native authority outcome is uncertain; status reconciliation required')); return this.call(operation, payload); }
    snapshot() { return this.call('snapshot', {}); }
    attachView(viewSession) { return this.call('attach', { viewSession }); }
    detachView(viewSession) { return this.call('detach', { viewSession }); }
    prepareAction(request, action) { try {
        admitText(action.expectedBody, 'Expected body');
        admitText(JSON.stringify(action.meta), 'Action metadata');
        admitArguments(action.textArgs);
    }
    catch (error) {
        return Promise.reject(error);
    } return this.mutation('prepare-action', { request, action }); }
    prepareHistory(request, replay) { try {
        admitText(replay.expectedBody, 'Expected body');
        admitText(replay.targetBody, 'Target body');
        admitText(replay.operationsJson, 'History operations');
    }
    catch (error) {
        return Promise.reject(error);
    } return this.mutation('prepare-history', { request, replay }); }
    prepareTrim(request, trim) { try {
        admitText(trim.expectedBody, 'Expected body');
        if (!Array.isArray(trim.kept) || trim.kept.length > 4096)
            throw new Error('Protected trim count limit');
    }
    catch (error) {
        return Promise.reject(error);
    } return this.mutation('prepare-trim', { request, trim }); }
    finish(request, token, afterHash) { return this.call('finish', { request, token, afterHash }); }
    abort(request, token) { return this.call('abort', { request, token }); }
    async status(request) { const result = await this.call('status', { request }); this.uncertain = false; return result; }
    ackOutcome(request, outcome, accepted) { return this.call('ack', { request, outcome, accepted }); }
    updateHistory(expectedBase, nextHistoryGeneration) { return this.mutation('update-history', { expectedBase, nextHistoryGeneration }); }
    query(name, args = []) { try {
        if (!Array.isArray(args) || args.length > 2)
            throw new Error('Query argument count limit');
        for (const arg of args)
            admitText(JSON.stringify(arg), 'Query argument');
    }
    catch (error) {
        return Promise.reject(error);
    } return this.call('query', { name, args }); }
    control(control) { try {
        for (const value of Object.values(control))
            admitText(value, 'Control argument');
    }
    catch (error) {
        return Promise.reject(error);
    } return this.mutation('control', control); }
    async dispose() {
        if (this.disposed)
            return;
        this.disposed = true;
        const error = new Error('Native authority disposed');
        for (const pending of this.waiting.values()) {
            clearTimeout(pending.timer);
            pending.reject(error);
        }
        this.waiting.clear();
        await this.worker.terminate();
        this.failureListeners.clear();
    }
}
exports.ResourceAuthority = ResourceAuthority;
//# sourceMappingURL=resourceAuthority.js.map