"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.CompletionCache = void 0;
/** Cache results only for the latest query and configuration, even across awaits. */
class CompletionCache {
    constructor() {
        this.generation = 0;
        this.revision = -1;
        this.query = '';
        this.results = [];
    }
    /** Start a distinct request, or reuse results for the same query/revision. */
    begin(query, revision) {
        if (query === this.query && revision === this.revision) {
            return { request: { generation: this.generation, revision }, cached: this.results };
        }
        this.query = query;
        this.revision = revision;
        this.results = [];
        return { request: { generation: ++this.generation, revision } };
    }
    /** Check before a fallback request so old text is not sent to a new endpoint. */
    isCurrent(request, revision) {
        return request.generation === this.generation && request.revision === revision && revision === this.revision;
    }
    /** Publish only the latest request's response; stale responses stay invisible. */
    commit(request, revision, results) {
        if (!this.isCurrent(request, revision))
            return [];
        this.results = results;
        return results;
    }
}
exports.CompletionCache = CompletionCache;
//# sourceMappingURL=completionCache.js.map