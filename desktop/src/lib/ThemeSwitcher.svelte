<script lang="ts">
  import { invoke } from '@tauri-apps/api/tauri';
  import { theme, role, is_tauri, serverUrl } from './stores';
  import { Field, Select } from 'svelma';
  import { CONFIG } from '../config';
  let configStore =[];
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
              configStore = res.roles;
              role.set(Object.keys(configStore)[0]);
              console.log('Role', $role);
              console.log('Value', configStore[$role]['theme']);
              theme.set(configStore[$role]['theme']);
              console.log(Object.keys(configStore));
              console.log(configStore);
              console.log(typeof configStore);
          })
            .catch((e) => console.error(e))
        } else {
                    console.log('test is_tauri False');
          };
        
      } else {
        is_tauri.set(false);
        // configURL = `${CONFIG.ServerURL}/config/`;
        configURL = `http://localhost:${port}/config/`;
        console.log('test configURL ', configURL);
        fetch(configURL)
          .then(response => response.json())
          .then(data => {
            console.log('test data fetched', data);
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
      console.log('test configURL ', configURL);

    } catch (error) {
      console.error(error);
    }
    return configStore;
  }

  async function initializeConfig() {
    configStore = await loadConfig();
  }

  initializeConfig();
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
