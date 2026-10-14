// src/helpers/getStore.ts
import { Store, Agent } from '@tomic/lib';
import * as vscode from 'vscode';
import { initOntologies } from '../ontologies';
let store: Store | undefined;
let storeServerUrl: string | undefined;
let storeAgent: string | undefined;

export function getStore(agent?: string): Store {
  const config = vscode.workspace.getConfiguration('terraphimIt');
  const atomicServerUrl = config.get<string>('atomicServerUrl') || 'https://common.terraphim.io/drive/h6grD0ID';
  const selectedAgent = agent ?? config.get<string>('agent') ?? '';
  if (!store || storeServerUrl !== atomicServerUrl || storeAgent !== selectedAgent) {
    // Construct before replacing the cache: malformed credentials must not
    // silently select an anonymous agent or retain a partially updated store.
    const nextStore = new Store({
      serverUrl: atomicServerUrl,
      ...(selectedAgent ? { agent: Agent.fromSecret(selectedAgent) } : {}),
    });
    store = nextStore;
    storeServerUrl = atomicServerUrl;
    storeAgent = selectedAgent;

    initOntologies();
    console.log(`Initialized Atomic Store with server: ${atomicServerUrl}`);
  }

  return store;
}

/**
 * Reset the store instance - useful when configuration changes
 */
export function resetStore(): void {
  store = undefined;
  storeServerUrl = undefined;
  storeAgent = undefined;
  console.log('Store instance reset');
}

/**
 * Get store with updated configuration
 */
export function getUpdatedStore(agent?: string): Store {
  // Configuration is read when constructing the store.
  resetStore();
  return getStore(agent);
}
