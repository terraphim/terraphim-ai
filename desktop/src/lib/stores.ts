import { writable } from "svelte/store";
import { CONFIG } from "../config";

// TypeScript interfaces for Rust types
interface Role {
  name: string;
  theme: string;
}

interface Config {
  id: string;
  global_shortcut: string;
  roles: Record<string, Role>;
  default_role: string;
}

interface ConfigResponse {
  status: string;
  config: Config;
}

// Default empty configuration
const defaultConfig: Config = {
  id: "",
  global_shortcut: "",
  roles: {},
  default_role: "",
};

const theme = writable<string>("spacelab");
const role = writable<string>("selected"); // Updated to be empty by default, set upon config load
const is_tauri = writable<boolean>(false);
const atomic_configured = writable<boolean>(false);
const serverUrl = writable<string>(`${CONFIG.ServerURL}/documents/search`);
const configStore = writable<Config>(defaultConfig); // Store the whole config object

// FIXME: add default role
const roles = writable<Record<string, Role>>({}); // Store roles separately for easier access

let input = writable<string>("");

export { configStore, input, is_tauri, role, roles, serverUrl, theme };
