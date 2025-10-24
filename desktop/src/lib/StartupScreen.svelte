<script lang="ts">
import { open } from '@tauri-apps/api/dialog';
import { register as registerShortcut } from '@tauri-apps/api/globalShortcut';
import { appDataDir } from '@tauri-apps/api/path';
import { invoke } from '@tauri-apps/api/tauri';
import { appWindow } from '@tauri-apps/api/window';
import { onMount } from 'svelte';
import { isInitialSetupComplete } from '$lib/stores';

let dataFolder = '';
let globalShortcut = 'CmdOrControl+X';
let _error = '';
let isCapturingShortcut = false;

async function _selectFolder() {
	try {
		const selected = await open({
			directory: true,
			multiple: false,
			defaultPath: await appDataDir(),
		});
		console.log(selected);
		console.log(typeof selected);
		if (selected && typeof selected === 'string') {
			dataFolder = selected;
		} else {
			_error = 'No folder selected or invalid selection';
		}
	} catch (err) {
		console.error('Failed to open folder selector:', err);
		_error = `Failed to open folder selector: ${err.message}`;
	}
}

function _startCapturingShortcut() {
	isCapturingShortcut = true;
	globalShortcut = 'Press your desired shortcut...';
}

function handleKeyDown(event) {
	if (!isCapturingShortcut) return;

	event.preventDefault();

	const key = event.key.toUpperCase();
	const modifiers = [];

	if (event.ctrlKey) modifiers.push('Ctrl');
	if (event.altKey) modifiers.push('Alt');
	if (event.shiftKey) modifiers.push('Shift');
	if (event.metaKey) modifiers.push('Cmd');

	if (key !== 'CONTROL' && key !== 'ALT' && key !== 'SHIFT' && key !== 'META') {
		globalShortcut = [...modifiers, key].join('+');
		isCapturingShortcut = false;
	}
}

async function _saveSettings() {
	// Register the global shortcut
	try {
		await registerShortcut(globalShortcut, () => {
			if (appWindow.isVisible()) {
				appWindow.hide();
			}
		});
		console.log(`Global shortcut ${globalShortcut} registered successfully`);
	} catch (err) {
		_error = `Failed to register global shortcut: ${err.message}`;
		console.error('Failed to register global shortcut:', err);
		return;
	}
	if (!dataFolder || !globalShortcut) {
		_error = 'Please fill in both fields';
		return;
	}

	try {
		await invoke('save_initial_settings', {
			newSettings: {
				data_folder: dataFolder,
				global_shortcut: globalShortcut,
			},
		});
		alert('Settings saved successfully');
		await invoke('close_splashscreen');
	} catch (e) {
		_error = 'Failed to save settings';
		console.error(e);
	} finally {
		// set initial setup complete to true
		isInitialSetupComplete.set(true);
	}
}

onMount(() => {
	// unregisterAllShortcuts();
	document.addEventListener('keydown', handleKeyDown);
	return () => {
		document.removeEventListener('keydown', handleKeyDown);
	};
});
</script>
<svelte:head>
  <meta
    name="color-scheme"
    content={$theme == "spacelab" ? "lumen darkly" : $theme}
  />
  <link
    rel="stylesheet"
    href={`/assets/bulmaswatch/${$theme}/bulmaswatch.min.css`}
  />
</svelte:head>
<div class="startup-screen section">
  <div class="container">
    <h1 class="title is-2">Welcome to Terraphim AI</h1>
    <p class="subtitle">Please set up your initial settings:</p>
    <div class="field">
      <label class="label" for="data-folder">Data Folder Path:</label>
      <div class="control">
        <!-- <button class="button is-link" id="open-dialog" on:click={selectFolder}>Select path for your data</button> -->
        <input class="input" id="data-folder" type="text" readonly placeholder="Click to set path" bind:value={dataFolder} on:click={selectFolder}/>
      </div>
    </div>
    <div class="field">
      <label class="label" for="global-shortcut">Global Shortcut:</label>
      <div class="control">
        <input
          class="input"
          id="global-shortcut"
          type="text"
          bind:value={globalShortcut}
          readonly
          placeholder="Click to set shortcut"
          on:click={startCapturingShortcut}
        />
      </div>
    </div>
    {#if error}
      <p class="help is-danger">{error}</p>
    {/if}
    <div class="field">
      <div class="control">
        <button class="button is-success" on:click={saveSettings}>Save Settings</button>
      </div>
    </div>
  </div>
</div>

<style>
  .startup-screen {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100vh;
  }

  .container {
    max-width: 500px;
  }
</style>
