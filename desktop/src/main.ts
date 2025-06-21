import App from "./App.svelte";
import { initStore } from "@tomic/svelte";
// Novel's compiled CSS lives in the package's dist directory.
import './styles/novel.css';
import { Store, Agent, properties, importJsonAdString } from "@tomic/lib";
// const agent = Agent.fromSecret("");
// This is where you configure your atomic data store.
// const store = new Store({
//   serverUrl: 'http://localhost:9883',
//   agent,
// });
const store = new Store();
initStore(store);
const app = new App({
  target: document.getElementById("app"),
});

export default app;
