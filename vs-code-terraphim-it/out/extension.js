"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.activate = activate;
const vscode = require("vscode");
const lib_1 = require("@tomic/lib");
const getStore_1 = require("./helpers/getStore");
const pkg_1 = require("../rust-lib/pkg");
const airportOntology_1 = require("./ontologies/airportOntology");
function activate(context) {
    let agent;
    const disposable = vscode.commands.registerCommand('extension.terraphimCommand', async function () {
        // Get the configuration
        const config = vscode.workspace.getConfiguration('terraphimIt');
        // Only ask for agent if it's not set and hasn't been asked before
        if (!agent) {
            agent = config.get('agent');
            if (!agent) {
                agent = await vscode.window.showInputBox({
                    prompt: 'Enter the agent to use for Terraphim IT (optional)',
                    placeHolder: 'Agent String'
                });
                // Save the agent to configuration if provided
                if (agent) {
                    await config.update('agent', agent, vscode.ConfigurationTarget.Global);
                }
            }
        }
        vscode.window.showInformationMessage("Replacing links using Rust");
        // --------- Create a Store ---------.
        const store = (0, getStore_1.getStore)(agent);
        // Get the active text editor
        const editor = vscode.window.activeTextEditor;
        if (editor) {
            const document = editor.document;
            // const selection = editor.selection;
            // Get the word within the selection
            const text = document.getText();
            const results = await get_all_resources(store);
            const thesaurus = JSON.stringify(results);
            const replacedText = await (0, pkg_1.replace_links)(text, thesaurus);
            editor.edit(editBuilder => {
                editBuilder.replace(new vscode.Range(0, 0, editor.document.lineCount, 0), replacedText.toString());
            });
            // Show information message with agent if it's set
            if (agent) {
                vscode.window.showInformationMessage(`Terraphim IT executed with agent starting with: ${agent.substring(0, 5)}`);
            }
            else {
                vscode.window.showInformationMessage('Terraphim IT executed');
            }
        }
    });
    context.subscriptions.push(disposable);
}
async function get_all_resources(store) {
    // search over all atomic server resources
    const itemCollection = new lib_1.CollectionBuilder(store)
        .setProperty(lib_1.core.properties.isA)
        .setValue(airportOntology_1.airportOntology.classes.systemOperatorAnalyticalLens)
        .setSortBy(airportOntology_1.airportOntology.properties.synonym)
        .setSortDesc(true)
        .build();
    const results = { name: "Engineering", data: {} };
    let counter = 1;
    for await (const inst of itemCollection) {
        const item = await store.getResource(inst);
        // console.log(item);
        if (item.props.synonym) {
            // split the synonym by comma and add each synonym as a key
            item.props.synonym.split(',').forEach(synonym => {
                results.data[synonym] = { id: counter, nterm: item.props.name, url: item.subject };
                counter++;
            });
        }
        results.data[item.props.name] = { id: counter, nterm: item.props.name, url: item.subject };
        counter++;
    }
    // console.log(results);
    return results;
}
//# sourceMappingURL=extension.js.map