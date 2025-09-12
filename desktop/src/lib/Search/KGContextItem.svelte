<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import { Tag, Button, Icon } from "svelma";

  export let contextItem: KGContextItem;
  export let removable: boolean = true;
  export let compact: boolean = false;

  const dispatch = createEventDispatcher();

  interface KGContextItem {
    id: string;
    context_type: "KGTermDefinition" | "KGIndex";
    title: string;
    summary?: string;
    content: string;
    metadata: Record<string, string>;
    created_at: string;
    relevance_score?: number;

    // KG-specific fields
    kg_term_definition?: KGTermDefinition;
    kg_index_info?: KGIndexInfo;
  }

  interface KGTermDefinition {
    term: string;
    normalized_term: string;
    id: number;
    definition?: string;
    synonyms: string[];
    related_terms: string[];
    usage_examples: string[];
    url?: string;
    metadata: Record<string, string>;
    relevance_score?: number;
  }

  interface KGIndexInfo {
    name: string;
    total_terms: number;
    total_nodes: number;
    total_edges: number;
    last_updated: string;
    source: string;
    version?: string;
  }

  // Helper function to format numbers
  function formatNumber(num: number): string {
    return new Intl.NumberFormat().format(num);
  }

  // Helper function to format date
  function formatDate(dateString: string): string {
    try {
      return new Date(dateString).toLocaleDateString();
    } catch {
      return dateString;
    }
  }

  // Handle remove context item
  function handleRemove() {
    dispatch("remove", { contextId: contextItem.id });
  }

  // Handle view details
  function handleViewDetails() {
    dispatch("viewDetails", { contextItem });
  }

  // Get display icon based on context type
  $: displayIcon = contextItem.context_type === "KGTermDefinition"
    ? "fas fa-tag"
    : "fas fa-sitemap";

  // Get display color based on context type
  $: displayColor = contextItem.context_type === "KGTermDefinition"
    ? "is-info"
    : "is-primary";
</script>

<style>
  .kg-context-item {
    border: 1px solid #e1e1e1;
    border-radius: 6px;
    padding: 1rem;
    margin-bottom: 0.75rem;
    background: #fefefe;
    transition: all 0.2s ease;
  }

  .kg-context-item:hover {
    border-color: #b0b0b0;
    box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
  }

  .kg-context-item.compact {
    padding: 0.5rem;
    margin-bottom: 0.5rem;
  }

  .context-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    margin-bottom: 0.5rem;
  }

  .context-header.compact {
    margin-bottom: 0.25rem;
  }

  .context-title {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    flex: 1;
  }

  .context-title-text {
    font-weight: 600;
    color: #363636;
    font-size: 1rem;
  }

  .context-title-text.compact {
    font-size: 0.875rem;
  }

  .context-actions {
    display: flex;
    gap: 0.25rem;
    align-items: center;
  }

  .context-content {
    margin-bottom: 0.75rem;
  }

  .context-content.compact {
    margin-bottom: 0.5rem;
  }

  .term-definition {
    background: #f8f9fa;
    border-left: 3px solid #3273dc;
    padding: 0.75rem;
    margin: 0.5rem 0;
    border-radius: 0 4px 4px 0;
  }

  .term-definition.compact {
    padding: 0.5rem;
    margin: 0.25rem 0;
  }

  .definition-text {
    font-style: italic;
    color: #4a4a4a;
    margin-bottom: 0.5rem;
  }

  .definition-text.compact {
    margin-bottom: 0.25rem;
    font-size: 0.875rem;
  }

  .term-metadata {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
    margin-top: 0.5rem;
  }

  .term-metadata.compact {
    gap: 0.25rem;
    margin-top: 0.25rem;
  }

  .kg-index-stats {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(120px, 1fr));
    gap: 0.75rem;
    background: #f8f9fa;
    padding: 0.75rem;
    border-radius: 4px;
    margin: 0.5rem 0;
  }

  .kg-index-stats.compact {
    grid-template-columns: repeat(auto-fit, minmax(100px, 1fr));
    gap: 0.5rem;
    padding: 0.5rem;
    margin: 0.25rem 0;
  }

  .stat-item {
    text-align: center;
  }

  .stat-value {
    font-weight: 600;
    font-size: 1.25rem;
    color: #3273dc;
  }

  .stat-value.compact {
    font-size: 1rem;
  }

  .stat-label {
    font-size: 0.75rem;
    color: #757575;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .context-summary {
    font-size: 0.875rem;
    color: #757575;
    line-height: 1.4;
  }

  .context-summary.compact {
    font-size: 0.75rem;
  }

  .context-meta {
    font-size: 0.75rem;
    color: #9e9e9e;
    margin-top: 0.5rem;
    display: flex;
    align-items: center;
    gap: 1rem;
  }

  .context-meta.compact {
    margin-top: 0.25rem;
    gap: 0.5rem;
  }

  .relevance-score {
    background: #e8f5e8;
    color: #2e7d2e;
    padding: 0.125rem 0.375rem;
    border-radius: 12px;
    font-weight: 500;
  }
