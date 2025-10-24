<!-- A simple component for testing context management without complex UI -->
<script lang="ts">
  import { onMount } from 'svelte';
  import { persistentConversations, currentPersistentConversationId, contexts } from '../../stores';
  import { getConversations, createContext } from '../../services/chatService';

  onMount(async () => {
    await getConversations();
  });

  async function handleAddContext() {
    if ($currentPersistentConversationId) {
      await createContext($currentPersistentConversationId, {
        title: 'New Test Context',
        content: 'Some content',
        context_type: 'UserInput'
      });
    }
  }
</script>

<div data-testid="harness">
  <div data-testid="conversation-count">{$persistentConversations.length}</div>
  <div data-testid="context-count">{$contexts.length}</div>
  <button on:click={handleAddContext}>Add Context</button>
</div>
