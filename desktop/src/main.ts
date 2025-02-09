import App from "./App.svelte";
import { initStore } from "@tomic/svelte";
import { Store, Agent, properties, importJsonAdString } from "@tomic/lib";
import { initTauri } from "./lib/tauri";
import { is_tauri } from "./lib/stores";
// const agent = Agent.fromSecret("");
// This is where you configure your atomic data store.
// const store = new Store({
//   serverUrl: 'http://localhost:9883',
//   agent,
// });

// Initialize store
const store = new Store();
initStore(store);

// Initialize Tauri if available
async function init() {
  const isTauriAvailable = await initTauri();
  is_tauri.set(isTauriAvailable);
  
  // Initialize app
  const app = new App({
    target: document.getElementById("app"),
  });
  
  return app;
}

export default init();
