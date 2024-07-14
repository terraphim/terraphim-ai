<script lang="ts">
  import { invoke } from "@tauri-apps/api/tauri";
  import { open } from "@tauri-apps/api/dialog";
  import { onMount } from "svelte";
  import { readDir } from "@tauri-apps/api/fs";
  import { resolve,appDir,appDataDir } from "@tauri-apps/api/path";

  let dataFolder = "";
  let globalShortcut = "";
  let error = "";
  let isCapturingShortcut = false;

  async function selectFolder() {
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        defaultPath: await appDir()
      });
      console.log(selected);
      console.log(typeof selected);
      if (selected && typeof selected === "string") {
        dataFolder = selected;
      } else {
        error = "No folder selected or invalid selection";
      }
    } catch (err) {
      console.error("Failed to open folder selector:", err);
      error = `Failed to open folder selector: ${err.message}`;
    }
  }
  function startCapturingShortcut() {
    isCapturingShortcut = true;
    globalShortcut = "Press your desired shortcut...";
  }
  function handleKeyDown(event) {
    if (!isCapturingShortcut) return;

    event.preventDefault();

    const key = event.key.toUpperCase();
    const modifiers = [];

    if (event.ctrlKey) modifiers.push("Ctrl");
    if (event.altKey) modifiers.push("Alt");
    if (event.shiftKey) modifiers.push("Shift");
    if (event.metaKey) modifiers.push("Cmd");

    if (key !== "CONTROL" && key !== "ALT" && key !== "SHIFT" && key !== "META") {
      globalShortcut = [...modifiers, key].join("+");
      isCapturingShortcut = false;
    }
  }
  async function saveSettings() {
    if (!dataFolder || !globalShortcut) {
      error = "Please fill in both fields";
      return;
    }

    try {
      await invoke("save_initial_settings", {
        settings: {
          data_folder: dataFolder,
          global_shortcut: globalShortcut,
        },
      });
      window.location.reload();
    } catch (e) {
      error = "Failed to save settings";
      console.error(e);
    }finally {
        // set initial setup complete to true
        isInitialSetupComplete = true;
    }
  }

  onMount(() => {
    document.addEventListener("keydown", handleKeyDown);
    return () => {
      document.removeEventListener("keydown", handleKeyDown);
    };
  });
</script>

<div class="startup-screen section">
  <div class="container">
    <h1 class="title is-2">Welcome to Terraphim AI</h1>
    <p class="subtitle">Please set up your initial settings:</p>
    <div class="field">
      <label class="label" for="data-folder">Data Folder Path:</label>
      <div class="control">
        <input class="input" id="data-folder" type="file" webkitdirectory="true" directory bind:value={dataFolder}/>
        <!-- <input class="input" id="data-folder" type="file" bind:value={dataFolder} readonly /> -->
      </div>
    </div>
    <div class="field">
      <div class="control">
        <button class="button is-primary" on:click={selectFolder}>Browse</button>
      </div>
    </div>
    <div class="field">
      <label class="label" for="global-shortcut">Global Shortcut:</label>
      <div class="control">
        <input 
          class="input" 
          id="global-shortcut" 
          type="text" 
          bind:value={globalShortcut} 
          readonly 
          placeholder="Click to set shortcut"
          on:click={startCapturingShortcut}
        />
      </div>
    </div>
    {#if error}
      <p class="help is-danger">{error}</p>
    {/if}
    <div class="field">
      <div class="control">
        <button class="button is-success" on:click={saveSettings}>Save Settings</button>
      </div>
    </div>
  </div>
</div>

<style>
  .startup-screen {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100vh;
  }

  .container {
    max-width: 500px;
  }
</style>