<script lang="ts">
  import { onMount, afterUpdate } from 'svelte';
  import Icicle from 'icicle-chart';
  import type { ChartData } from './SearchResult';

  export let data: ChartData;
  export let width: number = 800;
  export let height: number = 600;

  let chartContainer: HTMLElement;
  let error: string | null = null;
  let chart: any = null;

  function validateData(data: ChartData) {
    if (!data) throw new Error('Data is null or undefined');
    if (!data.children) throw new Error('Data has no children array');
    if (data.children.length === 0) throw new Error('Data children array is empty');
    
    // Basic validation without stringifying the entire structure
    data.children.forEach((node, index) => {
      if (!node.name) throw new Error(`Node ${index} missing name property`);
      if (typeof node.size !== 'number') throw new Error(`Node ${index} missing or invalid size property`);
      if (!node.color) throw new Error(`Node ${index} missing color property`);
    });

    return true;
  }

  function initChart() {
    try {
      if (!chartContainer) return;

      validateData(data);

      // Clear any existing chart
      while (chartContainer.firstChild) {
        chartContainer.removeChild(chartContainer.firstChild);
      }

      // Create new chart instance
      const myChart = Icicle();
      
      // Configure the chart before setting data
      myChart
        .width(width)
        .height(height)
        .orientation('td')
        .size('size')
        .label('name')
        .color('color')
        .minSegmentWidth(1)
        .excludeRoot(false)
        .showLabels(true);

      // Add click handler
      myChart.onClick((node: any) => {
        if (node) {
          myChart.focusOnNode(node);
        } else {
          myChart.focusOnNode(null);
        }
      });

      // Set data and render
      myChart.data(data);
      
      // Mount the chart
      myChart(chartContainer);
      
      // Store reference
      chart = myChart;

    } catch (e) {
      console.error('Error initializing chart:', e);
      error = `Failed to initialize chart: ${e}`;
    }
  }

  onMount(() => {
    if (data) {
      initChart();
    }
    
    // Add resize observer
    const resizeObserver = new ResizeObserver(() => {
      if (chart) {
        chart
          .width(chartContainer.offsetWidth)
          .height(chartContainer.offsetHeight);
      }
    });
    
    resizeObserver.observe(chartContainer);
    
    return () => {
      resizeObserver.disconnect();
      if (chartContainer) {
        while (chartContainer.firstChild) {
          chartContainer.removeChild(chartContainer.firstChild);
        }
      }
    };
  });

  afterUpdate(() => {
    if (data && chartContainer) {
      initChart();
    }
  });
</script>

<div class="chart-wrapper">
  {#if error}
    <div class="error" data-testid="chart-error">
      {error}
    </div>
  {/if}
  
  <div 
    bind:this={chartContainer} 
    class="chart-container" 
    data-testid="icicle-chart"
  />
</div>

<style>
  .chart-wrapper {
    position: relative;
    width: 100%;
    height: 100%;
  }

  .chart-container {
    width: 100%;
    height: 100%;
    min-height: 400px;
    background-color: #fff;
  }

  .error {
    position: absolute;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    padding: 1rem;
    background-color: #fee;
    border: 1px solid #f88;
    border-radius: 4px;
    color: #c00;
    z-index: 1;
  }
</style> 