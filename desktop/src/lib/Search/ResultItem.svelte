<script lang="ts">
  import { Taglist, Tag } from "svelma";
  import { fade } from "svelte/transition";
  import { router } from "tinro";
  import ArticleModal from "./ArticleModal.svelte";
  import AtomicSaveModal from "./AtomicSaveModal.svelte";
  import type { Document } from "./SearchResult";
  import { role, is_tauri, configStore as roleConfigStore } from "../stores";
  import { CONFIG } from "../../config";
  import { invoke } from '@tauri-apps/api/tauri';
  import type { DocumentListResponse, Role, Haystack } from "../generated/types";
  import type { ConversationListResponse, ConversationResponse, ContextMutationResponse } from "../Chat/types";
  import SvelteMarkdown from 'svelte-markdown';

  export let item: Document;
  let showModal = false;
  let showKgModal = false;
  let showAtomicSaveModal = false;
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

  // Context addition state
  let addingToContext = false;
  let contextAdded = false;
  let contextError: string | null = null;

  // Chat with document state
  let chattingWithDocument = false;
  let chatStarted = false;

  const toError = (value: unknown): Error =>
    value instanceof Error ? value : new Error(String(value));

  const resolveRoleName = (): string => {
    const current = $role as unknown;
    if (current && typeof current === 'object' && current !== null) {
      const withOriginal = current as { original?: string };
      return withOriginal.original ?? 'default';
    }
    if (typeof current === 'string') {
      return current;
    }
    return 'default';
  };

  function buildContextMetadata(): Record<string, string> {
    const metadata: Record<string, string> = {
      source_type: 'document',
      document_id: item.id
    };

    if (item.url) {
      metadata.url = item.url;
    }

    if (item.tags?.length) {
      metadata.tags = item.tags.join(', ');
    }

    if (item.rank !== undefined) {
      metadata.rank = String(item.rank);
    }

    return metadata;
  }

  async function getOrCreateConversationId(preferredTitle: string): Promise<string> {
    const targetRole = resolveRoleName();

    if ($is_tauri) {
      try {
        const result = await invoke<ConversationListResponse>('list_conversations');
        const existing = result?.conversations?.[0];
        if (existing?.id) {
          return existing.id;
        }
      } catch (error) {
        console.error('❌ Failed to list conversations via Tauri:', toError(error));
      }

      const created = await invoke<ConversationResponse>('create_conversation', {
        title: preferredTitle,
        role: targetRole
      });

      if (created.status === 'success' && created.conversation_id) {
        return created.conversation_id;
      }

      throw new Error(created.error ?? 'Unknown error creating conversation');
    }

    const baseUrl = CONFIG.ServerURL;

    try {
      const response = await fetch(`${baseUrl}/conversations`);
      if (response.ok) {
        const data: ConversationListResponse = await response.json();
        const existing = data.conversations?.[0];
        if (existing?.id) {
          return existing.id;
        }
      } else {
        console.warn('⚠️ Failed to list conversations:', response.status, response.statusText);
      }
    } catch (error) {
      console.error('❌ Error fetching conversations:', toError(error));
    }

    const createResponse = await fetch(`${baseUrl}/conversations`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        title: preferredTitle,
        role: targetRole
      })
    });

    if (!createResponse.ok) {
      throw new Error(`Failed to create conversation: ${createResponse.status} ${createResponse.statusText}`);
    }

    const created: ConversationResponse = await createResponse.json();

    if (created.status === 'success' && created.conversation_id) {
      return created.conversation_id;
    }

    throw new Error(created.error ?? 'Unknown error creating conversation');
  }

  async function addDocumentContext(conversationId: string, metadata: Record<string, string>): Promise<void> {
    if ($is_tauri) {
      const result = await invoke<ContextMutationResponse>('add_context_to_conversation', {
        conversationId,
        contextType: 'document',
        title: item.title,
        content: item.body,
        metadata
      });

      if (result.status !== 'success') {
        throw new Error(result.error ?? 'Unknown error adding document context');
      }

      return;
    }

    const baseUrl = CONFIG.ServerURL;
    const url = `${baseUrl}/conversations/${conversationId}/context`;
    const response = await fetch(url, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        context_type: 'document',
        title: item.title,
        content: item.body,
        metadata
      })
    });

    if (!response.ok) {
      throw new Error(`Failed to add context: ${response.status} ${response.statusText}`);
    }

    const data: ContextMutationResponse = await response.json();

    if (data.status !== 'success') {
      throw new Error(data.error ?? 'Unknown error adding document context');
    }
  }

  // Check if current role has atomic server configuration
  $: hasAtomicServer = checkAtomicServerAvailable();

  // Data-driven menu configuration
  $: menuItems = generateMenuItems();

  function generateMenuItems() {
    const items = [];

    // Always show download to markdown - downloads file only
    items.push({
      id: 'download-markdown',
      label: 'Download to Markdown',
      icon: 'fas fa-download',
      action: () => downloadToMarkdown(),
      visible: true,
      title: 'Download document as markdown file'
    });

    // Show atomic save only if configured
    if (hasAtomicServer) {
      items.push({
        id: 'save-atomic',
        label: 'Save to Atomic Server',
        icon: 'fas fa-cloud-upload-alt',
        action: () => onAtomicSaveClick(),
        visible: true,
        title: 'Save article to Atomic Server',
        className: 'has-text-primary'
      });
    }

    // Show external URL if available - opens URL in new tab and article modal
    if (item.url) {
      items.push({
        id: 'external-url',
        label: 'Open URL',
        icon: 'fas fa-link',
        action: () => openUrlAndModal(),
        visible: true,
        title: 'Open original URL in new tab'
      });
    }

    // VSCode integration removed as requested

    // Add to context for LLM conversation
    items.push({
      id: 'add-context',
      label: contextAdded ? 'Added to Context ✓' : (addingToContext ? 'Adding...' : 'Add to Context'),
      icon: contextAdded ? 'fas fa-check-circle' : (addingToContext ? 'fas fa-spinner fa-spin' : 'fas fa-plus-circle'),
      action: () => addToContext(),
      visible: true,
      title: contextAdded ? 'Document successfully added to chat context. Go to Chat tab to see it.' : 'Add document to LLM conversation context',
      disabled: addingToContext || contextAdded,
      className: contextAdded ? 'has-text-success' : (contextError ? 'has-text-danger' : ''),
      testId: 'add-to-context-button'
    });

    // Chat with document (add to context + navigate to chat)
    items.push({
      id: 'chat-with-document',
      label: chatStarted ? 'Opening Chat...' : (chattingWithDocument ? 'Adding to Chat...' : 'Chat with Document'),
      icon: chatStarted ? 'fas fa-external-link-alt' : (chattingWithDocument ? 'fas fa-spinner fa-spin' : 'fas fa-comment-dots'),
      action: () => addToContextAndChat(),
      visible: true,
      title: chatStarted ? 'Opening chat with this document' : 'Add document to context and open chat',
      disabled: chattingWithDocument || chatStarted || addingToContext,
      className: chatStarted ? 'has-text-info' : (contextError ? 'has-text-danger' : 'has-text-primary'),
      testId: 'chat-with-document-button'
    });

    return items;
  }

  function checkAtomicServerAvailable(): boolean {
    const currentRoleName = $role;
    const config = $roleConfigStore;

    if (!config?.roles || !currentRoleName) {
      return false;
    }

    // Find the current role object - handle the complex role structure
    let currentRole: Role | null = null;

    try {
      // Handle both string keys and RoleName objects in the roles map
      for (const [roleName, roleConfig] of Object.entries(config.roles)) {
        // Cast roleConfig to Role type for proper access
        const role = roleConfig as Role;

        // Check various ways the role name might match
        const roleNameStr = typeof role.name === 'object'
          ? role.name.original
          : String(role.name);

        if (roleName === currentRoleName || roleNameStr === currentRoleName) {
          currentRole = role;
          break;
        }
      }
    } catch (error) {
      console.warn('Error checking role configuration:', error);
      return false;
    }

    if (!currentRole) {
      return false;
    }

    // Check if role has any writable atomic server haystacks
    const atomicHaystacks = currentRole.haystacks?.filter(haystack =>
      haystack.service === "Atomic" &&
      haystack.location &&
      !haystack.read_only
    ) || [];

    return atomicHaystacks.length > 0;
  }

  const onTitleClick = () => {
    showModal = true;
  };

  const onAtomicSaveClick = () => {
    console.log('🔄 Opening atomic save modal for document:', item.title);
    showAtomicSaveModal = true;
  };

  async function handleTagClick(tag: string) {
    loadingKg = true;
    kgTerm = tag;

    console.log('🔍 KG Search Debug Info:', { tag, role: $role, isTauri: $is_tauri });

    try {
      if ($is_tauri) {
        const response = await invoke<DocumentListResponse>('find_documents_for_kg_term', {
          roleName: $role,
          term: tag
        });

        console.log('  📥 Tauri response received:', response);

        const doc = response.results?.[0];

        if (response.status === 'success' && doc) {
          kgDocument = doc;
          kgRank = doc.rank ?? 0;
          console.log('  ✅ Found KG document:', {
            title: doc.title,
            rank: kgRank,
            bodyLength: doc.body?.length ?? 0
          });
          showKgModal = true;
        } else {
          console.warn(`  ⚠️  No KG documents found for term: "${tag}" in role: "${$role}"`);
        }
      } else {
        const baseUrl = CONFIG.ServerURL;
        const roleName = encodeURIComponent(resolveRoleName());
        const url = `${baseUrl}/roles/${roleName}/kg_search?term=${encodeURIComponent(tag)}`;

        console.log('  📤 HTTP Request details:', { url });

        const response = await fetch(url);

        if (!response.ok) {
          throw new Error(`HTTP error! Status: ${response.status} - ${response.statusText}`);
        }

        const data: DocumentListResponse = await response.json();
        console.log('  📄 Response data:', data);

        const doc = data.results?.[0];

        if (data.status === 'success' && doc) {
          kgDocument = doc;
          kgRank = doc.rank ?? 0;
          console.log('  ✅ Found KG document:', {
            title: doc.title,
            rank: kgRank,
            bodyLength: doc.body?.length ?? 0
          });
          showKgModal = true;
        } else {
          console.warn(`  ⚠️  No KG documents found for term: "${tag}" in role: "${$role}"`);
        }
      }
    } catch (error) {
      const err = toError(error);
      console.error('❌ Error fetching KG document:', err);
      console.error('  Request details:', {
        tag,
        role: $role,
        isTauri: $is_tauri,
        timestamp: new Date().toISOString()
      });

      if (!$is_tauri && err.message.includes('Failed to fetch')) {
        console.error('  💡 Network error suggestions:', [
          'Check if server is running on expected port',
          'Check CORS configuration',
          'Verify server URL in CONFIG.ServerURL'
        ]);
      }
    } finally {
      loadingKg = false;
    }
  }

  async function generateSummary() {
    if (summaryLoading || !item.id || !$role) return;

    summaryLoading = true;
    summaryError = null;

    const roleName = resolveRoleName();

    console.log('🤖 AI Summary Debug Info:', {
      documentId: item.id,
      role: roleName,
      isTauri: $is_tauri
    });

    try {
      const requestBody = {
        document_id: item.id,
        role: roleName,
        max_length: 250,
        force_regenerate: false
      };

      console.log('  📤 Summarization request:', requestBody);

      const baseUrl = CONFIG.ServerURL;
      const url = `${baseUrl}/documents/summarize`;

      const response = await fetch(url, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json'
        },
        body: JSON.stringify(requestBody)
      });

      console.log('  📥 Summary response received:');
      console.log('    Status code:', response.status);
      console.log('    Status text:', response.statusText);

      if (!response.ok) {
        throw new Error(`HTTP error! Status: ${response.status} - ${response.statusText}`);
      }

      const data = await response.json();
      console.log('  📄 Summary response data:', data);

      if (data.status === 'success' && typeof data.summary === 'string') {
        const summary: string = data.summary;
        aiSummary = summary;
        summaryFromCache = Boolean(data.from_cache);
        showAiSummary = true;
        console.log('  ✅ Summary generated successfully');
        console.log('    Summary length:', summary.length, 'characters');
        console.log('    From cache:', summaryFromCache);
        console.log('    Model used:', data.model_used);
      } else {
        summaryError = data.error || 'Failed to generate summary';
        console.error('  ❌ Summary generation failed:', summaryError);
      }
    } catch (error) {
      const err = toError(error);
      console.error('❌ Error generating summary:', err);
      console.error('  Request details:', {
        document_id: item.id,
        role: roleName,
        isTauri: $is_tauri,
        timestamp: new Date().toISOString()
      });

      summaryError = err.message || 'Network error occurred';

      if (err.message.includes('Failed to fetch')) {
        console.error('  💡 Network error suggestions:', [
          'Check if server is running on expected port',
          'Verify OpenRouter is enabled for this role',
          'Check OPENROUTER_KEY environment variable',
          'Verify server URL in CONFIG.ServerURL'
        ]);
      }
    } finally {
      summaryLoading = false;
    }
  }

  async function downloadToMarkdown() {
    console.log('📥 Downloading document as markdown:', item.title);
    console.log('📄 Document data:', { title: item.title, bodyLength: item.body?.length, tags: item.tags });
    console.log('🖥️ Environment check - is_tauri:', $is_tauri);

    // Create markdown content
    let markdownContent = `# ${item.title}\n\n`;

    // Add metadata
    markdownContent += `**Source:** Terraphim Search\n`;
    markdownContent += `**Rank:** ${item.rank || 'N/A'}\n`;
    if (item.url) {
      markdownContent += `**URL:** ${item.url}\n`;
    }
    if (item.tags && item.tags.length > 0) {
      markdownContent += `**Tags:** ${item.tags.join(', ')}\n`;
    }
    markdownContent += `**Downloaded:** ${new Date().toISOString()}\n\n`;

    // Add description if available
    if (item.description) {
      markdownContent += `## Description\n\n${item.description}\n\n`;
    }

    // Add main content
    markdownContent += `## Content\n\n${item.body}\n`;

    // Create filename
    const filename = `${item.title.replace(/[^a-z0-9]/gi, '_').toLowerCase()}_${Date.now()}.md`;

    // Use the proven saveAsMarkdown implementation from Chat component
    try {
      if ($is_tauri) {
        // Import Tauri APIs dynamically
        const { save } = await import('@tauri-apps/api/dialog');
        const { writeTextFile } = await import('@tauri-apps/api/fs');

        console.log('💾 Using Tauri save dialog...');
        const savePath = await save({
          filters: [{ name: 'Markdown', extensions: ['md'] }],
          defaultPath: filename
        });

        if (savePath) {
          await writeTextFile(savePath as string, markdownContent);
          console.log('✅ File saved via Tauri:', savePath);
        } else {
          console.log('❌ Save dialog cancelled');
        }
      } else {
        // Browser fallback: trigger download
        console.log('🌐 Using browser download fallback...');
        const blob = new Blob([markdownContent], { type: 'text/markdown;charset=utf-8' });
        const url = URL.createObjectURL(blob);
        const a = document.createElement('a');
        a.href = url;
        a.download = filename;
        document.body.appendChild(a);
        a.click();
        document.body.removeChild(a);
        URL.revokeObjectURL(url);
        console.log('✅ File downloaded via browser:', filename);
      }
    } catch (error) {
      console.error('❌ Download failed:', toError(error));

      // Fallback to browser download even in Tauri if the above fails
      console.log('🔄 Falling back to browser download...');
      const blob = new Blob([markdownContent], { type: 'text/markdown;charset=utf-8' });
      const url = URL.createObjectURL(blob);
      const anchor = document.createElement('a');
      anchor.href = url;
      anchor.download = filename;
      document.body.appendChild(anchor);
      anchor.click();
      document.body.removeChild(anchor);
      URL.revokeObjectURL(url);
      console.log('✅ Fallback download completed:', filename);
    }
  }

  function downloadToMarkdownAndOpenModal() {
    console.log('🔄 Starting download and modal process...');
    // First download the markdown file
    downloadToMarkdown();
    // Add a small delay before opening modal to ensure download starts
    setTimeout(() => {
      console.log('📖 Opening article modal...');
      onTitleClick();
    }, 100);
  }

  function openUrlAndModal() {
    console.log('🔄 Opening URL in new tab and article modal...');
    // First open the URL in a new tab
    if (item.url) {
      console.log('🔗 Opening URL in new tab:', item.url);
      window.open(item.url, '_blank');
    }
    // Add a small delay before opening modal
    setTimeout(() => {
      console.log('📖 Opening article modal...');
      onTitleClick();
    }, 100);
  }

  function openInVSCode() {
    const vscodeUrl = `vscode://${encodeURIComponent(item.title)}.md?${encodeURIComponent(item.body)}`;
    window.open(vscodeUrl, '_blank');
  }

  async function addToContext() {
    if (addingToContext) {
      return;
    }

    console.log('📝 Adding document to LLM context:', item.title);

    addingToContext = true;
    contextAdded = false;
    contextError = null;

    try {
      const conversationIdValue = await getOrCreateConversationId('Search Context');
      const metadata = buildContextMetadata();

      await addDocumentContext(conversationIdValue, metadata);

      console.log('✅ Successfully added document to LLM context');
      contextAdded = true;

      const notification = window.document.createElement('div');
      notification.className = 'notification is-success is-light';
      notification.innerHTML = `
        <button class="delete" aria-label="Dismiss notification" onclick="this.parentElement.remove()"></button>
        <strong>✓ Added to Chat Context</strong><br>
        <small>Document added successfully. <a href="/chat" class="has-text-success has-text-weight-bold">Go to Chat →</a> to see it in the context panel.</small>
      `;
      notification.style.cssText = 'position: fixed; top: 20px; right: 20px; z-index: 1000; max-width: 350px;';
      window.document.body.appendChild(notification);

      setTimeout(() => notification.remove(), 8000);
      setTimeout(() => {
        if (!chattingWithDocument) {
          contextAdded = false;
        }
      }, 5000);
    } catch (error) {
      const err = toError(error);
      console.error('❌ Error adding document to context:', err);

      contextError = err.message || 'Failed to add document to context';

      const notification = window.document.createElement('div');
      notification.className = 'notification is-danger is-light';
      notification.innerHTML = `
        <button class="delete" aria-label="Dismiss notification" onclick="this.parentElement.remove()"></button>
        <strong>✗ Failed to Add Context</strong><br>
        <small>${contextError}</small>
      `;
      notification.style.cssText = 'position: fixed; top: 20px; right: 20px; z-index: 1000; max-width: 350px;';
      window.document.body.appendChild(notification);

      setTimeout(() => notification.remove(), 8000);
      setTimeout(() => {
        contextError = null;
      }, 5000);
    } finally {
      addingToContext = false;
    }
  }

  async function addToContextAndChat() {
    if (chattingWithDocument) {
      return;
    }

    console.log('💬 Adding document to context and opening chat:', item.title);

    chattingWithDocument = true;
    chatStarted = false;
    contextError = null;

    try {
      const conversationIdValue = await getOrCreateConversationId('Chat with Documents');
      const metadata = buildContextMetadata();

      await addDocumentContext(conversationIdValue, metadata);

      console.log('✅ Successfully added document to context');

      contextAdded = true;
      chatStarted = true;

      const notification = window.document.createElement('div');
      notification.className = 'notification is-success is-light';
      notification.innerHTML = `
        <button class="delete" aria-label="Dismiss notification" onclick="this.parentElement.remove()"></button>
        <strong>✓ Document Ready in Chat</strong><br>
        <small>Opening chat so you can continue the conversation.</small>
      `;
      notification.style.cssText = 'position: fixed; top: 20px; right: 20px; z-index: 1000; max-width: 350px;';
      window.document.body.appendChild(notification);

      setTimeout(() => {
        notification.remove();
        router.goto('/chat');
      }, 1500);

      setTimeout(() => {
        chatStarted = false;
      }, 2000);

      setTimeout(() => {
        contextAdded = false;
      }, 5000);
    } catch (error) {
      const err = toError(error);
      console.error('❌ Error adding document to context and opening chat:', err);

      contextError = err.message || 'Failed to add document to context';

      const notification = window.document.createElement('div');
      notification.className = 'notification is-danger is-light';
      notification.innerHTML = `
        <button class="delete" aria-label="Dismiss notification" onclick="this.parentElement.remove()"></button>
        <strong>✗ Failed to Open Chat with Document</strong><br>
        <small>${contextError}</small>
      `;
      notification.style.cssText = 'position: fixed; top: 20px; right: 20px; z-index: 1000; max-width: 350px;';
      window.document.body.appendChild(notification);

      setTimeout(() => notification.remove(), 8000);
      setTimeout(() => {
        contextError = null;
      }, 5000);
    } finally {
      chattingWithDocument = false;
    }
  }
