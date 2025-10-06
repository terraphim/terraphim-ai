<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/tauri';
  import { role } from '../stores';
  import { get } from 'svelte/store';

  // Types
  type ConversationSummary = {
    id: string;
    title: string;
    role: string;
    message_count: number;
    preview: string | null;
    created_at: string;
    updated_at: string;
  };

  type ListPersistentConversationsResponse = {
    status: 'success' | 'error';
    conversations: ConversationSummary[];
    total: number;
    error: string | null;
  };

  type DeletePersistentConversationResponse = {
    status: 'success' | 'error';
    error: string | null;
  };

  // Props
  export let currentConversationId: string | null = null;
  export let onSelectConversation: (conversationId: string) => void = () => {};
  export let onNewConversation: () => void = () => {};

  // State
  let conversations: ConversationSummary[] = [];
  let loading = false;
  let error: string | null = null;
  let searchQuery = '';
  let filterRole: string | null = null;
  let showDeleteConfirm: string | null = null;
  let deleting = false;

  // Computed
  $: filteredConversations = conversations.filter(conv => {
    const matchesSearch = !searchQuery || 
      conv.title.toLowerCase().includes(searchQuery.toLowerCase()) ||
      (conv.preview && conv.preview.toLowerCase().includes(searchQuery.toLowerCase()));
    const matchesRole = !filterRole || conv.role === filterRole;
    return matchesSearch && matchesRole;
  });

  // Load conversations on mount
  onMount(() => {
    loadConversations();
  });

  // Load conversations from backend
  async function loadConversations() {
    loading = true;
    error = null;
    
    try {
      const response = await invoke<ListPersistentConversationsResponse>(
        'list_persistent_conversations',
        { 
          role: filterRole,
          limit: 100 
        }
      );

      if (response.status === 'success') {
        conversations = response.conversations;
      } else {
        error = response.error || 'Failed to load conversations';
      }
    } catch (e) {
      console.error('Failed to load conversations:', e);
      error = String(e);
    } finally {
      loading = false;
    }
  }

  // Search conversations
  async function searchConversations() {
    if (!searchQuery.trim()) {
      await loadConversations();
      return;
    }

    loading = true;
    error = null;

    try {
      const response = await invoke<ListPersistentConversationsResponse>(
        'search_persistent_conversations',
        { query: searchQuery }
      );

      if (response.status === 'success') {
        conversations = response.conversations;
      } else {
        error = response.error || 'Search failed';
      }
    } catch (e) {
      console.error('Search failed:', e);
      error = String(e);
    } finally {
      loading = false;
    }
  }

  // Delete conversation
  async function deleteConversation(conversationId: string) {
    deleting = true;
    error = null;

    try {
      const response = await invoke<DeletePersistentConversationResponse>(
        'delete_persistent_conversation',
        { conversationId }
      );

      if (response.status === 'success') {
        // Remove from local list
        conversations = conversations.filter(c => c.id !== conversationId);
        showDeleteConfirm = null;
        
        // If we deleted the current conversation, trigger new conversation
        if (currentConversationId === conversationId) {
          onNewConversation();
        }
      } else {
        error = response.error || 'Failed to delete conversation';
      }
    } catch (e) {
      console.error('Failed to delete conversation:', e);
      error = String(e);
    } finally {
      deleting = false;
    }
  }

  // Format date for display
  function formatDate(dateStr: string): string {
    const date = new Date(dateStr);
    const now = new Date();
    const diffMs = now.getTime() - date.getTime();
    const diffMins = Math.floor(diffMs / 60000);
    const diffHours = Math.floor(diffMs / 3600000);
    const diffDays = Math.floor(diffMs / 86400000);

    if (diffMins < 1) return 'Just now';
    if (diffMins < 60) return `${diffMins}m ago`;
    if (diffHours < 24) return `${diffHours}h ago`;
    if (diffDays < 7) return `${diffDays}d ago`;
    
    return date.toLocaleDateString();
  }

  // Handle search input
  function handleSearchInput() {
    if (searchQuery.trim()) {
      searchConversations();
    } else {
      loadConversations();
    }
  }

  // Handle role filter change
  function handleRoleFilterChange() {
    loadConversations();
  }
