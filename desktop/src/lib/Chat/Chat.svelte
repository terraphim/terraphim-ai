<script lang="ts">
  import { onMount } from 'svelte';
  import { get } from 'svelte/store';
  import { role, is_tauri } from '../stores';
  import { CONFIG } from '../../config';
  import BackButton from '../BackButton.svelte';
  import { invoke } from '@tauri-apps/api/tauri';
  import ContextEditModal from './ContextEditModal.svelte';
  import KGSearchModal from '../Search/KGSearchModal.svelte';
  import KGContextItem from '../Search/KGContextItem.svelte';

  type ChatMessage = { role: 'system' | 'user' | 'assistant'; content: string };
  type ChatResponse = { status: string; message?: string; model_used?: string; error?: string };
  type ContextItem = {
    id: string;
    title: string;
    summary?: string;
    content: string;
    context_type: string;
    created_at: string;
    relevance_score?: number;
    metadata?: { [key: string]: string };
  };
  type Conversation = {
    id: string;
    title: string;
    role: string;
    messages: any[];
    global_context: ContextItem[];
    created_at: string;
    updated_at: string;
  };

  let messages: ChatMessage[] = [];
  let input: string = '';
  let sending = false;
  let error: string | null = null;
  let modelUsed: string | null = null;

  // Conversation and context management
  let conversationId: string | null = null;
  let contextItems: ContextItem[] = [];
  let loadingContext = false;
  let showContextPanel = false;

  // Manual context addition
  let showAddContextForm = false;
  let newContextTitle = '';
  let newContextContent = '';
  let newContextType = 'document';
  let savingContext = false;

  // Context editing
  let showContextEditModal = false;
  let editingContext: ContextItem | null = null;
  let contextEditMode: 'create' | 'edit' = 'edit';
  let deletingContextId: string | null = null;

  // KG search modal
  let showKGSearchModal = false;

  function addUserMessage(text: string) {
    messages = [...messages, { role: 'user', content: text }];
  }

  // Load or create a conversation
  async function initializeConversation() {
    try {
      if ($is_tauri) {
        // Try to get existing conversations
        const result = await invoke('list_conversations');
        if (result?.conversations && result.conversations.length > 0) {
          // Use the most recent conversation
          conversationId = result.conversations[0].id;
          console.log('🎯 Using existing conversation:', conversationId);
          await loadConversationContext();
        } else {
          // Create a new conversation
          await createNewConversation();
        }
      } else {
        // Web mode - HTTP API
        const response = await fetch(`${CONFIG.ServerURL}/conversations`);
        if (response.ok) {
          const data = await response.json();
          if (data.conversations && data.conversations.length > 0) {
            conversationId = data.conversations[0].id;
            console.log('🎯 Using existing conversation:', conversationId);
            await loadConversationContext();
          } else {
            await createNewConversation();
          }
        } else {
          await createNewConversation();
        }
      }
    } catch (error) {
      console.error('❌ Error initializing conversation:', error);
    }
  }

  // Create a new conversation
  async function createNewConversation() {
    try {
      const currentRole = get(role) as string;

      if ($is_tauri) {
        const result = await invoke('create_conversation', {
          title: 'Chat Conversation',
          role: currentRole
        });
        if (result.status === 'success' && result.conversation_id) {
          conversationId = result.conversation_id;
          console.log('🆕 Created new conversation:', conversationId);
        }
      } else {
        const response = await fetch(`${CONFIG.ServerURL}/conversations`, {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({
            title: 'Chat Conversation',
            role: currentRole
          })
        });
        if (response.ok) {
          const data = await response.json();
          if (data.status === 'Success' && data.conversation_id) {
            conversationId = data.conversation_id;
            console.log('🆕 Created new conversation:', conversationId);
          }
        }
      }
    } catch (error) {
      console.error('❌ Error creating conversation:', error);
    }
  }

  // Load context for the current conversation
  async function loadConversationContext() {
    if (!conversationId) {
      console.warn('⚠️ Cannot load context: no conversation ID available');
      return;
    }

    loadingContext = true;
    console.log('🔄 Loading conversation context for:', conversationId);

    try {
      if ($is_tauri) {
        console.log('📱 Loading context via Tauri...');
        const result = await invoke('get_conversation', { conversationId });

        console.log('📥 Tauri response:', result);

        if (result.status === 'success' && result.conversation) {
          const newContextItems = result.conversation.global_context || [];
          contextItems = newContextItems;
          console.log(`✅ Loaded ${newContextItems.length} context items via Tauri`);
        } else {
          console.error('❌ Failed to get conversation via Tauri:', result.error || 'Unknown error');
          contextItems = [];
        }
      } else {
        console.log('🌐 Loading context via HTTP...');
        const response = await fetch(`${CONFIG.ServerURL}/conversations/${conversationId}`);

        console.log('📥 HTTP response status:', response.status, response.statusText);

        if (response.ok) {
          const data = await response.json();
          console.log('📄 HTTP response data:', data);

          if (data.status === 'success' && data.conversation) {
            const newContextItems = data.conversation.global_context || [];
            contextItems = newContextItems;
            console.log(`✅ Loaded ${newContextItems.length} context items via HTTP`);
          } else {
            console.error('❌ Failed to get conversation via HTTP:', data.error || 'Unknown error');
            contextItems = [];
          }
        } else {
          console.error('❌ HTTP request failed:', response.status, response.statusText);
          contextItems = [];
        }
      }
    } catch (error) {
      console.error('❌ Error loading conversation context:', {
        error: error.message || error,
        conversationId,
        isTauri: $is_tauri,
        timestamp: new Date().toISOString()
      });
      contextItems = [];
    } finally {
      loadingContext = false;
      console.log('🏁 Context loading completed. Items count:', contextItems.length);
    }
  }

  // Toggle manual context form
  function toggleAddContextForm() {
    showAddContextForm = !showAddContextForm;
    if (!showAddContextForm) {
      // Reset form
      newContextTitle = '';
      newContextContent = '';
      newContextType = 'document';
    }
  }

  // Add manual context
  async function addManualContext() {
    if (!conversationId || !newContextTitle.trim() || !newContextContent.trim()) return;

    savingContext = true;
    try {
      const contextData = {
        title: newContextTitle.trim(),
        summary: null,
        content: newContextContent.trim(),
        context_type: newContextType
      };

      if ($is_tauri) {
        const result = await invoke('add_context_to_conversation', {
          conversationId,
          contextData
        });
        if (result.status === 'success') {
          await loadConversationContext();
          toggleAddContextForm();

          // Show success notification
          console.log('✅ Context added successfully via Tauri');
        } else {
          console.error('❌ Failed to add context via Tauri:', result.error);
        }
      } else {
        const response = await fetch(`${CONFIG.ServerURL}/conversations/${conversationId}/context`, {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify(contextData)
        });
        if (response.ok) {
          const data = await response.json();
          if (data.status === 'success') {
            await loadConversationContext();
            toggleAddContextForm();

            // Show success notification
            console.log('✅ Context added successfully via HTTP');
          } else {
            console.error('❌ Failed to add context via HTTP:', data.error);
          }
        } else {
          console.error('❌ HTTP request failed:', response.status, response.statusText);
        }
      }
    } catch (error) {
      console.error('❌ Error adding manual context:', error);
    } finally {
      savingContext = false;
    }
  }

  // Edit context functionality
  function editContext(context: ContextItem) {
    editingContext = context;
    contextEditMode = 'edit';
    showContextEditModal = true;
  }

  // Delete context with confirmation
  function confirmDeleteContext(context: ContextItem) {
    if (confirm(`Are you sure you want to delete "${context.title}"?`)) {
      deleteContext(context.id);
    }
  }

  // Delete context
  async function deleteContext(contextId: string) {
    if (!conversationId || deletingContextId) return;

    deletingContextId = contextId;
    console.log('🗑️ Deleting context:', contextId);

    try {
      if ($is_tauri) {
        const result = await invoke('delete_context', {
          conversationId,
          contextId
        });
        if (result?.status === 'success') {
          console.log('✅ Context deleted successfully via Tauri');
          await loadConversationContext();
        } else {
          console.error('❌ Failed to delete context via Tauri:', result?.error);
        }
      } else {
        const response = await fetch(`${CONFIG.ServerURL}/conversations/${conversationId}/context/${contextId}`, {
          method: 'DELETE',
          headers: { 'Content-Type': 'application/json' }
        });
        if (response.ok) {
          const data = await response.json();
          if (data.status === 'success') {
            console.log('✅ Context deleted successfully via HTTP');
            await loadConversationContext();
          } else {
            console.error('❌ Failed to delete context via HTTP:', data.error);
          }
        } else {
          console.error('❌ HTTP delete request failed:', response.status);
        }
      }
    } catch (error) {
      console.error('❌ Error deleting context:', error);
    } finally {
      deletingContextId = null;
    }
  }

  // Update context
  async function updateContext(updatedContext: ContextItem) {
    if (!conversationId) return;

    console.log('📝 Updating context:', updatedContext.id);

    try {
      const updatePayload = {
        context_type: updatedContext.context_type,
        title: updatedContext.title,
        summary: updatedContext.summary,
        content: updatedContext.content,
        metadata: updatedContext.metadata
      };

      if ($is_tauri) {
        const result = await invoke('update_context', {
          conversationId,
          contextId: updatedContext.id,
          request: updatePayload
        });
        if (result?.status === 'success') {
          console.log('✅ Context updated successfully via Tauri');
          await loadConversationContext();
        } else {
          console.error('❌ Failed to update context via Tauri:', result?.error);
        }
      } else {
        const response = await fetch(`${CONFIG.ServerURL}/conversations/${conversationId}/context/${updatedContext.id}`, {
          method: 'PUT',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify(updatePayload)
        });
        if (response.ok) {
          const data = await response.json();
          if (data.status === 'success') {
            console.log('✅ Context updated successfully via HTTP');
            await loadConversationContext();
          } else {
            console.error('❌ Failed to update context via HTTP:', data.error);
          }
        } else {
          console.error('❌ HTTP update request failed:', response.status);
        }
      }
    } catch (error) {
      console.error('❌ Error updating context:', error);
    }
  }

  async function sendMessage() {
    if (!input.trim() || sending) return;
    error = null;
    const currentRole = get(role) as string;
    const userText = input.trim();
    input = '';

    // Ensure we have a conversation
    if (!conversationId) {
      await initializeConversation();
    }

    addUserMessage(userText);
    sending = true;
    try {
      const requestBody: any = { role: currentRole, messages };

      // Include conversation_id if we have one
      if (conversationId) {
        requestBody.conversation_id = conversationId;
      }

      const res = await fetch(`${CONFIG.ServerURL}/chat`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(requestBody)
      });
      if (!res.ok) {
        throw new Error(`HTTP ${res.status}`);
      }
      const data: ChatResponse = await res.json();
      modelUsed = data.model_used ?? null;
      if (data.status?.toLowerCase() === 'success' && data.message) {
        messages = [...messages, { role: 'assistant', content: data.message }];
      } else {
        error = data.error || 'Chat failed';
      }
    } catch (e: any) {
      error = e?.message || String(e);
    } finally {
      sending = false;
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if ((e.key === 'Enter' || e.key === 'Return') && !e.shiftKey) {
      e.preventDefault();
      sendMessage();
    }
  }

  // KG search modal handlers
  function openKGSearch() {
    showKGSearchModal = true;
  }

  function handleKGTermAdded(event: CustomEvent) {
    console.log('✅ KG term added to context:', event.detail.term);
    // Reload context to show the new KG term
    loadConversationContext();
  }

  function handleKGIndexAdded(event: CustomEvent) {
    console.log('✅ KG index added to context for role:', event.detail.role);
    // Reload context to show the new KG index
    loadConversationContext();
  }

  onMount(() => {
    // seed with a friendly greeting
    messages = [
      { role: 'assistant', content: 'Hi! How can I help you? Ask me anything about your search results or documents.' }
    ];

    // Initialize conversation and load context
    initializeConversation();

    // Refresh context when navigating to chat page
    if (typeof window !== 'undefined') {
      const handleFocus = () => {
        // Refresh context when window regains focus (user comes back to chat)
        if (conversationId) {
          loadConversationContext();
        }
      };

      window.addEventListener('focus', handleFocus);

      // Cleanup
      return () => {
        window.removeEventListener('focus', handleFocus);
      };
    }
  });
