<script lang="ts">
	import { onMount } from 'svelte';

	export let fallbackPath: string = '/';
	export let showText: boolean = true;
	export let customClass: string = '';
	// Hide button on these paths (home by default)
	export let hideOnPaths: string[] = ['/'];

	let isVisible = true;

	function updateVisibility() {
		try {
			const path = window.location?.pathname || '/';
			isVisible = !hideOnPaths.includes(path);
		} catch (_) {
			isVisible = true;
		}
	}

	function goBack() {
		// Try to go back in browser history, fallback to specified path
		if (window.history.length > 1) {
			window.history.back();
		} else {
			window.location.href = fallbackPath;
		}
	}

	onMount(() => {
		updateVisibility();
		window.addEventListener('popstate', updateVisibility);
		window.addEventListener('hashchange', updateVisibility);
		return () => {
			window.removeEventListener('popstate', updateVisibility);
			window.removeEventListener('hashchange', updateVisibility);
		};
	});
</script>

{#if isVisible}
	<button 
		class="button is-light back-button {customClass}"
		on:click={goBack}
		on:keydown={(e) => {
			if (e.key === 'Enter' || e.key === ' ') {
				e.preventDefault();
				goBack();
			}
		}}
		title="Go back"
		aria-label="Go back"
	>
		<span class="icon">
			<i class="fas fa-arrow-left"></i>
		</span>
		{#if showText}
			<span class="back-text">Back</span>
		{/if}
	</button>
{/if}

<style>
	.back-button {
		/* Positioning */
		position: fixed;
		top: 1rem;
		left: 1rem;
		z-index: 1000;
		/* Layout */
		display: inline-flex;
		align-items: center;
		gap: 0.5rem;
	}
	
	/* Hover/active effects layer on top of Bulma button styles */
	.back-button:hover {
		transform: translateY(-1px);
		box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
	}
	
	.back-button:active {
		transform: translateY(0);
		box-shadow: 0 1px 2px rgba(0, 0, 0, 0.1);
	}
	
	.back-button .icon {
		font-size: 0.875rem;
	}
	
	.back-button .back-text {
		font-weight: 500;
	}
	
	/* Responsive design */
	@media (max-width: 768px) {
		.back-button {
			top: 0.5rem;
			left: 0.5rem;
		}
		.back-button .back-text {
			display: none;
		}
	}
</style>
