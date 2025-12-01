<script lang="ts">
let {
	term,
	isFromKG = false,
	onRemove = null,
}: { term: string; isFromKG?: boolean; onRemove?: (() => void) | null } = $props();
</script>

<span class="term-chip" class:from-kg={isFromKG}>
  {term}
  {#if onRemove}
    <button class="remove-btn" on:click|stopPropagation={onRemove} aria-label={`Remove term: ${term}`}>
      ×
    </button>
  {/if}
</span>