</script>

<div class="session-list">
  <!-- Header -->
  <div class="session-list-header">
    <h3>Chat History</h3>
    <button 
      class="btn btn-sm btn-primary new-chat-btn" 
      on:click={onNewConversation}
      title="Start new conversation"
    >
      <i class="bi bi-plus-lg"></i> New Chat
    </button>
  </div>

  <!-- Search and Filter -->
  <div class="session-list-controls">
    <div class="search-box">
      <input
        type="text"
        class="form-control form-control-sm"
        placeholder="Search conversations..."
        bind:value={searchQuery}
        on:input={handleSearchInput}
      />
      <i class="bi bi-search search-icon"></i>
    </div>

    <div class="filter-box">
      <select 
        class="form-select form-select-sm" 
        bind:value={filterRole}
        on:change={handleRoleFilterChange}
      >
        <option value={null}>All Roles</option>
        <option value={$role}>{$role}</option>
      </select>
    </div>
  </div>

  <!-- Error Display -->
  {#if error}
    <div class="alert alert-danger alert-sm" role="alert">
      {error}
      <button 
        type="button" 
        class="btn-close btn-close-sm" 
        on:click={() => error = null}
      ></button>
    </div>
  {/if}

  <!-- Loading State -->
  {#if loading}
    <div class="loading-state">
      <div class="spinner-border spinner-border-sm" role="status">
        <span class="visually-hidden">Loading...</span>
      </div>
      <span>Loading conversations...</span>
    </div>
  {/if}

  <!-- Conversation List -->
  <div class="conversation-list">
    {#if !loading && filteredConversations.length === 0}
      <div class="empty-state">
        <i class="bi bi-chat-dots"></i>
        <p>No conversations yet</p>
        <button class="btn btn-sm btn-outline-primary" on:click={onNewConversation}>
          Start your first chat
        </button>
      </div>
    {:else}
      {#each filteredConversations as conversation (conversation.id)}
        <div 
          class="conversation-item"
          class:active={currentConversationId === conversation.id}
          on:click={() => onSelectConversation(conversation.id)}
          on:keydown={(e) => e.key === 'Enter' && onSelectConversation(conversation.id)}
          role="button"
          tabindex="0"
        >
          <div class="conversation-header">
            <h4 class="conversation-title">{conversation.title}</h4>
            <div class="conversation-actions">
              {#if showDeleteConfirm === conversation.id}
                <button
                  class="btn btn-sm btn-danger"
                  on:click|stopPropagation={() => deleteConversation(conversation.id)}
                  disabled={deleting}
                  title="Confirm delete"
                >
                  <i class="bi bi-check-lg"></i>
                </button>
                <button
                  class="btn btn-sm btn-secondary"
                  on:click|stopPropagation={() => showDeleteConfirm = null}
                  disabled={deleting}
                  title="Cancel"
                >
                  <i class="bi bi-x-lg"></i>
                </button>
              {:else}
                <button
                  class="btn btn-sm btn-ghost"
                  on:click|stopPropagation={() => showDeleteConfirm = conversation.id}
                  title="Delete conversation"
                >
                  <i class="bi bi-trash"></i>
                </button>
              {/if}
            </div>
          </div>

          <div class="conversation-meta">
            <span class="badge bg-secondary">{conversation.role}</span>
            <span class="message-count">
              <i class="bi bi-chat-left-text"></i> {conversation.message_count}
            </span>
            <span class="timestamp">{formatDate(conversation.updated_at)}</span>
          </div>

          {#if conversation.preview}
            <p class="conversation-preview">{conversation.preview}</p>
          {/if}
        </div>
      {/each}
    {/if}
  </div>

  <!-- Footer with Stats -->
  <div class="session-list-footer">
    <small class="text-muted">
      {filteredConversations.length} conversation{filteredConversations.length !== 1 ? 's' : ''}
    </small>
    <button 
      class="btn btn-sm btn-ghost" 
      on:click={loadConversations}
      title="Refresh"
    >
      <i class="bi bi-arrow-clockwise"></i>
    </button>
  </div>
</div>

<style>
  .session-list {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: var(--bs-body-bg);
    border-right: 1px solid var(--bs-border-color);
  }

  .session-list-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 1rem;
    border-bottom: 1px solid var(--bs-border-color);
  }

  .session-list-header h3 {
    margin: 0;
    font-size: 1.25rem;
    font-weight: 600;
  }

  .new-chat-btn {
    display: flex;
    align-items: center;
    gap: 0.25rem;
  }

  .session-list-controls {
    padding: 0.75rem 1rem;
    border-bottom: 1px solid var(--bs-border-color);
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .search-box {
    position: relative;
  }

  .search-box input {
    padding-right: 2rem;
  }

  .search-icon {
    position: absolute;
    right: 0.75rem;
    top: 50%;
    transform: translateY(-50%);
    color: var(--bs-secondary);
    pointer-events: none;
  }

  .loading-state {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;
    padding: 2rem;
    color: var(--bs-secondary);
  }

  .conversation-list {
    flex: 1;
    overflow-y: auto;
    padding: 0.5rem;
  }

  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 3rem 1rem;
    text-align: center;
    color: var(--bs-secondary);
  }

  .empty-state i {
    font-size: 3rem;
    margin-bottom: 1rem;
    opacity: 0.5;
  }

  .empty-state p {
    margin-bottom: 1rem;
  }

  .conversation-item {
    padding: 0.75rem;
    margin-bottom: 0.5rem;
    border-radius: 0.375rem;
    cursor: pointer;
    transition: background-color 0.2s;
    border: 1px solid transparent;
  }

  .conversation-item:hover {
    background-color: var(--bs-secondary-bg);
  }

  .conversation-item.active {
    background-color: var(--bs-primary-bg-subtle);
    border-color: var(--bs-primary);
  }

  .conversation-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    margin-bottom: 0.5rem;
  }

  .conversation-title {
    margin: 0;
    font-size: 0.9rem;
    font-weight: 600;
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .conversation-actions {
    display: flex;
    gap: 0.25rem;
    opacity: 0;
    transition: opacity 0.2s;
  }

  .conversation-item:hover .conversation-actions {
    opacity: 1;
  }

  .conversation-meta {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-size: 0.75rem;
    margin-bottom: 0.5rem;
  }

  .message-count {
    display: flex;
    align-items: center;
    gap: 0.25rem;
    color: var(--bs-secondary);
  }

  .timestamp {
    color: var(--bs-secondary);
    margin-left: auto;
  }

  .conversation-preview {
    margin: 0;
    font-size: 0.8rem;
    color: var(--bs-secondary);
    overflow: hidden;
    text-overflow: ellipsis;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    -webkit-box-orient: vertical;
  }

  .session-list-footer {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.75rem 1rem;
    border-top: 1px solid var(--bs-border-color);
  }

  .btn-ghost {
    background: transparent;
    border: none;
    color: var(--bs-secondary);
    padding: 0.25rem 0.5rem;
  }

  .btn-ghost:hover {
    background-color: var(--bs-secondary-bg);
    color: var(--bs-body-color);
  }

  .alert-sm {
    padding: 0.5rem 0.75rem;
    margin: 0.5rem 1rem;
    font-size: 0.875rem;
  }

  .btn-close-sm {
    font-size: 0.75rem;
  }

  /* Scrollbar styling */
  .conversation-list::-webkit-scrollbar {
    width: 8px;
  }

  .conversation-list::-webkit-scrollbar-track {
    background: var(--bs-body-bg);
  }

  .conversation-list::-webkit-scrollbar-thumb {
    background: var(--bs-border-color);
    border-radius: 4px;
  }

  .conversation-list::-webkit-scrollbar-thumb:hover {
    background: var(--bs-secondary);
  }
</style>