</style>

<div class="kg-context-item {compact ? 'compact' : ''}">
  <div class="context-header {compact ? 'compact' : ''}">
    <div class="context-title">
      <Icon icon={displayIcon} size="is-small" />
      <span class="context-title-text {compact ? 'compact' : ''}">
        {contextItem.title}
      </span>
      <Tag type={displayColor} size="is-small">
        {contextItem.context_type === "KGTermDefinition" ? "KG Term" : "KG Index"}
      </Tag>
    </div>

    <div class="context-actions">
      <Button
        size="is-small"
        type="is-ghost"
        on:click={handleViewDetails}
        title="View details"
      >
        <Icon icon="fas fa-eye" size="is-small" />
      </Button>

      {#if removable}
        <Button
          size="is-small"
          type="is-ghost"
          on:click={handleRemove}
          title="Remove from context"
        >
          <Icon icon="fas fa-times" size="is-small" />
        </Button>
      {/if}
    </div>
  </div>

  <div class="context-content {compact ? 'compact' : ''}">
    {#if contextItem.context_type === "KGTermDefinition" && contextItem.kg_term_definition}
      {@const term = contextItem.kg_term_definition}

      <div class="term-definition {compact ? 'compact' : ''}">
        {#if term.definition}
          <div class="definition-text {compact ? 'compact' : ''}">
            {term.definition}
          </div>
        {/if}

        <div class="term-metadata {compact ? 'compact' : ''}">
          {#if term.synonyms.length > 0}
            <Tag type="is-light" size="is-small" title="Synonyms">
              <Icon icon="fas fa-equals" size="is-small" />
              <span>{term.synonyms.length} synonym{term.synonyms.length !== 1 ? 's' : ''}</span>
            </Tag>
          {/if}

          {#if term.related_terms.length > 0}
            <Tag type="is-light" size="is-small" title="Related Terms">
              <Icon icon="fas fa-link" size="is-small" />
              <span>{term.related_terms.length} related</span>
            </Tag>
          {/if}

          {#if term.usage_examples.length > 0}
            <Tag type="is-light" size="is-small" title="Usage Examples">
              <Icon icon="fas fa-quote-left" size="is-small" />
              <span>{term.usage_examples.length} example{term.usage_examples.length !== 1 ? 's' : ''}</span>
            </Tag>
          {/if}

          {#if term.url}
            <Tag type="is-link" size="is-small">
              <Icon icon="fas fa-external-link-alt" size="is-small" />
              <span>Source</span>
            </Tag>
          {/if}
        </div>
      </div>

    {:else if contextItem.context_type === "KGIndex" && contextItem.kg_index_info}
      {@const index = contextItem.kg_index_info}

      <div class="kg-index-stats {compact ? 'compact' : ''}">
        <div class="stat-item">
          <div class="stat-value {compact ? 'compact' : ''}">{formatNumber(index.total_terms)}</div>
          <div class="stat-label">Terms</div>
        </div>
        <div class="stat-item">
          <div class="stat-value {compact ? 'compact' : ''}">{formatNumber(index.total_nodes)}</div>
          <div class="stat-label">Nodes</div>
        </div>
        <div class="stat-item">
          <div class="stat-value {compact ? 'compact' : ''}">{formatNumber(index.total_edges)}</div>
          <div class="stat-label">Edges</div>
        </div>
        <div class="stat-item">
          <div class="stat-value {compact ? 'compact' : ''}">{index.version || 'N/A'}</div>
          <div class="stat-label">Version</div>
        </div>
      </div>
    {/if}

    {#if contextItem.summary && !compact}
      <div class="context-summary">
        {contextItem.summary}
      </div>
    {/if}
  </div>

  <div class="context-meta {compact ? 'compact' : ''}">
    <span>Added {formatDate(contextItem.created_at)}</span>

    {#if contextItem.relevance_score}
      <span class="relevance-score">
        Relevance: {(contextItem.relevance_score * 100).toFixed(0)}%
      </span>
    {/if}

    {#if contextItem.metadata.source_document}
      <span title="Source Document">
        <Icon icon="fas fa-file-alt" size="is-small" />
        {contextItem.metadata.document_title || contextItem.metadata.source_document}
      </span>
    {/if}
  </div>
</div>
