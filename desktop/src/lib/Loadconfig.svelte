<script context="module">
import { invoke } from "@tauri-apps/api/tauri";
import { theme,role, is_tauri} from './stores';

export function loadConfig(){
    let configStore=[];
    try {
        if (window.__TAURI__) {
            is_tauri.set(true);
            invoke("get_config").then((res) => {
            configStore=(JSON.parse(res));
            role.set(Object.keys(configStore)[0]); 
        
            }).catch((e) => console.error(e));
        } else {
        fetch('http://127.0.0.1:8000/config')
        .then(response => response.json())
        .then(data => {
            configStore=data.roles;
            role.set(Object.keys(configStore)[0]);
            console.log(configStore);
            console.log(typeof(configStore));
        }).catch((e) => console.error(e));
    }

    } catch (error) {
        console.error(error);
    }
    return configStore;
}
</script>