<script lang="ts">
  import { Editor as NovelEditor } from '@paralect/novel-svelte';
  import { Markdown } from 'tiptap-markdown';
  import { onMount } from 'svelte';
  import { novelAutocompleteService } from '../services/novelAutocompleteService';
  
  export let html: any = '';          // initial content in HTML/JSON
  export let readOnly: boolean = false;
  export let outputFormat: 'html' | 'markdown' = 'html';  // New prop to control output format
  export let enableAutocomplete: boolean = true; // New prop to enable/disable autocomplete
  export let showSnippets: boolean = true; // New prop to show snippets in autocomplete

  let editor: any = null;
  let autocompleteStatus = '⏳ Initializing...';
  let autocompleteReady = false;
  let mockSuggestions: string[] = [];

  onMount(async () => {
    if (enableAutocomplete) {
      try {
        // Initialize the autocomplete service
        const success = await novelAutocompleteService.buildAutocompleteIndex();
        if (success) {
          autocompleteStatus = '✅ Ready - Type to trigger autocomplete';
          autocompleteReady = true;
        } else {
          autocompleteStatus = '⚠️ Using mock autocomplete (MCP server not responding)';
          autocompleteReady = true;
          // Load mock suggestions for demonstration
          mockSuggestions = [
            'terraphim-graph',
            'terraphim-automata', 
            'terraphim-service',
            'terraphim-types',
            'terraphim-config',
            'knowledge-graph',
            'role-based-search',
            'haystack-integration',
            'atomic-server',
            'mcp-protocol'
          ];
        }
      } catch (error) {
        console.error('Error initializing autocomplete:', error);
        autocompleteStatus = '⚠️ Using mock autocomplete (MCP server error)';
        autocompleteReady = true;
        // Load mock suggestions for demonstration
        mockSuggestions = [
          'terraphim-graph',
          'terraphim-automata', 
          'terraphim-service',
          'terraphim-types',
          'terraphim-config',
          'knowledge-graph',
          'role-based-search',
          'haystack-integration',
          'atomic-server',
          'mcp-protocol'
        ];
      }
    }
  });

  /** Handler called by Novel editor on every update; we translate it to the
   *  wrapper's `html` variable so the parent can bind to it. */
  const handleUpdate = (editorInstance: any) => {
    editor = editorInstance;
    
    // Choose output format based on the outputFormat prop
    // For markdown content, use getMarkdown() to preserve markdown syntax
    // For HTML content, use getHTML() to preserve rich formatting
    if (outputFormat === 'markdown') {
      html = editorInstance.storage.markdown.getMarkdown();
    } else {
      html = editorInstance.getHTML();
    }
  };

  // Function to manually test autocomplete
  const testAutocomplete = async () => {
    if (autocompleteReady) {
      try {
        if (mockSuggestions.length > 0) {
          // Use mock suggestions for demonstration
          console.log('Mock autocomplete suggestions:', mockSuggestions);
          alert(`Found ${mockSuggestions.length} mock suggestions:\n${mockSuggestions.slice(0, 5).join('\n')}`);
        } else {
          const suggestions = await novelAutocompleteService.getSuggestions('terraphim', 5);
          console.log('Autocomplete test results:', suggestions);
          alert(`Found ${suggestions.length} suggestions for 'terraphim'`);
        }
      } catch (error) {
        console.error('Autocomplete test failed:', error);
        alert('Autocomplete test failed - check console for details');
      }
    } else {
      alert('Autocomplete service not ready yet');
    }
  };

  // Function to rebuild autocomplete index
  const rebuildIndex = async () => {
    autocompleteStatus = '⏳ Rebuilding index...';
    try {
      const success = await novelAutocompleteService.buildAutocompleteIndex();
      if (success) {
        autocompleteStatus = '✅ Ready - Index rebuilt successfully';
        autocompleteReady = true;
      } else {
        autocompleteStatus = '⚠️ Using mock autocomplete (MCP server not responding)';
        autocompleteReady = true;
      }
    } catch (error) {
      console.error('Error rebuilding index:', error);
      autocompleteStatus = '⚠️ Using mock autocomplete (MCP server error)';
      autocompleteReady = true;
    }
  };

  // Function to demonstrate autocomplete in action
  const demonstrateAutocomplete = () => {
    if (editor && mockSuggestions.length > 0) {
      // Insert some text to demonstrate autocomplete
      const demoText = `# Terraphim Autocomplete Demo

This is a demonstration of how autocomplete would work in the Novel editor.

Try typing these terms to see autocomplete suggestions:
- terraphim
- graph
- service
- automata

The autocomplete system provides suggestions based on your knowledge graph and document content.`;
      
      editor.commands.setContent(demoText);
      alert('Demo content inserted! Type "terraphim" or "graph" to see autocomplete suggestions.');
    }
  };
</script>

<NovelEditor
  defaultValue={html}
  isEditable={!readOnly}
  disableLocalStorage={true}
  onUpdate={handleUpdate}
  extensions={[Markdown]}
/>

<!-- Autocomplete Status and Controls -->
{#if enableAutocomplete}
  <div style="margin-top: 10px; padding: 12px; background: #f8f9fa; border-radius: 6px; border: 1px solid #e9ecef;">
    <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 8px;">
      <strong style="color: #495057;">Local Autocomplete Status:</strong>
      <div style="display: flex; gap: 8px;">
        <button 
          on:click={testAutocomplete}
          style="
            padding: 4px 8px; 
            background: #007bff; 
            color: white; 
            border: none; 
            border-radius: 4px; 
            cursor: pointer;
            font-size: 12px;
          "
          disabled={!autocompleteReady}
        >
          Test
        </button>
        <button 
          on:click={rebuildIndex}
          style="
            padding: 4px 8px; 
            background: #28a745; 
            color: white; 
            border: none; 
            border-radius: 4px; 
            cursor: pointer;
            font-size: 12px;
          "
        >
          Rebuild Index
        </button>
        <button 
          on:click={demonstrateAutocomplete}
          style="
            padding: 4px 8px; 
            background: #ffc107; 
            color: #212529; 
            border: none; 
            border-radius: 4px; 
            cursor: pointer;
            font-size: 12px;
          "
          disabled={!autocompleteReady}
        >
          Demo
        </button>
      </div>
    </div>
    
    <div style="font-size: 13px; color: #6c757d; margin-bottom: 8px;">
      {autocompleteStatus}
    </div>
    
    <div style="font-size: 12px; color: #6c757d;">
      <strong>Features:</strong>
      {#if showSnippets}
        <br>• Local autocomplete with snippets from MCP server
      {:else}
        <br>• Local autocomplete from MCP server
      {/if}
      <br>• Type at least 2 characters to trigger
      <br>• Uses role-based knowledge graph for suggestions
      {#if mockSuggestions.length > 0}
        <br>• <strong>Demo Mode:</strong> Using mock suggestions for demonstration
      {/if}
    </div>

    {#if mockSuggestions.length > 0}
      <div style="margin-top: 8px; padding: 8px; background: #fff3cd; border: 1px solid #ffeaa7; border-radius: 4px;">
        <strong>🎯 Mock Autocomplete Suggestions:</strong>
        <div style="font-size: 11px; margin-top: 4px;">
          {mockSuggestions.slice(0, 6).join(' • ')}
          {#if mockSuggestions.length > 6}
            <br>... and {mockSuggestions.length - 6} more
          {/if}
        </div>
      </div>
    {/if}
  </div>
{/if}
