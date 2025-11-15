<script>
import {
	register as registerShortcut,
	unregisterAll as unregisterAllShortcuts,
	unregister as unregisterShortcut,
} from '@tauri-apps/api/globalShortcut';
import { appWindow } from '@tauri-apps/api/window';
import { writable } from 'svelte/store';

const selectedWindow = appWindow.label;
let isvisible = $state(true);
const _windowMap = {
	[selectedWindow]: appWindow,
};

let { onMessage } = $props();
const shortcuts = writable([]);
const shortcut = 'CmdOrControl+X';

function _register() {
	const shortcut_ = shortcut;
	registerShortcut(shortcut_, () => {
		if (isvisible) {
			appWindow.hide();
			isvisible = false;
		} else {
			appWindow.show();
			appWindow.setFocus();
			isvisible = true;
		}
		onMessage(`Shortcut ${shortcut_} triggered`);
	})
		.then(() => {
			shortcuts.update((shortcuts_) => [...shortcuts_, shortcut_]);
			onMessage(`Shortcut ${shortcut_} registered successfully`);
		})
		.catch(onMessage);
}

function _unregister(shortcut) {
	const shortcut_ = shortcut;
	unregisterShortcut(shortcut_)
		.then(() => {
			shortcuts.update((shortcuts_) => shortcuts_.filter((s) => s !== shortcut_));
			onMessage(`Shortcut ${shortcut_} unregistered`);
		})
		.catch(onMessage);
}

function _unregisterAll() {
	unregisterAllShortcuts()
		.then(() => {
			shortcuts.update(() => []);
			onMessage(`Unregistered all shortcuts`);
		})
		.catch(onMessage);
}
</script>

<div>
  <div>
    <input
      placeholder="Type a shortcut with '+' as separator..."
      bind:value={shortcut}
    />
    <button type="button" on:click={register}>Register</button>
  </div>
  <div>
    {#each $shortcuts as savedShortcut}
      <div>
        {savedShortcut}
        <button type="button" on:click={() => unregister(savedShortcut)}
          >Unregister</button
        >
      </div>
    {/each}
    {#if $shortcuts.length}
      <button type="button" on:click={unregisterAll}>Unregister all</button>
    {/if}
  </div>
</div>
