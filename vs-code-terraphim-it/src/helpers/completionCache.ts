/** Identity of a request whose response may populate the completion cache. */
export interface CompletionRequest {
  generation: number;
  revision: number;
}

/** Cache results only for the latest query and configuration, even across awaits. */
export class CompletionCache<T> {
  private generation = 0;
  private revision = -1;
  private query = '';
  private results: T[] = [];

  /** Start a distinct request, or reuse results for the same query/revision. */
  begin(query: string, revision: number): { request: CompletionRequest; cached?: T[] } {
    if (query === this.query && revision === this.revision) {
      return { request: {generation: this.generation, revision}, cached: this.results };
    }
    this.query = query;
    this.revision = revision;
    this.results = [];
    return { request: {generation: ++this.generation, revision} };
  }

  /** Check before a fallback request so old text is not sent to a new endpoint. */
  isCurrent(request: CompletionRequest, revision: number): boolean {
    return request.generation === this.generation && request.revision === revision && revision === this.revision;
  }

  /** Publish only the latest request's response; stale responses stay invisible. */
  commit(request: CompletionRequest, revision: number, results: T[]): T[] {
    if (!this.isCurrent(request, revision)) return [];
    this.results = results;
    return results;
  }
}
