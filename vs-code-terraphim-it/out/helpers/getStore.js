"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.getStore = getStore;
// src/helpers/getStore.ts
const lib_1 = require("@tomic/lib");
const ontologies_1 = require("../ontologies");
let store;
function getStore(agent) {
    if (!store) {
        if (!agent) {
            store = new lib_1.Store({
                serverUrl: "https://common.terraphim.io/drive/h6grD0ID",
            });
        }
        else {
            store = new lib_1.Store({
                serverUrl: "https://common.terraphim.io/drive/h6grD0ID",
                agent: lib_1.Agent.fromSecret(agent),
            });
        }
        (0, ontologies_1.initOntologies)();
    }
    return store;
}
//# sourceMappingURL=getStore.js.map