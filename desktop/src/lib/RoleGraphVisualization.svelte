<script lang="ts">
  import { onMount } from 'svelte';
  import * as d3 from 'd3';

  export let apiUrl: string = '/rolegraph';
  export let width: number = 600;
  export let height: number = 400;

  let container: HTMLDivElement;
  let loading = true;
  let error: string | null = null;
  let nodes: any[] = [];
  let edges: any[] = [];

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

  function renderGraph() {
    if (!container) return;
    container.innerHTML = '';
    const svg = d3.select(container)
      .append('svg')
      .attr('width', width)
      .attr('height', height);

    const simulation = d3.forceSimulation(nodes)
      .force('link', d3.forceLink(edges).id((d: any) => d.id).distance(120))
      .force('charge', d3.forceManyBody().strength(-300))
      .force('center', d3.forceCenter(width / 2, height / 2));

    const link = svg.append('g')
      .attr('stroke', '#aaa')
      .selectAll('line')
      .data(edges)
      .enter().append('line')
      .attr('stroke-width', 2);

    const node = svg.append('g')
      .attr('stroke', '#fff')
      .attr('stroke-width', 1.5)
      .selectAll('circle')
      .data(nodes)
      .enter().append('circle')
      .attr('r', 18)
      .attr('fill', '#69b3a2')
      .call(d3.drag()
        .on('start', dragstarted)
        .on('drag', dragged)
        .on('end', dragended));

    const label = svg.append('g')
      .selectAll('text')
      .data(nodes)
      .enter().append('text')
      .attr('text-anchor', 'middle')
      .attr('dy', 4)
      .attr('font-size', 13)
      .text((d: any) => d.label);

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

  onMount(async () => {
    await fetchGraph();
    if (!error) renderGraph();
  });
</script>

<div bind:this={container} style="width: {width}px; height: {height}px; border: 1px solid #ccc; background: #fafafa; margin: 1em 0;"></div>
{#if loading}
  <div>Loading graph...</div>
{:else if error}
  <div style="color: red">Error: {error}</div>
{/if} 