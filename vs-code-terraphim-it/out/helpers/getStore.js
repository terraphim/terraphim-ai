"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.getStore = getStore;
exports.resetStore = resetStore;
exports.getUpdatedStore = getUpdatedStore;
// src/helpers/getStore.ts
const lib_1 = require("@tomic/lib");
const ontologies_1 = require("../ontologies");
const serverConnection_1 = require("./serverConnection");
let store;
function getStore(agent) {
    if (!store) {
        // Get the atomic server URL from configuration
        const atomicServerUrl = serverConnection_1.serverConnection.getAtomicServerUrl();
        if (!agent) {
            store = new lib_1.Store({
                serverUrl: atomicServerUrl,
            });
        }
        else {
            store = new lib_1.Store({
                serverUrl: atomicServerUrl,
                agent: lib_1.Agent.fromSecret(agent),
            });
        }
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
    console.log('Store instance reset');
}
/**
 * Get store with updated configuration
 */
function getUpdatedStore(agent) {
    // Update server connection configuration
    serverConnection_1.serverConnection.updateConfiguration();
    // Reset and recreate store with new config
    resetStore();
    return getStore(agent);
}
//# sourceMappingURL=getStore.js.map
