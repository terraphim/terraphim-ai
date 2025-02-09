<script lang="ts">
  import { invoke } from "@tauri-apps/api/tauri";
  import { CONFIG } from "../config";
  import { configStore, is_tauri, role, roles, theme, thesaurus, typeahead } from "./stores";

  interface ConfigResponse {
    status: string;
    config: {
      id: string;
      global_shortcut: string;
      roles: Record<string, { name: string; theme: string; kg?: { publish?: boolean } }>;
      default_role: string;
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
              configStore.set(res.config);
              roles.set(res.config.roles);
              role.set(res.config.selected_role || res.config.default_role);
              theme.set(
                res.config.roles[res.config.default_role]?.theme || "default"
              );
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
              configStore.set(received_config.config);
              roles.set(received_config.config.roles);
              role.set(received_config.config.selected_role || received_config.config.default_role);
              theme.set(
                received_config.config.roles[
                  received_config.config.default_role
                ]?.theme || "default"
              );
            }
          })
          .catch((error) => console.error("Error fetching config:", error));
      }
    } catch (error) {
      console.error("Unhandled error in loadConfig:", error);
    }
  }

  async function initializeConfig() {
    await loadConfig();
  }

  initializeConfig();
  console.log("Using Terraphim Server URL:", CONFIG.ServerURL);

  function updateRole(event: Event) {
    const target = event.target as HTMLSelectElement;
    console.log("updateRole event received:", event);
    console.log("Setting role to", target.value);
    var selectedTheme = "default";
    role.set(target.value);
    console.log($roles);
    
    // Update the config with the new selected role
    configStore.update(config => {
      return {
        ...config,
        selected_role: target.value
      };
    });

    // Send updated config to server
    if ($is_tauri) {
      console.log("Updating config on server");
      invoke("update_config", { configNew: $configStore })
        .then((res: any) => {
          console.log(`Config updated: ${res}`);
        })
        .catch((error) => console.error(error));
    } else {
      const configURL = `${CONFIG.ServerURL}/config/`;
      fetch(configURL, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify($configStore),
      })
        .then(response => response.json())
        .then(data => console.log('Config updated:', data))
        .catch((error) => console.error('Error updating config:', error));
    }

    // Find role settings in the roles record
    const roleSettings = Object.values($roles).find(r => r.name === target.value);
    console.log("Role settings ", roleSettings);
    if (roleSettings) {
      // if role have kg.publish=true then publish thesaurus
      if (roleSettings.kg?.publish) {
      console.log("Publishing thesaurus for role", $role);
      invoke("publish_thesaurus", { roleName: $role }).then((res: any) => {
        console.log("publish_thesaurus response", res);
        // update thesaurus store
        if (res && res.data) {
          thesaurus.set(res.data);
          typeahead.set(true);
        }
      });
      }else{
        typeahead.set(false);
      }
      selectedTheme = roleSettings.theme;
    }else{
      console.error(
        `No role settings found for role: ${$role}. Using default theme.`
      );
    }
    
    if (!selectedTheme) {
      console.error(
        `No theme defined for role: ${$role}. Using default theme.`
      );
    } else {
      theme.set(selectedTheme);
      console.log("New theme:", $theme);
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
