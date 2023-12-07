import App from './App.svelte'
import { initStore } from '@tomic/svelte';
  import { Store, Agent, properties, importJsonAdString} from '@tomic/lib';
    // const agent = Agent.fromSecret("");
    // This is where you configure your atomic data store.
    // const store = new Store({
    //   serverUrl: 'http://localhost:9883',
    //   agent,
    // });
    const store = new Store();
    initStore(store);
const app = new App({
  target: document.getElementById('app')
})

export default app
