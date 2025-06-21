<script lang="ts">
  import { JSONEditor } from "svelte-jsoneditor";
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/tauri";
  // @ts-ignore local store defined elsewhere
  import { is_tauri, configStore } from "$lib/stores";
  import { get } from "svelte/store";
  import { CONFIG } from "../config";

  let content = {
    json: $configStore,
  };

  function handleChange(updatedContent) {
    console.log("contents changed:", updatedContent);
    console.log("is tauri", $is_tauri);
    configStore.update((config) => {
      config = updatedContent.json;
      return config;
    });
    if (get(is_tauri)) {
      console.log("Updating config on server");
      invoke("update_config", { configNew: updatedContent.json })
        .then((res) => {
          console.log(`Message: ${res}`);
        })
        .catch((e) => console.error(e));
    } else {
      // post to server using /api/config
      let configURL = `${CONFIG.ServerURL}/config/`;
      fetch(configURL, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
        },
        body: JSON.stringify(updatedContent.json),
      });
    }
    content = updatedContent;
    content;
  }

  onMount(() => {
    // Initialize content with current config
    content = { json: $configStore };
  });
</script>

<div class="box">
  <p>
    <i>The best editing experience is to configure Atomic Server, in the meantime use editor below. You will need to refresh page via Command R or Ctrl-R to see changes</i>
  </p>
  <div class="editor">
    <JSONEditor {content} onChange={handleChange} />
  </div>
</div> 