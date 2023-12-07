import { writable } from 'svelte/store';
import { invoke } from '@tauri-apps/api/tauri';
import { CONFIG } from '../config';
const theme = writable('spacelab');
const role = writable('selected');
const is_tauri = writable(false);
const atomic_configured = writable(false);
const serverUrl=writable(`${CONFIG.ServerURL}/search`);

let input = writable('');
export { theme, role, is_tauri, input, serverUrl};