</script>

<div class="box">
  <article class="media">
    <div class="media-content">
      <div class="content">
        <div class="level-right">
          {#if item.tags}
          <Taglist>
              {#each item.tags as tag}
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
            <Tag rounded>Rank {item.rank || 0}</Tag>
          </Taglist>
        </div>
        <div transition:fade>
          <button on:click={onTitleClick}>
            <h2 class="title">
              {item.title}
            </h2>
          </button>
          <div class="description">
            <small class="description-label">Description:</small>
            <div class="description-content">
              {#if item.description}
                <SvelteMarkdown source={item.description} />
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
            {#each menuItems as item}
              {#if item.visible}
                {#if item.disabled}
                  <button
                    type="button"
                    class="level-item button is-ghost"
                    aria-label={item.title}
                    title={item.title}
                    disabled={true}
                  >
                    <span class="icon is-medium" class:has-text-primary={item.className}>
                      <i class={item.icon}></i>
                    </span>
                  </button>
                {:else}
                  <button
                    type="button"
                    class="level-item button is-ghost"
                    aria-label={item.title}
                    on:click={item.action}
                    title={item.title}
                    data-testid={item.testId || ''}
                  >
                    <span class="icon is-medium" class:has-text-primary={item.className?.includes('primary')} class:has-text-success={item.className?.includes('success')} class:has-text-danger={item.className?.includes('danger')}>
                      <i class={item.icon}></i>
                    </span>
                  </button>
                {/if}
              {/if}
            {/each}
          </div>
        </nav>
      </div>
    </div>
  </article>

  <!-- Context addition feedback -->
  {#if contextError}
    <div class="notification is-danger is-light mt-2">
      <button
        class="delete"
        aria-label="Dismiss error notification"
        on:click={() => contextError = null}
      ></button>
      {contextError}
    </div>
  {/if}
</div>

<!-- Original document modal -->
<ArticleModal bind:active={showModal} item={item} />

<!-- KG document modal -->
{#if kgDocument}
  <ArticleModal
    bind:active={showKgModal}
    item={kgDocument}
    kgTerm={kgTerm}
    kgRank={kgRank}
  />
{/if}

<!-- Atomic Save Modal -->
{#if hasAtomicServer}
  <AtomicSaveModal
    bind:active={showAtomicSaveModal}
    document={item}
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

  /* Button spacing for result item menu */
  .level.is-mobile .level-right {
    gap: 0.5rem;
  }

  .level-item.button {
    margin-left: 0.5rem;
    margin-right: 0;
  }

  .level-item.button:first-child {
    margin-left: 0;
  }

  .ai-summary-actions {
    display: flex;
    justify-content: flex-end;
  }
</style>
