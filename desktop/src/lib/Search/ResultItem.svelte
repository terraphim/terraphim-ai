<script lang="ts">
  import { Taglist, Tag } from "svelma";
  import { fade } from "svelte/transition";
  import ArticleModal from "./ArticleModal.svelte";
  import type { Document } from "./SearchResult";
  import configStore from "../ThemeSwitcher.svelte";
  import { role, is_tauri, serverUrl } from "../stores";
  import { CONFIG } from "../../config";
  import { invoke } from '@tauri-apps/api/tauri';
  import type { DocumentListResponse } from "../generated/types";
  import SvelteMarkdown from 'svelte-markdown';

  export let document: Document;
  let showModal = false;
  let showKgModal = false;
  let kgDocument: Document | null = null;
  let kgTerm: string | null = null;
  let kgRank: number | null = null;
  let loadingKg = false;

  // Summarization state
  let aiSummary: string | null = null;
  let summaryLoading = false;
  let summaryError: string | null = null;
  let showAiSummary = false;
  let summaryFromCache = false;

  const onTitleClick = () => {
    showModal = true;
  };

  async function handleTagClick(tag: string) {
    loadingKg = true;
    kgTerm = tag;
    
    // Add debugging information
    console.log('🔍 KG Search Debug Info:');
    console.log('  Tag clicked:', tag);
    console.log('  Current role:', $role);
    console.log('  Is Tauri mode:', $is_tauri);
    
    try {
      if ($is_tauri) {
        // Use Tauri command for desktop app
        console.log('  Making Tauri invoke call...');
        console.log('  Tauri command: find_documents_for_kg_term');
        console.log('  Tauri params:', { roleName: $role, term: tag });
        
        const response: DocumentListResponse = await invoke('find_documents_for_kg_term', {
          roleName: $role,
          term: tag
        });
        
        console.log('  📥 Tauri response received:');
        console.log('    Status:', response.status);
        console.log('    Results count:', response.results?.length || 0);
        console.log('    Total:', response.total || 0);
        console.log('    Full response:', JSON.stringify(response, null, 2));
        
        if (response.status === 'success' && response.results && response.results.length > 0) {
          // Get the first (highest-ranked) document
          kgDocument = response.results[0];
          kgRank = kgDocument.rank || 0;
          console.log('  ✅ Found KG document:');
          console.log('    Title:', kgDocument.title);
          console.log('    Rank:', kgRank);
          console.log('    Body length:', kgDocument.body?.length || 0, 'characters');
          showKgModal = true;
        } else {
          console.warn(`  ⚠️  No KG documents found for term: "${tag}" in role: "${$role}"`);
          console.warn('    This could indicate:');
          console.warn('    1. Knowledge graph not built for this role');
          console.warn('    2. Term not found in knowledge graph');
          console.warn('    3. Role not configured with TerraphimGraph relevance function');
          console.warn('    Suggestion: Check server logs for KG building status');
        }
      } else {
        // Use HTTP fetch for web mode
        console.log('  Making HTTP fetch call...');
        const baseUrl = CONFIG.ServerURL;
        const encodedRole = encodeURIComponent($role);
        const encodedTerm = encodeURIComponent(tag);
        const url = `${baseUrl}/roles/${encodedRole}/kg_search?term=${encodedTerm}`;
        
        console.log('  📤 HTTP Request details:');
        console.log('    Base URL:', baseUrl);
        console.log('    Role (encoded):', encodedRole);
        console.log('    Term (encoded):', encodedTerm);
        console.log('    Full URL:', url);
        
        const response = await fetch(url);
        
        console.log('  📥 HTTP Response received:');
        console.log('    Status code:', response.status);
        console.log('    Status text:', response.statusText);
        console.log('    Headers:', Object.fromEntries(response.headers.entries()));
        
        if (!response.ok) {
          throw new Error(`HTTP error! Status: ${response.status} - ${response.statusText}`);
        }
        
        const data = await response.json();
        console.log('  📄 Response data:');
        console.log('    Status:', data.status);
        console.log('    Results count:', data.results?.length || 0);
        console.log('    Total:', data.total || 0);
        console.log('    Full response:', JSON.stringify(data, null, 2));
        
        if (data.status === 'success' && data.results && data.results.length > 0) {
          // Get the first (highest-ranked) document
          kgDocument = data.results[0];
          kgRank = kgDocument.rank || 0;
          console.log('  ✅ Found KG document:');
          console.log('    Title:', kgDocument.title);
          console.log('    Rank:', kgRank);
          console.log('    Body length:', kgDocument.body?.length || 0, 'characters');
          showKgModal = true;
        } else {
          console.warn(`  ⚠️  No KG documents found for term: "${tag}" in role: "${$role}"`);
          console.warn('    This could indicate:');
          console.warn('    1. Server not configured with Terraphim Engineer role');
          console.warn('    2. Knowledge graph not built on server');
          console.warn('    3. Term not found in knowledge graph');
          console.warn('    Suggestion: Check server logs at startup for KG building status');
          console.warn('    API URL tested:', url);
        }
      }
    } catch (error) {
      console.error('❌ Error fetching KG document:');
      console.error('  Error type:', error.constructor.name);
      console.error('  Error message:', error.message || error);
      console.error('  Request details:', {
        tag,
        role: $role,
        isTauri: $is_tauri,
        timestamp: new Date().toISOString()
      });
      
      if (!$is_tauri && error.message?.includes('Failed to fetch')) {
        console.error('  💡 Network error suggestions:');
        console.error('    1. Check if server is running on expected port');
        console.error('    2. Check CORS configuration');
        console.error('    3. Verify server URL in CONFIG.ServerURL');
      }
      
      // Graceful fallback: could show error message or do nothing
    } finally {
      loadingKg = false;
    }
  }

  async function generateSummary() {
    if (summaryLoading || !document.id || !$role) return;
    
    summaryLoading = true;
    summaryError = null;
    
    console.log('🤖 AI Summary Debug Info:');
    console.log('  Document ID:', document.id);
    console.log('  Current role:', $role);
    console.log('  Is Tauri mode:', $is_tauri);
    
    try {
      const requestBody = {
        document_id: document.id,
        role: $role,
        max_length: 250,
        force_regenerate: false
      };
      
      console.log('  📤 Summarization request:', requestBody);
      
      let response;
      
      if ($is_tauri) {
        // For Tauri mode, we'll make an HTTP request directly
        // as we don't have a Tauri command for summarization yet
        const baseUrl = CONFIG.ServerURL;
        const url = `${baseUrl}/documents/summarize`;
        
        response = await fetch(url, {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json',
          },
          body: JSON.stringify(requestBody)
        });
      } else {
        // Web mode - direct HTTP request
        const baseUrl = CONFIG.ServerURL;
        const url = `${baseUrl}/documents/summarize`;
        
        response = await fetch(url, {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json',
          },
          body: JSON.stringify(requestBody)
        });
      }
      
      console.log('  📥 Summary response received:');
      console.log('    Status code:', response.status);
      console.log('    Status text:', response.statusText);
      
      if (!response.ok) {
        throw new Error(`HTTP error! Status: ${response.status} - ${response.statusText}`);
      }
      
      const data = await response.json();
      console.log('  📄 Summary response data:', data);
      
      if (data.status === 'Success' && data.summary) {
        aiSummary = data.summary;
        summaryFromCache = data.from_cache || false;
        showAiSummary = true;
        console.log('  ✅ Summary generated successfully');
        console.log('    Summary length:', aiSummary.length, 'characters');
        console.log('    From cache:', summaryFromCache);
        console.log('    Model used:', data.model_used);
      } else {
        summaryError = data.error || 'Failed to generate summary';
        console.error('  ❌ Summary generation failed:', summaryError);
      }
    } catch (error) {
      console.error('❌ Error generating summary:');
      console.error('  Error type:', error.constructor.name);
      console.error('  Error message:', error.message || error);
      console.error('  Request details:', {
        documentId: document.id,
        role: $role,
        isTauri: $is_tauri,
        timestamp: new Date().toISOString()
      });
      
      summaryError = error.message || 'Network error occurred';
      
      if (error.message?.includes('Failed to fetch')) {
        console.error('  💡 Network error suggestions:');
        console.error('    1. Check if server is running on expected port');
        console.error('    2. Verify OpenRouter is enabled for this role');
        console.error('    3. Check OPENROUTER_KEY environment variable');
        console.error('    4. Verify server URL in CONFIG.ServerURL');
      }
    } finally {
      summaryLoading = false;
    }
  }

  if (configStore[$role] !== undefined) {
    console.log("Have attribute", configStore[$role]);
    if (configStore[$role].hasOwnProperty("enableLogseq")) {
      console.log("enable logseq True");
    } else {
      console.log("Didn't make it");
    }
  }
