<script lang="ts">
  import { invoke } from "@tauri-apps/api/tauri";
  import { theme, role, is_tauri, configStore, roles } from "./stores";
  import { CONFIG } from "../config";

  let configURL = "";
  export async function loadConfig() {
    try {
      if (window.__TAURI__) {
        is_tauri.set(true);
        if (is_tauri) {
          console.log("test is_tauri True");
          invoke("get_config")
            .then((res) => {
              console.log("get_config response", res);
              console.log("Updating config store");
              configStore.update((config) => {
                // config["roles"] = res.roles;
                config = res.config;
                return config;
              });
              console.log("Updating roles");
              roles.update((roles) => {
                roles = $configStore["roles"];
                return roles;
              });

              const default_role = $configStore["default_role"];
              if (!default_role) {
                console.error("No default role defined in config!");
                return;
              }
              role.set(default_role);
              console.log("Role", $role);

              theme.set($roles[$role]["theme"]);
              console.log("Theme", $theme);
            })
            .catch((e) => console.error(e));
        } else {
          console.log("test is_tauri False");
        }
      } else {
        is_tauri.set(false);
        configURL = `${CONFIG.ServerURL}/config/`;
        fetch(configURL)
          .then((response) => response.json())
          .then((received_config) => {
            console.log("Config received", received_config);
            configStore.update((config) => {
              config = received_config.config;
              return config;
            });
            console.log("Config store updated");
            roles.update((roles) => {
              roles = received_config.config.roles;
              return roles;
            });
            console.log("Default role:", $configStore["default_role"]);
            const default_role = $configStore["default_role"];
            role.set(default_role);
            // Check that we have a theme for the role
            if ($roles[$role]["theme"] === undefined) {
              console.error(
                `No theme defined for role: ${$role}. Using default theme.`
              );
            } else {
              theme.set($roles[$role]["theme"]);
            }
          })
          .catch((e) => console.error(e));
      }
    } catch (error) {
      console.error(error);
    }
    return configStore;
  }

  async function initializeConfig() {
    await loadConfig();
  }

  initializeConfig();
  console.log("Using Terraphim Server URL:", CONFIG.ServerURL);
  function updateRole(event) {
    console.log("updateRole event received:", event);
    console.log("Setting role to", event.target.value);
    role.set(event.target.value);

    if ($roles[$role]["theme"] === undefined) {
      console.error(
        `No theme defined for role: ${$role}. Using default theme.`
      );
    } else {
      theme.set($roles[$role]["theme"]);
      console.log("New theme:", $theme);
    }
  }
</script>

<div class="field is-grouped is-grouped-right">
  <div class="control">
    <div class="select">
      <select bind:value={$role} on:change={updateRole}>
        {#each Object.values($roles) as role_value}
          <option value={role_value.name}>{role_value.name}</option>
        {/each}
      </select>
    </div>
  </div>
</div>