</script>

<BackButton fallbackPath="/" />

<section class="section" data-testid="chat-interface">
  <div class="container">
    <div class="columns">
      <!-- Main Chat Area -->
      <div class="column is-8">
        <h2 class="title is-4">Chat</h2>
        <p class="subtitle is-6">Role: {get(role)}</p>
        {#if conversationId}
          <p class="is-size-7 has-text-grey">Conversation ID: {conversationId}</p>
        {/if}

        <div class="chat-window" data-testid="chat-messages">
      {#each messages as m, i}
        <div class={`msg ${m.role}`}>
          <div class="bubble">
            <pre>{m.content}</pre>
          </div>
        </div>
      {/each}
      {#if sending}
        <div class="msg assistant">
          <div class="bubble loading">
            <span class="icon is-small"><i class="fas fa-spinner fa-spin"></i></span>
            <span>Thinking...</span>
          </div>
        </div>
      {/if}
    </div>

    {#if modelUsed}
      <p class="is-size-7 has-text-grey">Model: {modelUsed}</p>
    {/if}
    {#if error}
      <p class="has-text-danger is-size-7">{error}</p>
    {/if}

        <div class="field has-addons chat-input">
          <div class="control is-expanded">
            <textarea class="textarea" rows="3" bind:value={input} on:keydown={handleKeydown} placeholder="Type your message and press Enter..." data-testid="chat-input" />
          </div>
          <div class="control">
            <button class="button is-primary" on:click={sendMessage} disabled={sending || !input.trim()} data-testid="send-message-button">
              <span class="icon"><i class="fas fa-paper-plane"></i></span>
            </button>
          </div>
        </div>
      </div>

      <!-- Context Panel -->
      <div class="column is-4">
        <div class="box context-panel" data-testid="context-panel">
          <div class="level is-mobile">
            <div class="level-left">
              <div class="level-item">
  <h4 class="title is-5">Context</h4>
                <div class="buttons has-addons">
                  <button class="button is-small is-primary" data-testid="show-add-context-button" on:click={toggleAddContextForm}>
                    <span class="icon is-small">
                      <i class="fas fa-plus"></i>
                    </span>
                    <span>Add</span>
                  </button>
                  <button class="button is-small is-info" data-testid="kg-search-button" on:click={openKGSearch}>
                    <span class="icon is-small">
                      <i class="fas fa-sitemap"></i>
                    </span>
                    <span>KG Search</span>
                  </button>
                </div>
              </div>
            </div>
            <div class="level-right">
              <div class="level-item">
                <button
                  class="button is-small is-light"
                  on:click={loadConversationContext}
                  disabled={loadingContext}
                  data-testid="refresh-context-button"
                >
                  {#if loadingContext}
                    <span class="icon is-small">
                      <i class="fas fa-spinner fa-spin"></i>
                    </span>
                  {:else}
                    <span class="icon is-small">
                      <i class="fas fa-refresh"></i>
                    </span>
                  {/if}
                </button>
              </div>
            </div>
          </div>

          <!-- Manual Context Addition Form -->
          {#if showAddContextForm}
            <div class="box has-background-light mb-4" data-testid="add-context-form">
              <div class="field">
                <label class="label is-small">Context Type</label>
                <div class="control">
                  <div class="select is-small is-fullwidth">
                    <select bind:value={newContextType} data-testid="context-type-select">
                      <option value="document">Document</option>
                      <option value="search_result">Search Result</option>
                      <option value="user_input">User Input</option>
                      <option value="note">Note</option>
                    </select>
                  </div>
                </div>
              </div>

              <div class="field">
                <label class="label is-small">Title</label>
                <div class="control">
                  <input class="input is-small" type="text" placeholder="Enter context title" bind:value={newContextTitle} data-testid="context-title-input" />
                </div>
              </div>

              <div class="field">
                <label class="label is-small">Content</label>
                <div class="control">
                  <textarea class="textarea is-small" rows="4" placeholder="Enter context content" bind:value={newContextContent} data-testid="context-content-textarea"></textarea>
                </div>
              </div>

              <div class="field is-grouped">
                <div class="control">
                  <button class="button is-primary is-small" on:click={addManualContext} disabled={savingContext || !newContextTitle.trim() || !newContextContent.trim()} data-testid="add-context-submit-button">
                    {#if savingContext}
                      <span class="icon is-small"><i class="fas fa-spinner fa-spin"></i></span>
                    {:else}
                      <span class="icon is-small"><i class="fas fa-save"></i></span>
                    {/if}
                    <span>Save Context</span>
                  </button>
                </div>
                <div class="control">
                  <button class="button is-light is-small" on:click={toggleAddContextForm} disabled={savingContext}>
                    <span class="icon is-small"><i class="fas fa-times"></i></span>
                    <span>Cancel</span>
                  </button>
                </div>
              </div>
            </div>
          {/if}

          {#if contextItems.length === 0}
            <div class="has-text-centered has-text-grey-light" data-testid="empty-context-message">
              <span class="icon is-large">
                <i class="fas fa-inbox fa-2x"></i>
              </span>
              <p class="is-size-6">No context items yet</p>
              <p class="is-size-7">Add documents from search results to provide context for your chat.</p>
            </div>
          {:else}
            <div class="context-items" data-testid="conversation-context">
              {#each contextItems as item, index}
                {#if item.context_type === 'KGTermDefinition' || item.context_type === 'KGIndex'}
                  <!-- Use specialized KG context item component -->
                  <KGContextItem
                    contextItem={item}
                    compact={true}
                    on:remove={e => deleteContext(e.detail.contextId)}
                    on:viewDetails={e => editContext(e.detail.contextItem)}
                  />
                {:else}
                  <!-- Use default context item rendering for non-KG items -->
                  <div class="context-item" data-context-id={item.id} data-testid={`context-item-${index}`} data-context-type={item.context_type}>
                    <div class="level is-mobile">
                      <div class="level-left">
                        <div class="level-item">
                          <span class="tag is-small {
                            item.context_type === 'Document' ? 'is-info' :
                            item.context_type === 'SearchResult' ? 'is-primary' :
                            item.context_type === 'UserInput' ? 'is-warning' : 'is-light'
                          }" data-testid={`context-type-${index}`}>
                            {item.context_type.replace(/([A-Z])/g, ' $1').trim()}
                          </span>
                        </div>
                      </div>
                      <div class="level-right">
                        <div class="level-item">
                          {#if item.relevance_score}
                            <span class="tag is-light is-small">
                              {item.relevance_score.toFixed(1)}
                            </span>
                          {/if}
                        </div>
                        <div class="level-item context-actions">
                          <div class="field is-grouped">
                            <div class="control">
                              <button
                                class="button is-small is-light"
                                on:click={() => editContext(item)}
                                data-testid={`edit-context-${index}`}
                                title="Edit context"
                              >
                                <span class="icon is-small">
                                  <i class="fas fa-edit"></i>
                                </span>
                              </button>
                            </div>
                            <div class="control">
                              <button
                                class="button is-small is-light is-danger"
                                on:click={() => confirmDeleteContext(item)}
                                data-testid={`delete-context-${index}`}
                                title="Delete context"
                              >
                                <span class="icon is-small">
                                  <i class="fas fa-trash"></i>
                                </span>
                              </button>
                            </div>
                          </div>
                        </div>
                      </div>
                    </div>

                    <h6 class="title is-6 has-text-dark" data-testid={`context-title-${index}`}>{item.title}</h6>

                    <div class="content is-small">
                      {#if item.summary}
                        <p class="context-summary" data-testid={`context-summary-${index}`}>
                          {item.summary}
                        </p>
                      {:else}
                        <p class="context-preview" data-testid={`context-content-${index}`}>
                          {item.content.substring(0, 150)}{item.content.length > 150 ? '...' : ''}
                        </p>
                      {/if}
                    </div>

                    <div class="is-size-7 has-text-grey">
                      Added: {new Date(item.created_at).toLocaleString()}
                    </div>
                  </div>
                {/if}

                {#if index < contextItems.length - 1}
                  <hr class="context-divider">
                {/if}
              {/each}
            </div>
          {/if}

          <div class="mt-4">
            <div class="level is-mobile">
              <div class="level-left">
                <div class="level-item">
                  <span class="tag is-light is-small" data-testid="context-summary">
                    {contextItems.length} context items
                  </span>
                </div>
              </div>
              <div class="level-right">
                <div class="level-item">
                  <span class="is-size-7 has-text-grey">
                    Context is automatically included in your chat
                  </span>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</section>

<!-- Context Edit Modal -->
<ContextEditModal
  bind:active={showContextEditModal}
  context={editingContext}
  mode={contextEditMode}
  on:update={e => updateContext(e.detail)}
  on:delete={e => deleteContext(e.detail)}
  on:close={() => {
    showContextEditModal = false;
    editingContext = null;
  }}
/>

<!-- KG Search Modal -->
<KGSearchModal
  bind:active={showKGSearchModal}
  conversationId={conversationId}
  on:termAdded={handleKGTermAdded}
  on:kgIndexAdded={handleKGIndexAdded}
/>

<style>
  .chat-window {
    border: 1px solid #ececec;
    border-radius: 6px;
    padding: 0.75rem;
    height: 50vh;
    overflow: auto;
    background: #fff;
    margin-bottom: 0.75rem;
  }
  .msg { display: flex; margin-bottom: 0.5rem; }
  .msg.user { justify-content: flex-end; }
  .msg.assistant { justify-content: flex-start; }
  .bubble { max-width: 70ch; padding: 0.5rem 0.75rem; border-radius: 12px; }
  .user .bubble { background: #3273dc; color: #fff; }
  .assistant .bubble { background: #f5f5f5; color: #333; }
  .bubble pre { white-space: pre-wrap; word-wrap: break-word; margin: 0; font-family: inherit; }
  .loading { display: inline-flex; gap: 0.5rem; align-items: center; }
  .chat-input { align-items: flex-end; }

  /* Context Panel Styles */
  .context-panel {
    max-height: 70vh;
    overflow-y: auto;
    background: #fafafa;
  }

  .context-items {
    max-height: 50vh;
    overflow-y: auto;
  }

  .context-item {
    padding: 0.75rem 0;
    transition: background-color 0.2s ease;
  }

  .context-item:hover {
    background-color: rgba(0, 0, 0, 0.02);
    border-radius: 6px;
    padding: 0.75rem;
    margin: 0 -0.75rem;
  }

  .context-preview {
    line-height: 1.4;
    color: #666;
    margin-bottom: 0.5rem;
  }

  .context-summary {
    line-height: 1.4;
    color: #333;
    font-weight: 500;
    margin-bottom: 0.5rem;
    font-style: italic;
  }

  .context-actions {
    opacity: 0;
    transition: opacity 0.2s ease;
  }

  .context-item:hover .context-actions {
    opacity: 1;
  }

  .context-divider {
    margin: 0.5rem 0;
    background-color: #e8e8e8;
  }

  @media screen and (max-width: 768px) {
    .columns {
      display: block;
    }

    .context-panel {
      margin-top: 1rem;
      max-height: 40vh;
    }
  }
</style>
