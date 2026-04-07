// Loading wasm directly into browser

// var importObject = {
//     imports: {
//         imported_func: function (arg) {
//             console.log(arg);
//         }
//     }
// };

// var response = null;
// var bytes = null;
// var results = null;

// var wasmPath = chrome.runtime.getURL("terrraphim_automata_wasm_bg.wasm");
// console.log("myPath: " + wasmPath);

// fetch(wasmPath).then(response =>
//     response.arrayBuffer()
// ).then(bytes =>
//     WebAssembly.instantiate(bytes, importObject)
// ).then(results => {
//     results.instance.exports.exported_func();
// });
//  for loading into browser add
//     "web_accessible_resources": [
// {
//     "resources": [
//         "terrraphim_automata_wasm_bg.wasm"
//     ],
//         "matches": [
//             "<all_urls>"
//         ]
// }
//     ],
// into manifest file
