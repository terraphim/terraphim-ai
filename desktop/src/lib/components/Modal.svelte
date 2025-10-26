<script lang="ts">
import { createEventDispatcher } from 'svelte';

export let active: boolean = false;

const dispatch = createEventDispatcher();

function handleClose() {
	dispatch('close');
}

function handleBackgroundClick() {
	handleClose();
}

function handleKeydown(event: KeyboardEvent) {
	if (event.key === 'Escape' && active) {
		handleClose();
	}
}
</script>

<svelte:window on:keydown={handleKeydown} />

{#if active}
	<div class="modal is-active">
		<div
			class="modal-background"
			on:click={handleBackgroundClick}
			on:keydown={(e) => {
				if (e.key === 'Enter' || e.key === ' ') {
					e.preventDefault();
					handleBackgroundClick();
				}
			}}
			tabindex="0"
			role="button"
			aria-label="Close modal"
		></div>
		<slot />
		<button
			class="modal-close is-large"
			aria-label="close"
			on:click={handleClose}
		></button>
	</div>
{/if}

<style>
	.modal-background {
		cursor: pointer;
	}
</style>
