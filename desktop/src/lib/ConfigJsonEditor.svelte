<script lang="ts">
import { invoke } from '@tauri-apps/api/tauri';
import { onMount } from 'svelte';
import { get } from 'svelte/store';
// import { JSONEditor } from 'svelte-jsoneditor'; // Removed - using textarea instead
// @ts-expect-error local store defined elsewhere
import { configStore, is_tauri } from '$lib/stores';
import { CONFIG } from '../config';

let _content = {
	json: $configStore,
};

function _handleChange(updatedContent) {
	console.log('contents changed:', updatedContent);
	console.log('is tauri', $is_tauri);
	configStore.update((config) => {
		config = updatedContent.json;
		return config;
	});
	if (get(is_tauri)) {
		console.log('Updating config on server');
		invoke('update_config', { configNew: updatedContent.json })
			.then((res) => {
				console.log(`Message: ${res}`);
			})
			.catch((e) => console.error(e));
	} else {
		// post to server using /api/config
		const configURL = `${CONFIG.ServerURL}/config/`;
		fetch(configURL, {
			method: 'POST',
			headers: {
				'Content-Type': 'application/json',
			},
			body: JSON.stringify(updatedContent.json),
		});
	}
	_content = updatedContent;
	_content;
}

onMount(() => {
	// Initialize content with current config
	_content = { json: $configStore };
});
</script>

<div class="box">
  <p>
    <i>The best editing experience is to configure Atomic Server, in the meantime use editor below. You will need to refresh page via Command R or Ctrl-R to see changes</i>
  </p>
  <div class="editor">
    <textarea class="textarea" rows="20" bind:value={_content.json} on:change={() => _handleChange(_content)} style="font-family: monospace;"></textarea>
  </div>
</div>
