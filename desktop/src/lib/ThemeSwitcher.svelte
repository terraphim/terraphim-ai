<script lang="ts">
  import { invoke } from "@tauri-apps/api/tauri";
  import { listen } from "@tauri-apps/api/event";
  import { CONFIG } from "../config";
  import { configStore, is_tauri, role, roles, theme, thesaurus, typeahead } from "./stores";

  interface ConfigResponse {
    status: string;
    config: {
      id: string;
      global_shortcut: string;
      roles: { [key: string]: { name: string; theme: string; kg?: { publish?: boolean } } };
      selected_role: string;
    };
  }

  let configURL = "";
  export async function loadConfig() {
    try {
      is_tauri.set(window.__TAURI__ !== undefined);
      if ($is_tauri) {
        console.log("test is_tauri True");
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
        console.log("test is_tauri False");
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

  function updateStoresFromConfig(config: any) {
    configStore.set(config);
    roles.set(Object.values(config.roles) as any);
    role.set(config.selected_role);
    
    // Set theme based on selected role
    const selectedRoleSettings = config.roles[config.selected_role];
    if (selectedRoleSettings) {
      theme.set(selectedRoleSettings.theme || "default");
      
      // Handle thesaurus publishing
      if (selectedRoleSettings.kg?.publish) {
        console.log("Publishing thesaurus for role", config.selected_role);
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
    }
  }

  // Listen for role changes from the backend (e.g., from system tray)
  if (typeof window !== 'undefined' && window.__TAURI__) {
    listen('role_changed', (event) => {
      console.log('Role changed event received from backend:', event.payload);
      updateStoresFromConfig(event.payload);
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
    console.log("updateRole event received:", event);
    console.log("Setting role to", newRoleName);

    if ($is_tauri) {
      // Use the centralized select_role command
      invoke("select_role", { roleName: newRoleName })
        .then((res: ConfigResponse) => {
          console.log("select_role response", res);
          if (res && res.status === "success") {
            // The backend will emit a role_changed event, which our listener will handle
            // But we can also update immediately for better UX
            updateStoresFromConfig(res.config);
          }
        })
        .catch((e) => console.error("Error selecting role:", e));
    } else {
      // For non-Tauri environments, fall back to the old method
      var selectedTheme = "default";
      role.set(newRoleName);
      
      const rolesArray = Array.isArray($roles) ? $roles : Object.values($roles);
      const roleSettings = rolesArray.find((r: any) => r.name === newRoleName);
      console.log("Role settings ", roleSettings);

      if (roleSettings) {
        configStore.update(config => {
          config.selected_role = roleSettings.shortname || newRoleName;
          return config;
        });
        
        if (roleSettings.kg?.publish) {
          console.log("Publishing thesaurus for role", newRoleName);
          typeahead.set(true);
        } else {
          typeahead.set(false);
        }
        selectedTheme = roleSettings.theme;
      }
      
      theme.set(selectedTheme);
    }
  }
</script>

<div class="field is-grouped is-grouped-right">
  <div class="control">
    <div class="select">
      <select bind:value={$role} on:change={updateRole}>
        {#each Object.values($roles) as { name, theme }}
          <option value={name}>{name}</option>
        {/each}
      </select>
    </div>
  </div>
</div>
