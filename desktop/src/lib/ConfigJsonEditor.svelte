<script lang="ts">
  import { JSONEditor } from "svelte-jsoneditor";
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/tauri";
  // @ts-ignore local store defined elsewhere
  import { is_tauri, configStore } from "$lib/stores";
  import { get } from "svelte/store";
  import { CONFIG } from "../config";
  import type { Config } from "./generated/types";

  type JsonEditorContent = { json: Config };

  let content: JsonEditorContent = {
    json: $configStore as Config,
  };

  type JsonEditorOnChange = (
    content: unknown,
    previousContent: unknown,
    status: unknown,
    ...rest: unknown[]
  ) => void;

  function handleJsonChange(updatedContent: JsonEditorContent) {
    console.log("contents changed:", updatedContent);
    console.log("is tauri", $is_tauri);

    const newConfig = updatedContent.json;
    configStore.set(newConfig);

    if (get(is_tauri)) {
      console.log("Updating config on server");
      invoke("update_config", { configNew: newConfig })
        .then((res) => {
          console.log(`Message: ${res}`);
        })
        .catch((e) => console.error(e));
    } else {
      const configURL = `${CONFIG.ServerURL}/config/`;
      fetch(configURL, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
        },
        body: JSON.stringify(newConfig),
      }).catch((e) => console.error("Failed to update config via HTTP:", e));
    }

    content = { json: newConfig };
  }

  const handleEditorChange: JsonEditorOnChange = (
    updatedContent,
    _previousContent,
    _status,
    ..._rest
  ) => {
    if (
      !updatedContent ||
      typeof updatedContent !== "object" ||
      !("json" in updatedContent)
    ) {
      console.warn("Ignoring non-JSON editor content update");
      return;
    }

    handleJsonChange({ json: (updatedContent as { json: Config }).json });
  };

  onMount(() => {
    // Initialize content with current config
    content = { json: $configStore as Config };
  });
</script>

<div class="box">
  <p>
    <i>The best editing experience is to configure Atomic Server, in the meantime use editor below. You will need to refresh page via Command R or Ctrl-R to see changes</i>
  </p>
  <div class="editor">
    <JSONEditor {content} onChange={handleEditorChange as any} />
  </div>
</div>
