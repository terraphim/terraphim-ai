<script lang="ts">
  import { onMount } from 'svelte';
  import { get } from 'svelte/store';
  import { role, is_tauri } from '../stores';
  import { CONFIG } from '../../config';
  import BackButton from '../BackButton.svelte';
  import { invoke } from '@tauri-apps/api/tauri';

  type ChatMessage = { role: 'system' | 'user' | 'assistant'; content: string };
  type ChatResponse = { status: string; message?: string; model_used?: string; error?: string };
  type ContextItem = {
    id: string;
    title: string;
    content: string;
    context_type: string;
    created_at: string;
    relevance_score?: number;
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
        if (result.status === 'Success' && result.conversationId) {
          conversationId = result.conversationId;
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
    if (!conversationId) return;

    loadingContext = true;
    try {
      if ($is_tauri) {
        const result = await invoke('get_conversation', { conversationId });
        if (result.status === 'Success' && result.conversation) {
          contextItems = result.conversation.global_context || [];
        }
      } else {
        const response = await fetch(`${CONFIG.ServerURL}/conversations/${conversationId}`);
        if (response.ok) {
          const data = await response.json();
          if (data.status === 'Success' && data.conversation) {
            contextItems = data.conversation.global_context || [];
          }
        }
      }
    } catch (error) {
      console.error('❌ Error loading conversation context:', error);
    } finally {
      loadingContext = false;
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

  onMount(() => {
    // seed with a friendly greeting
    messages = [
      { role: 'assistant', content: 'Hi! How can I help you? Ask me anything about your search results or documents.' }
    ];

    // Initialize conversation and load context
    initializeConversation();
  });
</script>

<BackButton fallbackPath="/" />

<section class="section">
  <div class="container">
    <div class="columns">
      <!-- Main Chat Area -->
      <div class="column is-8">
        <h2 class="title is-4">Chat</h2>
        <p class="subtitle is-6">Role: {get(role)}</p>
        {#if conversationId}
          <p class="is-size-7 has-text-grey">Conversation ID: {conversationId}</p>
        {/if}

        <div class="chat-window">
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
            <textarea class="textarea" rows="3" bind:value={input} on:keydown={handleKeydown} placeholder="Type your message and press Enter..." />
          </div>
          <div class="control">
            <button class="button is-primary" on:click={sendMessage} disabled={sending || !input.trim()}>
              <span class="icon"><i class="fas fa-paper-plane"></i></span>
            </button>
          </div>
        </div>
      </div>

      <!-- Context Panel -->
      <div class="column is-4">
        <div class="box context-panel">
          <div class="level is-mobile">
            <div class="level-left">
              <div class="level-item">
                <h4 class="title is-5">Context</h4>
              </div>
            </div>
            <div class="level-right">
              <div class="level-item">
                <button
                  class="button is-small is-light"
                  on:click={loadConversationContext}
                  disabled={loadingContext}
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

          {#if contextItems.length === 0}
            <div class="has-text-centered has-text-grey-light">
              <span class="icon is-large">
                <i class="fas fa-inbox fa-2x"></i>
              </span>
              <p class="is-size-6">No context items yet</p>
              <p class="is-size-7">Add documents from search results to provide context for your chat.</p>
            </div>
          {:else}
            <div class="context-items">
              {#each contextItems as item, index}
                <div class="context-item" data-context-id={item.id}>
                  <div class="level is-mobile">
                    <div class="level-left">
                      <div class="level-item">
                        <span class="tag is-small {
                          item.context_type === 'Document' ? 'is-info' :
                          item.context_type === 'SearchResult' ? 'is-primary' :
                          item.context_type === 'UserInput' ? 'is-warning' : 'is-light'
                        }">
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
                    </div>
                  </div>

                  <h6 class="title is-6 has-text-dark">{item.title}</h6>

                  <div class="content is-small">
                    <p class="context-preview">
                      {item.content.substring(0, 150)}{item.content.length > 150 ? '...' : ''}
                    </p>
                  </div>

                  <div class="is-size-7 has-text-grey">
                    Added: {new Date(item.created_at).toLocaleString()}
                  </div>
                </div>
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
                  <span class="tag is-light is-small">
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