</script>

<div class="box">
  <article class="media">
    <div class="media-content">
      <div class="content">
        <div class="level-right">
          {#if document.tags}
          <Taglist>
              {#each document.tags as tag}
                <button 
                  class="tag-button" 
                  on:click={() => handleTagClick(tag)}
                  disabled={loadingKg}
                  title="Click to view knowledge graph document"
                >
                  <Tag rounded>{tag}</Tag>
                </button>
              {/each}
          </Taglist>
          {/if}
        </div>
          <div class="level-right">
          <Taglist>
            <Tag rounded>Rank {document.rank}</Tag>
          </Taglist>
        </div>
        <div transition:fade>
          <button on:click={onTitleClick}>
            <h2 class="title">
              {document.title}
            </h2>
          </button>
          <div class="description">
            <small class="description-label">Description:</small>
            <div class="description-content">
              {#if document.description}
                <SvelteMarkdown source={document.description} />
              {:else}
                <small class="no-description">No description available</small>
              {/if}
            </div>
          </div>
          
          <!-- AI Summary Section -->
          <div class="ai-summary-section">
            {#if !showAiSummary && !summaryLoading && !summaryError}
              <button 
                class="button is-small is-info is-outlined ai-summary-button"
                on:click={generateSummary}
                disabled={summaryLoading}
                title="Generate AI-powered summary using OpenRouter"
              >
                <span class="icon is-small">
                  <i class="fas fa-magic" aria-hidden="true"></i>
                </span>
                <span>AI Summary</span>
              </button>
            {/if}
            
            {#if summaryLoading}
              <div class="ai-summary-loading">
                <span class="icon">
                  <i class="fas fa-spinner fa-spin" aria-hidden="true"></i>
                </span>
                <small>Generating AI summary...</small>
              </div>
            {/if}
            
            {#if summaryError}
              <div class="ai-summary-error">
                <span class="icon has-text-danger">
                  <i class="fas fa-exclamation-triangle" aria-hidden="true"></i>
                </span>
                <small class="has-text-danger">Summary error: {summaryError}</small>
                <button 
                  class="button is-small is-text"
                  on:click={() => { summaryError = null; generateSummary(); }}
                  title="Retry generating summary"
                >
                  Retry
                </button>
              </div>
            {/if}
            
            {#if showAiSummary && aiSummary}
              <div class="ai-summary" transition:fade>
                <div class="ai-summary-header">
                  <small class="ai-summary-label">
                    <span class="icon is-small">
                      <i class="fas fa-robot" aria-hidden="true"></i>
                    </span>
                    AI Summary
                    {#if summaryFromCache}
                      <span class="tag is-small is-light">cached</span>
                    {:else}
                      <span class="tag is-small is-success">fresh</span>
                    {/if}
                  </small>
                  <button 
                    class="button is-small is-text"
                    on:click={() => showAiSummary = false}
                    title="Hide AI summary"
                  >
                    <span class="icon is-small">
                      <i class="fas fa-times" aria-hidden="true"></i>
                    </span>
                  </button>
                </div>
                <div class="ai-summary-content">
                  <SvelteMarkdown source={aiSummary} />
                </div>
                <div class="ai-summary-actions">
                  <button 
                    class="button is-small is-text"
                    on:click={() => { generateSummary(); }}
                    disabled={summaryLoading}
                    title="Regenerate summary"
                  >
                    <span class="icon is-small">
                      <i class="fas fa-redo" aria-hidden="true"></i>
                    </span>
                    <span>Regenerate</span>
                  </button>
                </div>
              </div>
            {/if}
          </div>
          
          <br />
        </div>
      </div>
      <div class="level-right">
        <nav class="level is-mobile" transition:fade>
          <div class="level-right">
            {#if "url" in document}
              <a
                href={document.url}
                target="_blank"
                class="level-item"
                aria-label="URL"
              >
                <span class="icon is-medium">
                  <i class="fas fa-link" />
                </span>
              </a>
            {/if}
            <button 
              type="button" 
              class="level-item button is-ghost" 
              aria-label="Add to favorites"
              on:click={() => {/* TODO: Implement add to favorites */}}
            >
              <span class="icon is-medium">
                <i class="fas fa-plus" aria-hidden="true" />
              </span>
            </button>
            <a
            href={`vscode://${encodeURIComponent(document.title)}.md?${encodeURIComponent(document.body)}`}
            class="level-item"
            aria-label="Open in VSCode"
          >
            <span class="icon is-medium">
              <i class="fas fa-code" aria-hidden="true" />
            </span>
          </a>
          </div>
        </nav>
      </div>
    </div>
  </article>
</div>

<!-- Original document modal -->
<ArticleModal bind:active={showModal} item={document} />

<!-- KG document modal -->
{#if kgDocument}
  <ArticleModal 
    bind:active={showKgModal} 
    item={kgDocument} 
    kgTerm={kgTerm}
    kgRank={kgRank}
  />
{/if}

<style lang="scss">
  button {
    background: none;
    border: none;
    padding: 0;
    font: inherit;
    cursor: pointer;
    outline: inherit;
    display: block;
  }
  
  .tag-button {
    background: none;
    border: none;
    padding: 0;
    cursor: pointer;
    outline: inherit;
    display: inline-block;
    
    &:hover {
      opacity: 0.8;
    }
    
    &:disabled {
      opacity: 0.5;
      cursor: not-allowed;
    }
  }
  
  .title {
    font-size: 1.3em;
    margin-bottom: 0px;

    &:hover,
    &:focus {
      text-decoration: underline;
    }
  }
  
  .description {
    margin-top: 0.5rem;
  }
  
  .description-label {
    font-weight: 600;
    color: #666;
    margin-right: 0.5rem;
  }
  
  .description-content {
    display: inline;
    
    // Style markdown content within description
    :global(p) {
      display: inline;
      margin: 0;
    }
    
    :global(strong) {
      font-weight: 600;
    }
    
    :global(em) {
      font-style: italic;
    }
    
    :global(code) {
      background-color: #f5f5f5;
      padding: 0.1rem 0.3rem;
      border-radius: 3px;
      font-family: monospace;
      font-size: 0.9em;
    }
    
    :global(a) {
      color: #3273dc;
      text-decoration: none;
      
      &:hover {
        text-decoration: underline;
      }
    }
  }
  
  .no-description {
    color: #999;
    font-style: italic;
  }
  
  /* AI Summary Styling */
  .ai-summary-section {
    margin-top: 0.75rem;
  }
  
  .ai-summary-button {
    margin-top: 0.5rem;
  }
  
  .ai-summary-loading {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin-top: 0.5rem;
    color: #3273dc;
  }
  
  .ai-summary-error {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin-top: 0.5rem;
  }
  
  .ai-summary {
    margin-top: 0.75rem;
    padding: 0.75rem;
    background-color: #f8f9fa;
    border-left: 4px solid #3273dc;
    border-radius: 4px;
  }
  
  .ai-summary-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 0.5rem;
  }
  
  .ai-summary-label {
    display: flex;
    align-items: center;
    gap: 0.25rem;
    font-weight: 600;
    color: #3273dc;
  }
  
  .ai-summary-content {
    margin-bottom: 0.5rem;
    
    // Style markdown content within AI summary
    :global(p) {
      margin: 0 0 0.5rem 0;
      line-height: 1.4;
    }
    
    :global(p:last-child) {
      margin-bottom: 0;
    }
    
    :global(strong) {
      font-weight: 600;
    }
    
    :global(em) {
      font-style: italic;
    }
    
    :global(code) {
      background-color: #e8e8e8;
      padding: 0.1rem 0.3rem;
      border-radius: 3px;
      font-family: monospace;
      font-size: 0.9em;
    }
    
    :global(a) {
      color: #3273dc;
      text-decoration: none;
      
      &:hover {
        text-decoration: underline;
      }
    }
  }
  
  .ai-summary-actions {
    display: flex;
    justify-content: flex-end;
  }
</style>
