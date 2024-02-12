<script lang="ts">
  import { invoke } from '@tauri-apps/api/tauri';
  import { theme, role, is_tauri, serverUrl, configStore, roles } from './stores';
  import { Field, Select } from 'svelma';
  import { CONFIG } from '../config';
    import { event } from '@tauri-apps/api';
  // let configStore =[];
  let port = 8000;
  let configURL="";
  export async function loadConfig() {
    try {
      if (window.__TAURI__) {
        is_tauri.set(true);
        if (is_tauri) {
          console.log('test is_tauri True');
          invoke('get_config').then((res) =>{
              console.log(`Message: ${res.global_shortcut}`);
              configStore.update(config => {
                  // config["roles"] = res.roles;
                  config = res;
                  return config;
              });
              roles.update(roles => {
              roles = data.roles;
              return roles;
            });
              // configStore["roles"] = res.roles;
              // FIXME: set to default role
              role.set($configStore["roles"]["default"]);
              theme.set($role['theme']);
              console.log('Role', $role);
              theme.set(configStore[$role]['theme']);

          })
            .catch((e) => console.error(e))
        } else {
                    console.log('test is_tauri False');
          };
        
      } else {
        is_tauri.set(false);
        configURL = `${CONFIG.ServerURL}/config/`;
        // configURL = `http://localhost:${port}/config/`;
        console.log('test configURL ', configURL);
        fetch(configURL)
          .then(response => response.json())
          .then(data => {
            console.log('test data fetched', data);
            configStore.update(config => {
                  // config["roles"] = data.roles;
                  config = data;
                  return config;
              });
            roles.update(roles => {
              roles = data.roles;
              return roles;
            });
            // configStore = data.roles;
            console.log('test configStore', $configStore);
            console.log('test configStore default role ', $configStore["default_role"]);
            const role_value=$configStore["default_role"].toLowerCase();
            role.set(role_value);
            console.log('test role', $role);
            theme.set($roles[$role]['theme']);
            console.log("Keys in configstore", Object.keys($configStore));
            console.log($configStore);
            console.log(typeof $configStore);
          })
          .catch(e => console.error(e));
      }
      console.log('test configURL ', configURL);

    } catch (error) {
      console.error(error);
    }
    return configStore;
  }

  async function initializeConfig() {
    await loadConfig();
  }

  initializeConfig();
  // console.log('test ', configStore.length);
  console.log('test CONFIG.ServerURL ', CONFIG.ServerURL);
  function updateRole(event) {
    role.set(event.target.value.toLowerCase());
    theme.set($roles[$role]['theme']);
    console.log('Role changed:', event.target.value);
    console.log('Role changed name:', event.target.value);
    console.log('Role changed theme:', $theme);
  }

</script>

<div class="field is-grouped is-grouped-right">
  <div class="control">
    <div class="select">
      <select bind:value={$role} on:change={updateRole}>
        {#each Object.values($roles) as role_value}
          <option value={role_value.name.toLowerCase()}>{role_value.name}</option>
        {/each}
      </select>
    </div>
  </div>
</div>
