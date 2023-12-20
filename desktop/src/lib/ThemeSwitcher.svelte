<script lang="ts">
  import { invoke } from '@tauri-apps/api/tauri';
  import { theme, role, is_tauri, serverUrl } from './stores';
  import { Field, Select } from 'svelma';
  import { CONFIG } from '../config';
  let configStore =[];
  export function loadConfig() {
    try {
      if (window.__TAURI__) {
        is_tauri.set(true);
        invoke('get_config')
          .then(res => {
            configStore = JSON.parse(res);
            role.set(Object.keys(configStore)[0]);
          })
          .catch(e => console.error(e));
      } else {
        fetch(`${CONFIG.ServerURL}/config`)
          .then(response => response.json())
          .then(data => {
            configStore = data.roles;
            role.set(Object.keys(configStore)[0]);
            console.log('Role', $role);
            console.log('Value', configStore[$role]['theme']);
            theme.set(configStore[$role]['theme']);
            console.log(Object.keys(configStore));
            console.log(configStore);
            console.log(typeof configStore);
          })
          .catch(e => console.error(e));
      }
    } catch (error) {
      console.error(error);
    }
    return configStore;
  }

  configStore = loadConfig();
  console.log('test ', configStore.length);
  console.log('test CONFIG.ServerURL ', CONFIG.ServerURL);
  let themes = '';
  $: if (themes) {
    role.set(themes);
    if (configStore[themes]!== undefined){
    console.log("Setting up theme and url from config");
    console.log(configStore[themes]);
    console.log(configStore[themes].serverUrl);
    theme.set(configStore[themes].theme);
    if (configStore[themes].serverUrl !== undefined) {
              console.log("Setting URL");
              console.log(configStore[themes].serverUrl);
              serverUrl.set(configStore[themes].serverUrl);
              
            }else{
              serverUrl.set(`${CONFIG.ServerURL}/search`);
              console.log("Fallback to default");
            }
  } else{
      console.log(configStore);
      console.log("Setting up default theme to spacelab");
      theme.set('spacelab');
      serverUrl.set(`${CONFIG.ServerURL}/search`);
      console.log(configStore);
    }
}
      

</script>

<Field grouped position="is-right">
  <Select bind:selected={themes}>
    {#each Object.values(configStore) as role_value}
      <option value={role_value.name}>{role_value.name}</option>
    {/each}
  </Select>
</Field>
