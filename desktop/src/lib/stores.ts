import { writable } from "svelte/store";
import { CONFIG } from "../config";
// Import generated types instead of manual definitions
import type {
  Role,
  Config,
  ConfigResponse,
  RoleName
} from "./generated/types";

// Custom interface for thesaurus (not in generated types)
interface NormalisedThesaurus {
  id: string;
  term: string;
}

// writable key value store for thesaurus, where value is id and normalised term
const thesaurus = writable<Array<Record<string, NormalisedThesaurus>>>([]);

// Default empty configuration - updated to match generated Config type
const defaultConfig: Config = {
  id: "Desktop" as const,
  global_shortcut: "",
  roles: {} as Record<string, Role>,
  default_role: { original: "", lowercase: "" } as RoleName,
  selected_role: { original: "", lowercase: "" } as RoleName
};

const theme = writable<string>("spacelab");
const role = writable<string>("selected"); // Updated to be empty by default, set upon config load
const is_tauri = writable<boolean>(false);
const atomic_configured = writable<boolean>(false);
const serverUrl = writable<string>(`${CONFIG.ServerURL}/documents/search`);
const configStore = writable<Config>(defaultConfig); // Store the whole config object
const isInitialSetupComplete = writable<boolean>(false);

// Roles should be an array of Role objects - using generated Role type
const roles = writable<Role[]>([]);

let input = writable<string>("");
const typeahead = writable<boolean>(false);

export { configStore, input, is_tauri, role, roles, serverUrl, theme, typeahead, thesaurus, isInitialSetupComplete };
