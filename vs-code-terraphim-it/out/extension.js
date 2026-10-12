"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.activate = activate;
const vscode = require("vscode");
const alternativeControl_1 = require("./alternativeControl");
const lib_1 = require("@tomic/lib");
const getStore_1 = require("./helpers/getStore");
const serverConnection_1 = require("./helpers/serverConnection");
const linkReplacement_1 = require("./helpers/linkReplacement");
const airportOntology_1 = require("./ontologies/airportOntology");
function activate(context) {
    (0, alternativeControl_1.registerAlternativeControl)(context);
    let agent;
    let store;
    let activeProvider;
    const disposable = vscode.commands.registerCommand('extension.terraphimCommand', async function () {
        const editor = vscode.window.activeTextEditor;
        if (!editor) {
            vscode.window.showWarningMessage('Open a text document before replacing knowledge-graph links.');
            return false;
        }
        const document = editor.document;
        const version = document.version;
        const text = document.getText();
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
        try {
            store = (0, getStore_1.getStore)(agent);
            const results = await get_all_resources(store);
            const replacedText = await (0, linkReplacement_1.replaceLinks)(text, JSON.stringify(results));
            if (document.isClosed || document.version !== version || document.getText() !== text) {
                throw new Error('The document changed while knowledge-graph links were loading. Run Terraphim IT again.');
            }
            const applied = await editor.edit(editBuilder => {
                editBuilder.replace(new vscode.Range(document.positionAt(0), document.positionAt(text.length)), replacedText);
            });
            if (!applied) {
                throw new Error('VS Code did not apply the knowledge-graph link edit.');
            }
            vscode.window.showInformationMessage('Terraphim IT replaced knowledge-graph links');
            return true;
        }
        catch (error) {
            vscode.window.showErrorMessage(`Terraphim IT could not replace links: ${error instanceof Error ? error.message : String(error)}`);
            return false;
        }
    });
    context.subscriptions.push(disposable);
    // Register the Terraphim AI Autocomplete command
    const autocompleteDisposable = vscode.commands.registerCommand('extension.terraphimAIAutocomplete', async function () {
        if (!store) {
            store = (0, getStore_1.getStore)(agent);
        }
        // Deactivate the existing provider if any
        if (activeProvider) {
            activeProvider.dispose();
            activeProvider = undefined;
        }
        // Register the completion provider
        const provider = vscode.languages.registerCompletionItemProvider({ scheme: 'file', language: '*' }, new TerraphimCompletionProvider(store), ' ' // Trigger on space
        );
        activeProvider = provider;
        context.subscriptions.push(provider);
        vscode.window.showInformationMessage('Terraphim AI Autocomplete activated');
    });
    context.subscriptions.push(autocompleteDisposable);
    // Preserve the legacy command ID while using the packaged server client.
    // A missing optional NAPI module must never masquerade as active autocomplete.
    const activateServerAutocomplete = async () => {
        const healthCheck = await serverConnection_1.serverConnection.healthCheck();
        if (!healthCheck.healthy) {
            vscode.window.showWarningMessage(`Terraphim server is not available: ${healthCheck.message}. Please check your configuration and server status.`);
            return false;
        }
        activeProvider?.dispose();
        const provider = vscode.languages.registerCompletionItemProvider({ scheme: 'file', language: '*' }, new TerraphimServerCompletionProvider(), ' ');
        activeProvider = provider;
        context.subscriptions.push(provider);
        vscode.window.showInformationMessage('Terraphim Server Autocomplete activated');
        return true;
    };
    context.subscriptions.push(vscode.commands.registerCommand('extension.terraphimNapiAutocomplete', activateServerAutocomplete));
    context.subscriptions.push(vscode.commands.registerCommand('extension.terraphimServerAutocomplete', activateServerAutocomplete));
    // Register the Server Health Check command
    const healthCheckDisposable = vscode.commands.registerCommand('extension.terraphimHealthCheck', async function () {
        vscode.window.showInformationMessage('Checking server health...');
        const testResults = await serverConnection_1.serverConnection.testConnection();
        const terraphimStatus = testResults.terraphim.healthy ? '✅' : '❌';
        const atomicStatus = testResults.atomic.healthy ? '✅' : '❌';
        const message = `Server Status:\n${terraphimStatus} Terraphim Server: ${testResults.terraphim.message}\n${atomicStatus} Atomic Server: ${testResults.atomic.message}`;
        if (testResults.terraphim.healthy && testResults.atomic.healthy) {
            vscode.window.showInformationMessage(message);
        }
        else {
            vscode.window.showWarningMessage(message);
        }
    });
    context.subscriptions.push(healthCheckDisposable);
    // Listen for configuration changes
    const configChangeDisposable = vscode.workspace.onDidChangeConfiguration(event => {
        if (event.affectsConfiguration('terraphimIt')) {
            serverConnection_1.serverConnection.updateConfiguration();
            vscode.window.showInformationMessage('Terraphim IT configuration updated');
        }
    });
    context.subscriptions.push(configChangeDisposable);
}
class TerraphimCompletionProvider {
    constructor(store) {
        this.store = store;
    }
    async provideCompletionItems(document, position, token, context) {
        const linePrefix = document.lineAt(position).text.substr(0, position.character);
        if (!linePrefix.endsWith(' ')) {
            return [];
        }
        const results = await get_all_resources(this.store);
        return Object.entries(results.data).map(([key, value]) => {
            const item = new vscode.CompletionItem(key, vscode.CompletionItemKind.Text);
            item.detail = value.description;
            item.documentation = new vscode.MarkdownString(`[${value.nterm}](${value.url})`);
            return item;
        });
    }
}
class TerraphimServerCompletionProvider {
    constructor() {
        this.lastQuery = '';
        this.lastResults = [];
    }
    async provideCompletionItems(document, position, token, context) {
        const wordRange = document.getWordRangeAtPosition(position) || (position.character > 0 ? document.getWordRangeAtPosition(position.translate(0, -1)) : undefined);
        if (!wordRange) {
            return [];
        }
        const word = document.getText(wordRange);
        if (word.length < 2) {
            return [];
        }
        if (word !== this.lastQuery) {
            this.lastQuery = word;
            try {
                // Try autocomplete endpoint first
                const autocompleteResponse = await serverConnection_1.serverConnection.getAutocompleteSuggestions(word, 10);
                if (autocompleteResponse.status === 'success' && autocompleteResponse.suggestions.length > 0) {
                    this.lastResults = autocompleteResponse.suggestions.map(suggestion => {
                        const item = new vscode.CompletionItem(suggestion.term, vscode.CompletionItemKind.Text);
                        item.sortText = suggestion.score.toString().padStart(8, '0');
                        return item;
                    });
                }
                else {
                    // Fallback to search endpoint
                    const searchResponse = await serverConnection_1.serverConnection.searchDocuments(word, 10);
                    if (searchResponse.status === 'success' && searchResponse.results.length > 0) {
                        this.lastResults = searchResponse.results.map(result => {
                            const item = new vscode.CompletionItem(result.title, vscode.CompletionItemKind.Text);
                            item.detail = result.description || result.body || '';
                            item.documentation = new vscode.MarkdownString(`[${result.title}](${result.url})`);
                            if (result.rank) {
                                item.sortText = result.rank.toString().padStart(3, '0');
                            }
                            return item;
                        });
                    }
                    else {
                        this.lastResults = [];
                    }
                }
            }
            catch (error) {
                console.error('Server completion failed:', error);
                this.lastResults = [];
            }
        }
        return this.lastResults;
    }
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
                results.data[synonym] = { id: counter, nterm: item.props.name, url: item.subject, description: item.props.description };
                counter++;
            });
        }
        results.data[item.props.name] = { id: counter, nterm: item.props.name, url: item.subject, description: item.props.description };
        counter++;
    }
    // console.log(results);
    return results;
}
//# sourceMappingURL=extension.js.map