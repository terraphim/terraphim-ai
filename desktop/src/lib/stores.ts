import { writable } from "svelte/store";
import { CONFIG } from "../config";

// TypeScript interfaces for Rust types
export interface Role {
  name: string;
  theme: string;
  shortname?: string;
  kg?: { publish?: boolean };
}
interface NormalisedThesaurus {
  id: string;
  term: string;
}
// writable key value store for thesaurus, where value is id and normalised term
const thesaurus = writable<Array<Record<string, NormalisedThesaurus>>>([]);

interface Config {
  id: string;
  global_shortcut: string;
  roles: Record<string, Role>;
  default_role: string;
  selected_role: string;
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
  selected_role: ""
};

const theme = writable<string>("spacelab");
const role = writable<string>("selected"); // Updated to be empty by default, set upon config load
const is_tauri = writable<boolean>(false);
const atomic_configured = writable<boolean>(false);
const serverUrl = writable<string>(`${CONFIG.ServerURL}/documents/search`);
const configStore = writable<Config>(defaultConfig); // Store the whole config object
const isInitialSetupComplete = writable<boolean>(false);

// Roles should be an array of Role objects
const roles = writable<Role[]>([]);

let input = writable<string>("");
const typeahead = writable<boolean>(false);

export { configStore, input, is_tauri, role, roles, serverUrl, theme, typeahead, thesaurus, isInitialSetupComplete };