"use strict";
/* -----------------------------------
* GENERATED WITH @tomic/cli
* -------------------------------- */
Object.defineProperty(exports, "__esModule", { value: true });
exports.initOntologies = initOntologies;
const lib_1 = require("@tomic/lib");
const airportOntology_js_1 = require("./airportOntology.js");
function initOntologies() {
    (0, lib_1.registerOntologies)(airportOntology_js_1.airportOntology);
}
//# sourceMappingURL=index.js.map