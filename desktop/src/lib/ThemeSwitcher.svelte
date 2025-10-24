<script lang="ts">
  import { invoke } from "@tauri-apps/api/tauri";
  import { listen } from "@tauri-apps/api/event";
  import { CONFIG } from "../config";
  import { configStore, is_tauri, role, roles, theme, thesaurus, typeahead } from "./stores";
  import type { Role as RoleInterface } from "./generated/types";

  interface ConfigResponse {
    status: string;
    config: {
      id: string;
      global_shortcut: string;
      roles: { [key: string]: RoleInterface };
      selected_role: string;
    };
  }

  let configURL = "";
  export async function loadConfig() {
    try {
      is_tauri.set(window.__TAURI__ !== undefined);
      if ($is_tauri) {
        console.log("Loading config from Tauri");
        invoke<ConfigResponse>("get_config")
          .then((res) => {
            console.log("get_config response", res);
            if (res && res.status === "success") {
              updateStoresFromConfig(res.config);
            }
          })
          .catch((error) =>
            console.error("Error fetching config in Tauri:", error)
          );
      } else {
        console.log("Loading config from server");
        configURL = `${CONFIG.ServerURL}/config/`;
        fetch(configURL)
          .then((response) => response.json())
          .then((received_config: ConfigResponse) => {
            console.log("Config received", received_config);
            if (received_config && received_config.status === "success") {
              updateStoresFromConfig(received_config.config);
            }
          })
          .catch((error) => console.error("Error fetching config:", error));
      }
    } catch (error) {
      console.error("Unhandled error in loadConfig:", error);
    }
  }

  function updateStoresFromConfig(config: ConfigResponse['config']) {
    console.log("Updating stores from config:", config);
    // The global Config interface expects a `default_role` property which older
    // backend versions might omit.  Provide a sensible fallback so TypeScript
    // type-checks and downstream code remains happy.
    const fullConfig = {
      default_role: config.selected_role,
      ...config,
    };
    // Cast is safe: we just guaranteed the presence of every required key.
    configStore.set(fullConfig as any);
    // Convert the roles map (keyed by role name) to an array and inject the
    // `name` field so that each entry is self-contained. This makes look-ups by
    // role name trivial later on.
    const rolesArray = Object.entries(config.roles).map(([roleName, settings]) => {
      const { name: existingName, ...rest } = settings as RoleInterface & { name?: string };
      return {
        name: existingName ?? roleName,
        ...rest,
      };
    });
    roles.set(rolesArray);
    role.set(config.selected_role);

    // Set theme based on selected role
    const selectedRoleSettings = config.roles[config.selected_role];
    console.log("Selected role settings:", selectedRoleSettings);

    if (selectedRoleSettings) {
      const newTheme = selectedRoleSettings.theme || "spacelab";
      console.log("Setting theme to:", newTheme);
      theme.set(newTheme);

      // Handle thesaurus publishing with storage quota management
      if (selectedRoleSettings.kg?.publish) {
        if ($is_tauri) {
          invoke("publish_thesaurus", { roleName: config.selected_role })
            .then((res) => {
              console.log("publish_thesaurus response", res);
              thesaurus.set(res as any);
              typeahead.set(true);
            })
            .catch((error) => {
              console.error("Error publishing thesaurus:", error);
              typeahead.set(false);
            });
        } else {
          // Web mode: fetch thesaurus from HTTP endpoint with storage management
          console.log("Fetching thesaurus from HTTP endpoint for role", config.selected_role);

          fetch(`${CONFIG.ServerURL}/thesaurus/${encodeURIComponent(config.selected_role)}`)
            .then((response) => response.json())
            .then((res) => {
              console.log("thesaurus HTTP response", res);
              if (res && res.status === "success" && res.thesaurus) {
                thesaurus.set(res.thesaurus);
                typeahead.set(true);
              } else {
                console.error("Failed to fetch thesaurus:", res);
                typeahead.set(false);
              }
            })
            .catch((error) => {
              console.error("Error fetching thesaurus:", error);
              typeahead.set(false);
            });
        }
      } else {
        typeahead.set(false);
      }
    } else {
      console.warn("No role settings found for:", config.selected_role);
      theme.set("spacelab"); // Default theme
    }
  }

  // Listen for role changes from the backend (e.g., from system tray)
  if (typeof window !== 'undefined' && window.__TAURI__) {
    listen('role_changed', (event) => {
      console.log('Role changed event received from backend:', event.payload);
      updateStoresFromConfig(event.payload as ConfigResponse['config']);
    });
  }

  async function initializeConfig() {
    await loadConfig();
  }

  // Initialize config on component mount
  initializeConfig();
  console.log("Using Terraphim Server URL:", CONFIG.ServerURL);

  function updateRole(event: Event) {
    const target = event.currentTarget as HTMLSelectElement;
const newRoleName = target.value;
    console.log("Role change requested:", newRoleName);

    // Persist the newly selected role in the dedicated store **first** so that
    // any reactive subscribers update immediately (e.g. App.svelte head link).
    role.set(newRoleName);

    // Handle both string and RoleName object types for name field
    const roleSettings = $roles.find((r) => {
      const roleName = typeof r.name === 'string' ? r.name : r.name.original;
      return roleName === newRoleName;
    });
    if (!roleSettings) {
      console.error(`No role settings found for role: ${newRoleName}.`);
      return;
    }

    const newTheme = roleSettings.theme || "spacelab";
    theme.set(newTheme);
    console.log(`Theme changed to ${newTheme}`);

    // Update selected role in the main config object
    configStore.update((cfg) => {
      // Handle RoleName type - convert string to RoleName if needed
      cfg.selected_role = typeof cfg.selected_role === 'string'
        ? { original: newRoleName, lowercase: newRoleName.toLowerCase() } as any
        : { original: newRoleName, lowercase: newRoleName.toLowerCase() } as any;
      return cfg;
    });

    // In Tauri, notify the backend about the role change
    if ($is_tauri) {
      invoke("select_role", { roleName: newRoleName })
        .catch((e) => console.error("Error selecting role:", e));

      // Handle thesaurus publishing
      if (roleSettings.kg?.publish) {
        console.log("Publishing thesaurus for role", newRoleName);
        invoke("publish_thesaurus", { roleName: newRoleName }).then((res) => {
          thesaurus.set(res as any);
          typeahead.set(true);
        });
      } else {
        typeahead.set(false);
      }
    } else {
        // For non-Tauri, update the config on the server
        fetch(`${CONFIG.ServerURL}/config/`, {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify($configStore),
        }).catch(error => console.error("Error updating config on server:", error));

        // Handle thesaurus publishing in web mode
        if (roleSettings.kg?.publish) {
          console.log("Fetching thesaurus from HTTP endpoint for role", newRoleName);
          fetch(`${CONFIG.ServerURL}/thesaurus/${encodeURIComponent(newRoleName)}`)
            .then((response) => response.json())
            .then((res) => {
              console.log("thesaurus HTTP response", res);
              if (res && res.status === "success" && res.thesaurus) {
                thesaurus.set(res.thesaurus);
                typeahead.set(true);
              } else {
                console.error("Failed to fetch thesaurus:", res);
                typeahead.set(false);
              }
            })
            .catch((error) => {
              console.error("Error fetching thesaurus:", error);
              typeahead.set(false);
            });
        } else {
          typeahead.set(false);
        }
    }
  }
</script>

<div class="field is-grouped is-grouped-right">
  <div class="control">
    <div class="select">
      <!-- We set the current value explicitly and handle updates via `updateRole`.
           Direct store binding with `$role` is avoided because `$role` is read-only. -->
      <select value={$role} on:change={updateRole}>
        {#each $roles as r}
          {@const roleName = typeof r.name === 'string' ? r.name : r.name.original}
          <option value={roleName}>{roleName}</option>
        {/each}
      </select>
    </div>
  </div>
</div>
