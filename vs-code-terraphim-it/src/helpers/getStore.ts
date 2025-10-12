// src/helpers/getStore.ts
import { Store, Agent } from '@tomic/lib';
import * as vscode from 'vscode';
import { initOntologies } from '../ontologies';
import { serverConnection } from './serverConnection';

let store: Store;

export function getStore(agent?: string): Store {
  if (!store) {
    // Get the atomic server URL from configuration
    const atomicServerUrl = serverConnection.getAtomicServerUrl();

    if (!agent) {
      store = new Store({
        serverUrl: atomicServerUrl,
      });
    } else {
      store = new Store({
        serverUrl: atomicServerUrl,
        agent: Agent.fromSecret(agent),
      });
    }

    initOntologies();
    console.log(`Initialized Atomic Store with server: ${atomicServerUrl}`);
  }

  return store;
}

/**
 * Reset the store instance - useful when configuration changes
 */
export function resetStore(): void {
  store = undefined as any;
  console.log('Store instance reset');
}

/**
 * Get store with updated configuration
 */
export function getUpdatedStore(agent?: string): Store {
  // Update server connection configuration
  serverConnection.updateConfiguration();
  // Reset and recreate store with new config
  resetStore();
  return getStore(agent);
}
