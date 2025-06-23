<script lang="ts">
  import { invoke } from "@tauri-apps/api/tauri";
  import { listen } from "@tauri-apps/api/event";
  import { CONFIG } from "../config";
  import { configStore, is_tauri, role, roles, theme, thesaurus, typeahead, type Role as RoleInterface } from "./stores";

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
    configStore.set(config);
    roles.set(Object.values(config.roles));
    role.set(config.selected_role);
    
    // Set theme based on selected role
    const selectedRoleSettings = config.roles[config.selected_role];
    console.log("Selected role settings:", selectedRoleSettings);
    
    if (selectedRoleSettings) {
      const newTheme = selectedRoleSettings.theme || "spacelab";
      console.log("Setting theme to:", newTheme);
      theme.set(newTheme);
      
      // Handle thesaurus publishing
      if (selectedRoleSettings.kg?.publish) {
        if ($is_tauri) {
          invoke("publish_thesaurus", { roleName: config.selected_role }).then((res) => {
            console.log("publish_thesaurus response", res);
            thesaurus.set(res as any);
            typeahead.set(true);
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

  initializeConfig();
  console.log("Using Terraphim Server URL:", CONFIG.ServerURL);

  function updateRole(event: Event) {
    const target = event.target as HTMLSelectElement;
    const newRoleName = target.value;
    console.log("Role change requested:", newRoleName);

    const roleSettings = $roles.find(r => r.name === newRoleName);
    if (!roleSettings) {
      console.error(`No role settings found for role: ${newRoleName}.`);
      return;
    }

    const newTheme = roleSettings.theme || 'spacelab';
    theme.set(newTheme);
    console.log(`Theme changed to ${newTheme}`);

    // Update selected role in the main config
    configStore.update(config => {
      config.selected_role = newRoleName;
      return config;
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
    }
  }
</script>

<div class="field is-grouped is-grouped-right">
  <div class="control">
    <div class="select">
      <select bind:value={$role} on:change={updateRole}>
        {#each $roles as r}
          <option value={r.name}>{r.name}</option>
        {/each}
      </select>
    </div>
  </div>
</div>
