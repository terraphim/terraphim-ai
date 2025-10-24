<script lang="ts">
import { onMount } from 'svelte';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/tauri';
import { CONFIG } from '../config';
import type { Role as RoleInterface } from './generated/types';
import { configStore, is_tauri, role, roles, theme } from './stores';

interface ConfigResponse {
	status: string;
	config: {
		id: string;
		global_shortcut: string;
		roles: { [key: string]: RoleInterface };
		selected_role: string;
	};
}

let configURL = '';
export async function loadConfig() {
	try {
		is_tauri.set(window.__TAURI__ !== undefined);
		if ($is_tauri) {
			console.log('Loading config from Tauri');
			invoke<ConfigResponse>('get_config')
				.then((res) => {
					console.log('get_config response', res);
					if (res && res.status === 'success') {
						updateStoresFromConfig(res.config);
					}
				})
				.catch((err) => {
					console.error('Error loading config from Tauri:', err);
				});
		} else {
			console.log('Loading config from REST API');
			// For web mode, load from API endpoint
			try {
				const response = await fetch(`${CONFIG.ServerURL}/config`);
				if (response.ok) {
					const data = await response.json();
					if (data.status === 'success') {
						updateStoresFromConfig(data.config);
					}
				}
			} catch (err) {
				console.error('Error loading config from API:', err);
			}
		}
	} catch (err) {
		console.error('Error in loadConfig:', err);
	}
}

function updateStoresFromConfig(config: ConfigResponse['config']) {
	// Update config store
	configStore.set(config);

	// Update roles store
	const roleArray = Object.entries(config.roles).map(([key, value]) => ({
		...value,
		name: key
	}));
	roles.set(roleArray);

	// Update theme
	if (config.global_shortcut) {
		theme.set(config.global_shortcut);
	}

	// Update selected role
	if (config.selected_role) {
		role.set(config.selected_role);
	}
}

export async function saveConfig(config: any) {
	try {
		if ($is_tauri) {
			console.log('Saving config to Tauri');
			const response = await invoke('save_config', { config });
			console.log('save_config response', response);
			return response;
		} else {
			console.log('Saving config to REST API');
			const response = await fetch(`${CONFIG.ServerURL}/config`, {
				method: 'POST',
				headers: {
					'Content-Type': 'application/json'
				},
				body: JSON.stringify(config)
			});
			const data = await response.json();
			console.log('save config API response', data);
			return data;
		}
	} catch (err) {
		console.error('Error saving config:', err);
		throw err;
	}
}

// Theme switching functionality
export function switchTheme(newTheme: string) {
	theme.set(newTheme);
	if ($is_tauri) {
		invoke('set_theme', { theme: newTheme })
			.catch(err => console.error('Error setting theme:', err));
	}
	// For web mode, theme is handled by CSS variables
}

// Listen for theme changes from Tauri backend
onMount(() => {
	if ($is_tauri) {
		listen('theme-changed', (event: any) => {
			theme.set(event.payload);
		});
	}
});
</script>