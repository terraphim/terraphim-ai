<script lang="ts">
  import { onMount } from 'svelte';
  import IcicleChart from '../Search/IcicleChart.svelte';
  import type { ChartData, NodeResponse, ApiRankedNode, ChartNode } from '../Search/SearchResult';
  import { CONFIG } from "../../config";
  import { is_tauri, role } from '../stores';
  import { invokeTauri } from '../tauri';

  let chartData: ChartData | null = null;
  let error: string | null = null;
  let loading = true;
  let rawResponse: NodeResponse | null = null;

  function generateColor(str: string): string {
    // Generate a deterministic color based on string
    let hash = 0;
    for (let i = 0; i < str.length; i++) {
      hash = str.charCodeAt(i) + ((hash << 5) - hash);
    }
    const c = (hash & 0x00FFFFFF)
      .toString(16)
      .toUpperCase();
    return '#' + '00000'.substring(0, 6 - c.length) + c;
  }

  function prepareNodeData(nodeResponse: NodeResponse): ChartData {
    if (!nodeResponse.nodes || nodeResponse.nodes.length === 0) {
        throw new Error('No nodes data available');
    }

    // Create a clean node structure without circular references
    function createNode(node: ApiRankedNode): ChartNode {
        // Create a clean object with only the required properties
        const cleanNode = {
            name: node.normalized_term,
            size: Math.max(1, node.total_documents), // Ensure minimum size of 1
            color: generateColor(node.normalized_term)
        };

        // Only add children if they exist and have documents
        const validChildren = node.children
            .filter(child => child.total_documents > 0)
            .map(child => createNode(child));

        if (validChildren.length > 0) {
            cleanNode['children'] = validChildren;
        }

        return cleanNode;
    }

    // Create the root structure with only essential properties
    const root: ChartData = {
        name: "Knowledge Graph",
        children: []
    };

    // Process nodes one by one to avoid circular references
    root.children = nodeResponse.nodes
        .filter(node => node.total_documents > 0)
        .map(node => createNode(node));

    // Verify the structure is valid
    try {
        const debugStr = JSON.stringify(root);
        console.log('Chart data structure (length):', debugStr.length);
        console.log('Sample of first node:', root.children[0]);
    } catch (e) {
        console.error('Error stringifying data:', e);
        throw new Error('Invalid data structure: ' + e.message);
    }
    
    return root;
  }

  async function fetchNodes() {
    loading = true;
    error = null;
    chartData = null;
    rawResponse = null;

    try {
      let responseData: NodeResponse;
      
      if ($is_tauri) {
        try {
          responseData = await invokeTauri("get_nodes", {
            role: $role
          }) as NodeResponse;
          console.log('Received Tauri response:', responseData);
        } catch (e) {
          console.error('Tauri invocation error:', e);
          throw new Error(`Tauri error: ${e}`);
        }
      } else {
        console.log('Fetching nodes from API for role:', $role);
        console.log('Using server URL:', `${CONFIG.ServerURL}/nodes`);
        
        try {
          const response = await fetch(`${CONFIG.ServerURL}/nodes`, {
            method: 'POST',
            headers: {
              'Content-Type': 'application/json',
              'Accept': 'application/json'
            },
            body: JSON.stringify({ role: $role })
          });
          
          console.log('Response status:', response.status);
          console.log('Response headers:', Object.fromEntries(response.headers.entries()));
          
          if (!response.ok) {
            const errorText = await response.text();
            console.error('Error response body:', errorText);
            throw new Error(`HTTP error! Status: ${response.status}, Body: ${errorText}`);
          }
          
          responseData = await response.json() as NodeResponse;
          console.log('Received API response:', responseData);
        } catch (e) {
          console.error('Fetch error details:', {
            message: e.message,
            stack: e.stack,
            cause: e.cause
          });
          throw e;
        }
      }

      if (!responseData || !responseData.status || !Array.isArray(responseData.nodes)) {
        console.error('Invalid response format:', responseData);
        throw new Error('Invalid response format');
      }

      rawResponse = responseData;
      
      if (responseData.status !== "success") {
        throw new Error(`API error: ${responseData.status}`);
      }
      
      chartData = prepareNodeData(responseData);
    } catch (e) {
      const errorMessage = `Error fetching nodes: ${e}`;
      error = errorMessage;
      console.error(errorMessage);
      console.error('Full error object:', e);
    } finally {
      loading = false;
    }
  }

  onMount(() => {
    console.log('NodesView mounted, fetching data...');
    fetchNodes();
  });

  $: if ($role) {
    console.log('Role changed to:', $role);
    fetchNodes();
  }
</script>

<div class="nodes-view">
  <h1 class="title">Knowledge Graph Visualization for {$role}</h1>
  
  {#if error}
    <div class="notification is-danger" data-testid="error-message">
      {error}
    </div>
  {:else if loading}
    <div class="notification is-info" data-testid="loading-message">
      Loading nodes...
    </div>
  {:else if chartData}
    <div class="debug-info">
      <details>
        <summary>Debug Information</summary>
        {#if rawResponse}
          <pre>{JSON.stringify({ rawResponse, chartData }, null, 2)}</pre>
        {:else}
          <pre>{JSON.stringify({ chartData }, null, 2)}</pre>
        {/if}
      </details>
    </div>
    <div class="chart-container" data-testid="chart-container">
      <IcicleChart data={chartData} />
    </div>
  {:else}
    <div class="notification is-warning" data-testid="no-data-message">
      No node data available.
    </div>
  {/if}
</div>

<style>
  .nodes-view {
    padding: 2rem;
  }

  .chart-container {
    margin: 2rem 0;
    height: 600px;
    width: 100%;
    border: 1px solid #ddd;
    border-radius: 4px;
    background-color: #fff;
  }

  .notification {
    margin: 1rem 0;
  }

  .title {
    margin-bottom: 2rem;
  }

  .debug-info {
    margin: 1rem 0;
    padding: 1rem;
    background-color: #f5f5f5;
    border-radius: 4px;
  }

  .debug-info pre {
    white-space: pre-wrap;
    word-wrap: break-word;
    max-height: 200px;
    overflow: auto;
  }
</style> 