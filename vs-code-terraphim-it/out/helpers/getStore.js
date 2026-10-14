"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.getStore = getStore;
exports.resetStore = resetStore;
exports.getUpdatedStore = getUpdatedStore;
// src/helpers/getStore.ts
const lib_1 = require("@tomic/lib");
const vscode = require("vscode");
const ontologies_1 = require("../ontologies");
let store;
let storeServerUrl;
let storeAgent;
function getStore(agent) {
    const config = vscode.workspace.getConfiguration('terraphimIt');
    const atomicServerUrl = config.get('atomicServerUrl') || 'https://common.terraphim.io/drive/h6grD0ID';
    const selectedAgent = agent ?? config.get('agent') ?? '';
    if (!store || storeServerUrl !== atomicServerUrl || storeAgent !== selectedAgent) {
        // Construct before replacing the cache: malformed credentials must not
        // silently select an anonymous agent or retain a partially updated store.
        const nextStore = new lib_1.Store({
            serverUrl: atomicServerUrl,
            ...(selectedAgent ? { agent: lib_1.Agent.fromSecret(selectedAgent) } : {}),
        });
        store = nextStore;
        storeServerUrl = atomicServerUrl;
        storeAgent = selectedAgent;
        (0, ontologies_1.initOntologies)();
        console.log(`Initialized Atomic Store with server: ${atomicServerUrl}`);
    }
    return store;
}
/**
 * Reset the store instance - useful when configuration changes
 */
function resetStore() {
    store = undefined;
    storeServerUrl = undefined;
    storeAgent = undefined;
    console.log('Store instance reset');
}
/**
 * Get store with updated configuration
 */
function getUpdatedStore(agent) {
    // Configuration is read when constructing the store.
    resetStore();
    return getStore(agent);
}
//# sourceMappingURL=getStore.js.map