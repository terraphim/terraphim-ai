<script lang="ts">
  import { onMount } from 'svelte';
  import * as d3 from 'd3';
  import { invoke } from '@tauri-apps/api/tauri';
  import { is_tauri, role } from './stores';
  import ArticleModal from './Search/ArticleModal.svelte';
  import type { Document } from './Search/SearchResult';
  import type { RoleGraphResponse } from './generated/types';

  export let apiUrl: string = '/rolegraph';
  export let fullscreen: boolean = true;

  let container: HTMLDivElement;
  let loading = true;
  let error: string | null = null;
  let nodes: any[] = [];
  let edges: any[] = [];
  let selectedNode: Document | null = null;
  let showModal = false;
  let startInEditMode = false;
  let debugMessage = '';

  // Dimensions
  let width = window.innerWidth;
  let height = window.innerHeight;

  function updateDimensions() {
    width = window.innerWidth;
    height = window.innerHeight;
    if (!loading && !error) {
      renderGraph();
    }
  }

  async function fetchGraph() {
    loading = true;
    error = null;
    try {
      if ($is_tauri) {
        console.log("Loading rolegraph from Tauri");
        const response = await invoke<RoleGraphResponse>('get_rolegraph', { 
          role_name: $role || undefined 
        });
        if (response && response.status === 'success') {
          nodes = response.nodes;
          edges = response.edges;
        } else {
          throw new Error(`Tauri rolegraph fetch failed: ${response?.status || 'unknown error'}`);
        }
      } else {
        console.log("Loading rolegraph from server");
        const url = $role ? `${apiUrl}?role=${encodeURIComponent($role)}` : apiUrl;
        const res = await fetch(url);
        if (!res.ok) throw new Error(`Failed to fetch: ${res.status}`);
        const data = await res.json();
        nodes = data.nodes;
        edges = data.edges;
      }
    } catch (e) {
      error = e.message;
      console.error("Error fetching rolegraph:", e);
    } finally {
      loading = false;
    }
  }

  function nodeToDocument(node: any): Document {
    return {
      id: `kg-node-${node.id}`,
      url: `#/graph/node/${node.id}`,
      title: node.label,
      body: `# ${node.label}\n\n**Knowledge Graph Node**\n\nID: ${node.id}\nRank: ${node.rank}\n\nThis is a concept node from the knowledge graph. You can edit this content to add more information about "${node.label}".`,
      description: `Knowledge graph concept: ${node.label}`,
      tags: ['knowledge-graph', 'concept'],
      rank: node.rank,
      stub: `Knowledge graph concept: ${node.label}`
    };
  }

  function handleNodeClick(event: any, nodeData: any) {
    event.stopPropagation();
    console.log('Node clicked:', nodeData.label);
    selectedNode = nodeToDocument(nodeData);
    startInEditMode = false;
    showModal = true;
  }

  function handleNodeRightClick(event: any, nodeData: any) {
    event.preventDefault();
    event.stopPropagation();
    console.log('Node right-clicked:', nodeData.label);
    debugMessage = `Right-clicked: ${nodeData.label}`;
    selectedNode = nodeToDocument(nodeData);
    startInEditMode = true;
    showModal = true;
    
    // Clear debug message after 2 seconds
    setTimeout(() => {
      debugMessage = '';
    }, 2000);
  }

  function handleModalClose() {
    showModal = false;
    selectedNode = null;
    startInEditMode = false;
  }

  async function handleModalSave() {
    if (!selectedNode) return;
    
    try {
      const response = await fetch('/documents', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify(selectedNode),
      });
      
      if (response.ok) {
        console.log('Successfully saved KG record:', selectedNode.id);
      } else {
        console.error('Failed to save KG record:', response.statusText);
      }
    } catch (error) {
      console.error('Error saving KG record:', error);
    } finally {
      showModal = false;
      selectedNode = null;
      startInEditMode = false;
    }
  }

  function renderGraph() {
    if (!container) return;
    container.innerHTML = '';
    
    const svg = d3.select(container)
      .append('svg')
      .attr('width', width)
      .attr('height', height);

    // Zoom and pan behavior
    const zoom = d3.zoom()
      .scaleExtent([0.1, 10])
      .on('zoom', (event) => {
        g.attr('transform', event.transform);
      });

    svg.call(zoom);

    const g = svg.append('g');

    // Force simulation
    const simulation = d3.forceSimulation(nodes)
      .force('link', d3.forceLink(edges).id((d: any) => d.id).distance(100))
      .force('charge', d3.forceManyBody().strength(-200))
      .force('center', d3.forceCenter(width / 2, height / 2))
      .force('collision', d3.forceCollide().radius(20));

    // Render edges with thickness based on weight
    const link = g.append('g')
      .attr('class', 'links')
      .selectAll('line')
      .data(edges)
      .enter().append('line')
      .attr('stroke', '#666')
      .attr('stroke-opacity', 0.7)
      .attr('stroke-width', (d: any) => {
        // Edge thickness based on weight/rank
        const weight = d.weight || d.rank || 1;
        return Math.max(1, Math.min(8, weight * 2));
      });

    // Render nodes
    const node = g.append('g')
      .attr('class', 'nodes')
      .selectAll('circle')
      .data(nodes)
      .enter().append('circle')
      .attr('r', (d: any) => {
        // Node size based on rank
        const rank = d.rank || 1;
        return Math.max(6, Math.min(20, rank * 2));
      })
      .attr('fill', (d: any) => {
        // Node color based on rank
        const rank = d.rank || 1;
        const intensity = Math.min(rank / 10, 1);
        return d3.interpolateBlues(0.2 + intensity * 0.6);
      })
      .attr('stroke', '#fff')
      .attr('stroke-width', 2)
      .style('cursor', 'pointer')
      .on('click', (event, d) => handleNodeClick(event, d))
      .on('contextmenu', (event, d) => handleNodeRightClick(event, d))
      .on('mouseover', function(event, d) {
        d3.select(this)
          .transition()
          .duration(150)
          .attr('stroke-width', 3)
          .attr('r', (d: any) => {
            const rank = d.rank || 1;
            return Math.max(8, Math.min(24, rank * 2.5));
          });
      })
      .on('mouseout', function(event, d) {
        d3.select(this)
          .transition()
          .duration(150)
          .attr('stroke-width', 2)
          .attr('r', (d: any) => {
            const rank = d.rank || 1;
            return Math.max(6, Math.min(20, rank * 2));
          });
      })
      .call(d3.drag()
        .on('start', dragstarted)
        .on('drag', dragged)
        .on('end', dragended));

    // Node labels
    const label = g.append('g')
      .attr('class', 'labels')
      .selectAll('text')
      .data(nodes)
      .enter().append('text')
      .attr('text-anchor', 'middle')
      .attr('dy', '.35em')
      .attr('font-size', '11px')
      .attr('font-family', 'Arial, sans-serif')
      .attr('fill', '#333')
      .attr('pointer-events', 'none')
      .text((d: any) => {
        // Show full label on hover, truncated otherwise
        return d.label.length > 12 ? d.label.substring(0, 12) + '...' : d.label;
      });

    // Update positions on simulation tick
    simulation.on('tick', () => {
      link
        .attr('x1', (d: any) => d.source.x)
        .attr('y1', (d: any) => d.source.y)
        .attr('x2', (d: any) => d.target.x)
        .attr('y2', (d: any) => d.target.y);
      
      node
        .attr('cx', (d: any) => d.x)
        .attr('cy', (d: any) => d.y);
      
      label
        .attr('x', (d: any) => d.x)
        .attr('y', (d: any) => d.y);
    });

    function dragstarted(event: any, d: any) {
      if (!event.active) simulation.alphaTarget(0.3).restart();
      d.fx = d.x;
      d.fy = d.y;
    }
    
    function dragged(event: any, d: any) {
      d.fx = event.x;
      d.fy = event.y;
    }
    
    function dragended(event: any, d: any) {
      if (!event.active) simulation.alphaTarget(0);
      d.fx = null;
      d.fy = null;
    }
  }

  onMount(() => {
    fetchGraph().then(() => {
      if (!error) renderGraph();
    });
    
    // Prevent browser context menu on the graph container
    const preventContextMenu = (e: Event) => {
      e.preventDefault();
    };
    
    if (container) {
      container.addEventListener('contextmenu', preventContextMenu);
    }
    
    window.addEventListener('resize', updateDimensions);
    
    return () => {
      if (container) {
        container.removeEventListener('contextmenu', preventContextMenu);
      }
      window.removeEventListener('resize', updateDimensions);
    };
  });
