"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.activate = activate;
const vscode = require("vscode");
const lib_1 = require("@tomic/lib");
const getStore_1 = require("./helpers/getStore");
const airportOntology_1 = require("./airport-ontology/src/ontologies/airportOntology");
// --------- Create a Store ---------.
const store = (0, getStore_1.getStore)();
async function get_all_resources() {
    // search over all atomic server resources
    const itemCollection = new lib_1.CollectionBuilder(store)
        .setProperty(lib_1.core.properties.isA)
        .setValue(airportOntology_1.airportOntology.classes.systemOperatorAnalyticalLens)
        .setSortBy(airportOntology_1.airportOntology.properties.synonym)
        .setSortDesc(true)
        .build();
    const results = [];
    for await (const inst of itemCollection) {
        const item = await store.getResource(inst);
        results.push({
            id: item.subject,
            title: item.title,
            description: item.subject,
        });
    }
    console.log(results);
    return results;
}
function activate(context) {
    const disposable = vscode.commands.registerCommand('extension.terraphimCommand', async function () {
        // Get the active text editor
        const editor = vscode.window.activeTextEditor;
        if (editor) {
            const document = editor.document;
            // const selection = editor.selection;
            // Get the word within the selection
            const word = document.getText();
            const reversed = word.split('').reverse().join('');
            const results = await get_all_resources();
            editor.edit(editBuilder => {
                editBuilder.replace(new vscode.Range(0, 0, editor.document.lineCount, 0), reversed);
            });
            editor.insertSnippet(new vscode.SnippetString(results[0].description));
        }
    });
    context.subscriptions.push(disposable);
}
//# sourceMappingURL=extension.js.map