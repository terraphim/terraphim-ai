import { writable } from 'svelte/store';
import { invoke } from '@tauri-apps/api/tauri';
import { CONFIG } from '../config';
const theme = writable('spacelab');
const role = writable('selected');
const is_tauri = writable(false);
const atomic_configured = writable(false);
const serverUrl=writable(`${CONFIG.ServerURL}/articles/search`);
const configStore = writable([]);
// FIXME: add default role
const roles = writable({});

let input = writable('');
export { theme, role, is_tauri, input, serverUrl,configStore, roles};