</script>

<svelte:window bind:innerWidth={width} bind:innerHeight={height} />

<!-- Graph container -->
<div 
  bind:this={container} 
  class="graph-container"
  class:fullscreen
  style="width: {fullscreen ? '100vw' : '600px'}; height: {fullscreen ? '100vh' : '400px'};"
>
  {#if loading}
    <div class="loading-overlay">
      <div class="loading-content">
        <div class="loader"></div>
        <p>Loading knowledge graph...</p>
      </div>
    </div>
  {:else if error}
    <div class="error-overlay">
      <div class="error-content">
        <h3>Error loading graph</h3>
        <p>{error}</p>
        <button class="button is-primary" on:click={fetchGraph}>Retry</button>
      </div>
    </div>
  {/if}
</div>

<!-- Close button for fullscreen mode -->
{#if fullscreen}
  <button class="close-button" on:click={() => history.back()} title="Close Graph">
    <i class="fas fa-times"></i>
  </button>
{/if}

<!-- Simple controls info -->
{#if !loading && !error && nodes.length > 0}
  <div class="controls-info">
    <span><strong>Left-click:</strong> View node • <strong>Right-click:</strong> Edit node • <strong>Drag:</strong> Move • <strong>Scroll:</strong> Zoom</span>
  </div>
{/if}

<!-- Debug message -->
{#if debugMessage}
  <div class="debug-message">
    {debugMessage}
  </div>
{/if}

<!-- Modal for viewing/editing KG records -->
{#if selectedNode}
  {#key selectedNode.id}
    <ArticleModal 
      bind:active={showModal} 
      item={selectedNode}
      initialEdit={startInEditMode}
      on:close={handleModalClose}
      on:save={handleModalSave}
    />
  {/key}
{/if}

<style>
  .graph-container {
    position: relative;
    background: linear-gradient(135deg, #f5f7fa 0%, #c3cfe2 100%);
    border-radius: 8px;
    overflow: hidden;
    box-shadow: 0 4px 20px rgba(0, 0, 0, 0.1);
    z-index: 100;
  }

  .graph-container.fullscreen {
    position: fixed;
    top: 0;
    left: 0;
    z-index: 100;
    border-radius: 0;
    background: linear-gradient(135deg, #f5f7fa 0%, #c3cfe2 100%);
  }

  .loading-overlay, .error-overlay {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    background: rgba(255, 255, 255, 0.9);
    backdrop-filter: blur(5px);
  }

  .loading-content, .error-content {
    text-align: center;
    padding: 2rem;
  }

  .loader {
    border: 4px solid #f3f3f3;
    border-top: 4px solid #3498db;
    border-radius: 50%;
    width: 50px;
    height: 50px;
    animation: spin 2s linear infinite;
    margin: 0 auto 1rem;
  }

  @keyframes spin {
    0% { transform: rotate(0deg); }
    100% { transform: rotate(360deg); }
  }

  .close-button {
    position: fixed;
    top: 20px;
    right: 20px;
    z-index: 150;
    background: rgba(255, 255, 255, 0.9);
    border: none;
    border-radius: 50%;
    width: 50px;
    height: 50px;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    box-shadow: 0 2px 10px rgba(0, 0, 0, 0.2);
    transition: all 0.3s ease;
  }

  .close-button:hover {
    background: rgba(255, 255, 255, 1);
    transform: scale(1.1);
  }

  .close-button i {
    font-size: 1.2rem;
    color: #333;
  }

  .controls-info {
    position: fixed;
    bottom: 20px;
    left: 50%;
    transform: translateX(-50%);
    z-index: 150;
    background: rgba(255, 255, 255, 0.9);
    backdrop-filter: blur(10px);
    padding: 8px 16px;
    border-radius: 20px;
    box-shadow: 0 2px 15px rgba(0, 0, 0, 0.1);
    font-size: 0.85rem;
    color: #666;
  }

  .debug-message {
    position: fixed;
    top: 20px;
    left: 50%;
    transform: translateX(-50%);
    z-index: 150;
    background: rgba(52, 152, 219, 0.9);
    color: white;
    padding: 10px 20px;
    border-radius: 20px;
    box-shadow: 0 2px 15px rgba(0, 0, 0, 0.2);
    font-size: 0.9rem;
    font-weight: 500;
  }

  /* Global styles for graph elements */
  :global(.graph-container svg) {
    background: transparent;
  }

  :global(.graph-container .links line) {
    transition: stroke-width 0.2s ease;
  }

  :global(.graph-container .nodes circle) {
    transition: all 0.2s ease;
    filter: drop-shadow(0 1px 3px rgba(0, 0, 0, 0.2));
  }

  :global(.graph-container .labels text) {
    text-shadow: 1px 1px 2px rgba(255, 255, 255, 0.9);
    font-weight: 500;
  }

  /* Ensure Bulma/Svelma modal sits above the graph */
  :global(.modal) {
    z-index: 2000 !important;
  }

  /* Let background stay behind card */
  :global(.modal-background) {
    z-index: 100 !important;
  }

  :global(.modal-card),
  :global(.modal-content) {
    z-index: 2010 !important;
  }
</style> 