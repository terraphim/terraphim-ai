<script lang="ts">
  import { onMount } from 'svelte';
  import { get } from 'svelte/store';
  import { role } from '../stores';
  import { CONFIG } from '../../config';
  import BackButton from '../BackButton.svelte';

  type ChatMessage = { role: 'system' | 'user' | 'assistant'; content: string };
  type ChatResponse = { status: string; message?: string; model_used?: string; error?: string };

  let messages: ChatMessage[] = [];
  let input: string = '';
  let sending = false;
  let error: string | null = null;
  let modelUsed: string | null = null;

  function addUserMessage(text: string) {
    messages = [...messages, { role: 'user', content: text }];
  }

  async function sendMessage() {
    if (!input.trim() || sending) return;
    error = null;
    const currentRole = get(role) as string;
    const userText = input.trim();
    input = '';

    addUserMessage(userText);
    sending = true;
    try {
      const res = await fetch(`${CONFIG.ServerURL}/chat`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ role: currentRole, messages })
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
  });
</script>

<BackButton fallbackPath="/" />

<section class="section">
  <div class="container">
    <h2 class="title is-4">Chat</h2>
    <p class="subtitle is-6">Role: {get(role)}</p>

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
</style>


