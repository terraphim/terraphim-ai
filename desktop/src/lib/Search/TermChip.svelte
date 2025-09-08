<script lang="ts">
  export let term: string;
  export let isFromKG: boolean = false;
  export let onRemove: (() => void) | null = null;
</script>

<span class="term-chip" class:from-kg={isFromKG}>
  {term}
  {#if onRemove}
    <button class="remove-btn" on:click|stopPropagation={onRemove} aria-label={`Remove term: ${term}`}>
      ×
    </button>
  {/if}
</span>
