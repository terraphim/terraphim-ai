<script lang="ts">
  import { onMount } from 'svelte';
  import * as d3 from 'd3';
  import ArticleModal from './Search/ArticleModal.svelte';
  import type { Document } from './Search/SearchResult';

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
  let clickTimer: number | null = null;

  // Full-screen dimensions
  let width = window.innerWidth;
  let height = window.innerHeight;

  // Update dimensions on resize
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
      const res = await fetch(apiUrl);
      if (!res.ok) throw new Error(`Failed to fetch: ${res.status}`);
      const data = await res.json();
      nodes = data.nodes;
      edges = data.edges;
    } catch (e) {
      error = e.message;
    } finally {
      loading = false;
    }
  }

  // Convert graph node to Document for ModalArticle
  function nodeToDocument(node: any): Document {
    return {
      id: `kg-node-${node.id}`,
      url: `#/graph/node/${node.id}`,
      title: node.label,
      body: `# ${node.label}\n\n**Knowledge Graph Node**\n\nID: ${node.id}\nRank: ${node.rank}\n\nThis is a concept node from the knowledge graph. You can edit this content to add more information about "${node.label}".`,
      description: `Knowledge graph concept: ${node.label}`,
      tags: ['knowledge-graph', 'concept'],
      rank: node.rank
    };
  }

  function handleNodeClick(nodeData: any) {
    if (clickTimer) {
      // Double-click detected
      clearTimeout(clickTimer);
      clickTimer = null;
      handleNodeDoubleClick(nodeData);
    } else {
      // Single-click detected
      clickTimer = window.setTimeout(() => {
        clickTimer = null;
        selectedNode = nodeToDocument(nodeData);
        startInEditMode = false;
        showModal = true;
      }, 250); // ms
    }
  }

  function handleNodeDoubleClick(nodeData: any) {
    selectedNode = nodeToDocument(nodeData);
    startInEditMode = true;
    showModal = true;
  }

  function handleModalClose() {
    showModal = false;
    selectedNode = null;
    startInEditMode = false;
  }

  async function handleModalSave() {
    if (!selectedNode) return;
    
    try {
      // Save the updated KG record using the document API
      const response = await fetch('/documents', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify(selectedNode),
      });
      
      if (response.ok) {
        console.log('Successfully saved KG record:', selectedNode.id);
        // Optionally refresh the graph to reflect changes
        // await fetchGraph();
        // if (!error) renderGraph();
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

    // Add zoom and pan behavior
    const zoom = d3.zoom()
      .scaleExtent([0.1, 10])
      .on('zoom', (event) => {
        g.attr('transform', event.transform);
      });

    // Apply zoom to svg but filter out double-click events on nodes
    svg.call(zoom)
      .on('dblclick.zoom', null); // Disable double-click zoom

    // Create main group for zooming/panning
    const g = svg.append('g');

    const simulation = d3.forceSimulation(nodes)
      .force('link', d3.forceLink(edges).id((d: any) => d.id).distance(120))
      .force('charge', d3.forceManyBody().strength(-300))
      .force('center', d3.forceCenter(width / 2, height / 2))
      .force('collision', d3.forceCollide().radius(25));

    const link = g.append('g')
      .attr('class', 'links')
      .selectAll('line')
      .data(edges)
      .enter().append('line')
      .attr('stroke', '#999')
      .attr('stroke-opacity', 0.6)
      .attr('stroke-width', (d: any) => Math.sqrt(d.rank || 1));

    const node = g.append('g')
      .attr('class', 'nodes')
      .selectAll('circle')
      .data(nodes)
      .enter().append('circle')
      .attr('r', (d: any) => Math.max(8, Math.sqrt(d.rank || 1) * 3))
      .attr('fill', (d: any) => {
        // Color nodes based on rank
        const intensity = Math.min(d.rank / 10, 1);
        return d3.interpolateBlues(0.3 + intensity * 0.7);
      })
      .attr('stroke', '#fff')
      .attr('stroke-width', 2)
      .style('cursor', 'pointer')
      .on('click', (event, d) => {
        event.stopPropagation();
        handleNodeClick(d);
      })
      .on('mouseover', function(event, d) {
        d3.select(this)
          .transition()
          .duration(100)
          .attr('r', (d: any) => Math.max(12, Math.sqrt(d.rank || 1) * 4))
          .attr('stroke-width', 3);
      })
      .on('mouseout', function(event, d) {
        d3.select(this)
          .transition()
          .duration(100)
          .attr('r', (d: any) => Math.max(8, Math.sqrt(d.rank || 1) * 3))
          .attr('stroke-width', 2);
      })
      .call(d3.drag()
        .on('start', dragstarted)
        .on('drag', dragged)
        .on('end', dragended));

    const label = g.append('g')
      .attr('class', 'labels')
      .selectAll('text')
      .data(nodes)
      .enter().append('text')
      .attr('text-anchor', 'middle')
      .attr('dy', '.35em')
      .attr('font-size', '12px')
      .attr('font-family', 'Arial, sans-serif')
      .attr('fill', '#333')
      .attr('pointer-events', 'none')
      .text((d: any) => {
        // Truncate long labels
        return d.label.length > 15 ? d.label.substring(0, 15) + '...' : d.label;
      });

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
    
    // Add resize listener
    window.addEventListener('resize', updateDimensions);
    
    return () => {
      window.removeEventListener('resize', updateDimensions);
    };
  });
</script>

<svelte:window bind:innerWidth={width} bind:innerHeight={height} />

<!-- Full-screen graph container -->
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

<!-- Instructions overlay -->
{#if !loading && !error && nodes.length > 0}
  <div class="instructions">
    <p><i class="fas fa-info-circle"></i> Click nodes to view • Double-click to edit • Drag to rearrange • Scroll to zoom</p>
  </div>
{/if}

<!-- Modal for viewing/editing KG records -->
{#if selectedNode}
  <ArticleModal 
    bind:active={showModal} 
    item={selectedNode}
    initialEdit={startInEditMode}
    on:close={handleModalClose}
    on:save={handleModalSave}
  />
{/if}

<style>
  .graph-container {
    position: relative;
    background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
    border-radius: 8px;
    overflow: hidden;
    box-shadow: 0 4px 20px rgba(0, 0, 0, 0.1);
  }

  .graph-container.fullscreen {
    position: fixed;
    top: 0;
    left: 0;
    z-index: 1000;
    border-radius: 0;
    background: linear-gradient(135deg, #2c3e50 0%, #3498db 100%);
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
    z-index: 1001;
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

  .instructions {
    position: fixed;
    bottom: 20px;
    left: 50%;
    transform: translateX(-50%);
    z-index: 1001;
    background: rgba(255, 255, 255, 0.9);
    backdrop-filter: blur(10px);
    padding: 10px 20px;
    border-radius: 25px;
    box-shadow: 0 2px 15px rgba(0, 0, 0, 0.1);
    font-size: 0.9rem;
    color: #666;
  }

  .instructions i {
    color: #3498db;
    margin-right: 0.5rem;
  }

  /* Global styles for graph elements */
  :global(.graph-container svg) {
    background: transparent;
  }

  :global(.graph-container .links line) {
    transition: stroke-width 0.3s ease;
  }

  :global(.graph-container .nodes circle) {
    transition: all 0.3s ease;
    filter: drop-shadow(0 2px 4px rgba(0, 0, 0, 0.2));
  }

  :global(.graph-container .labels text) {
    text-shadow: 1px 1px 2px rgba(255, 255, 255, 0.8);
    font-weight: 500;
  }
</style> 