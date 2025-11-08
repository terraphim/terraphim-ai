<script lang="ts">
	import { marked } from 'marked';

	export let source: string = '';

	$: html = marked(source);
</script>

<div class="markdown-content">{@html html}</div>

<style>
	/* Basic markdown styling */
	.markdown-content {
		line-height: 1.6;
		color: #333;
	}

	.markdown-content :global(h1) {
		font-size: 2em;
		margin-bottom: 0.5em;
		font-weight: bold;
	}

	.markdown-content :global(h2) {
		font-size: 1.5em;
		margin-bottom: 0.5em;
		font-weight: bold;
	}

	.markdown-content :global(h3) {
		font-size: 1.25em;
		margin-bottom: 0.5em;
		font-weight: bold;
	}

	.markdown-content :global(h4) {
		font-size: 1.1em;
		margin-bottom: 0.5em;
		font-weight: bold;
	}

	.markdown-content :global(p) {
		margin-bottom: 1em;
	}

	.markdown-content :global(ul), .markdown-content :global(ol) {
		margin-bottom: 1em;
		padding-left: 2em;
	}

	.markdown-content :global(li) {
		margin-bottom: 0.25em;
	}

	.markdown-content :global(blockquote) {
		border-left: 4px solid #ddd;
		margin: 0 0 1em 0;
		padding: 0.5em 1em;
		background-color: #f9f9f9;
		font-style: italic;
	}

	.markdown-content :global(code) {
		background-color: #f5f5f5;
		border-radius: 3px;
		padding: 0.1em 0.3em;
		font-family: 'Monaco', 'Menlo', 'Ubuntu Mono', monospace;
		font-size: 0.9em;
	}

	.markdown-content :global(pre) {
		background-color: #f5f5f5;
		border-radius: 5px;
		padding: 1em;
		margin-bottom: 1em;
		overflow-x: auto;
	}

	.markdown-content :global(pre code) {
		background: none;
		padding: 0;
	}

	.markdown-content :global(a) {
		color: #3273dc;
		text-decoration: none;
	}

	.markdown-content :global(a:hover) {
		text-decoration: underline;
	}

	.markdown-content :global(table) {
		border-collapse: collapse;
		width: 100%;
		margin-bottom: 1em;
	}

	.markdown-content :global(th), .markdown-content :global(td) {
		border: 1px solid #ddd;
		padding: 8px;
		text-align: left;
	}

	.markdown-content :global(th) {
		background-color: #f2f2f2;
		font-weight: bold;
	}

	.markdown-content :global(hr) {
		border: none;
		border-top: 2px solid #eee;
		margin: 2em 0;
	}
</style>