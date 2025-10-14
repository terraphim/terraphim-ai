<script lang="ts">
import { onMount } from 'svelte';

export const fallbackPath: string = '/';
export const showText: boolean = true;
export const customClass: string = '';
// Hide button on these paths (home by default)
export const hideOnPaths: string[] = ['/'];

let _isVisible = true;

function updateVisibility() {
	try {
		const path = window.location?.pathname || '/';
		_isVisible = !hideOnPaths.includes(path);
	} catch (_) {
		_isVisible = true;
	}
}

function _goBack() {
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
		/* Layout - no fixed positioning */
		display: inline-flex;
		align-items: center;
		gap: 0.5rem;
		margin-right: 1rem;
		/* Ensure proper spacing */
		flex-shrink: 0;
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
			margin-right: 0.5rem;
		}
		.back-button .back-text {
			display: none;
		}
	}
</style>
