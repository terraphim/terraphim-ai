// src/helpers/getStore.ts
import { Store,Agent } from '@tomic/lib';
import { initOntologies } from '../ontologies';

let store: Store;

export function getStore(agent?: string): Store {
  if (!store) {
    if (!agent) {
    store = new Store({
      serverUrl: "https://common.terraphim.io/drive/h6grD0ID",
    });
  }else{
    store = new Store({
      serverUrl: "https://common.terraphim.io/drive/h6grD0ID",
      agent: Agent.fromSecret(agent),
    });
  }

    initOntologies();
  }

  return store;
}