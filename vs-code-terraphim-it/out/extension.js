"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.activate = activate;
const vscode = require("vscode");
const lib_1 = require("@tomic/lib");
const getStore_1 = require("./helpers/getStore");
const serverConnection_1 = require("./helpers/serverConnection");
const pkg_1 = require("../rust-lib/pkg");
const index_js_1 = require("terraphim_ai_nodejs/index.js");
const airportOntology_1 = require("./ontologies/airportOntology");
function activate(context) {
    let agent;
    let store;
    let activeProvider;
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
        store = (0, getStore_1.getStore)(agent);
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
    const napiAutocompleteDisposable = vscode.commands.registerCommand('extension.terraphimNapiAutocomplete', async function () {
        // Deactivate the existing provider if any
        if (activeProvider) {
            activeProvider.dispose();
            activeProvider = undefined;
        }
        // Register the completion provider
        const provider = vscode.languages.registerCompletionItemProvider({ scheme: 'file', language: '*' }, new TerraphimNapiCompletionProvider(), ' ' // Trigger on space
        );
        activeProvider = provider;
        context.subscriptions.push(provider);
        vscode.window.showInformationMessage('Terraphim Napi Autocomplete activated');
    });
    context.subscriptions.push(napiAutocompleteDisposable);
    // Register the Terraphim Server Autocomplete command
    const serverAutocompleteDisposable = vscode.commands.registerCommand('extension.terraphimServerAutocomplete', async function () {
        // Deactivate the existing provider if any
        if (activeProvider) {
            activeProvider.dispose();
            activeProvider = undefined;
        }
        // Check server health first
        const healthCheck = await serverConnection_1.serverConnection.healthCheck();
        if (!healthCheck.healthy) {
            vscode.window.showWarningMessage(`Terraphim server is not available: ${healthCheck.message}. Please check your configuration and server status.`);
            return;
        }
        // Register the completion provider
        const provider = vscode.languages.registerCompletionItemProvider({ scheme: 'file', language: '*' }, new TerraphimServerCompletionProvider(), ' ' // Trigger on space
        );
        activeProvider = provider;
        context.subscriptions.push(provider);
        vscode.window.showInformationMessage('Terraphim Server Autocomplete activated');
    });
    context.subscriptions.push(serverAutocompleteDisposable);
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
class TerraphimNapiCompletionProvider {
    constructor() {
        this.lastQuery = '';
        this.lastResults = [];
        this.updateSubscription = null;
        this.subscribeForUpdates();
    }
    subscribeForUpdates() {
        if (this.updateSubscription) {
            this.updateSubscription.dispose();
        }
        this.updateSubscription = vscode.workspace.onDidChangeTextDocument(this.onDocumentChange.bind(this));
    }
    onDocumentChange(event) {
        const activeEditor = vscode.window.activeTextEditor;
        if (activeEditor && event.document === activeEditor.document) {
            this.updateCompletions(activeEditor.document, activeEditor.selection.active);
        }
    }
    async updateCompletions(document, position) {
        const wordRange = document.getWordRangeAtPosition(position) || document.getWordRangeAtPosition(position.translate(0, -1));
        if (wordRange) {
            const word = document.getText(wordRange);
            console.log("word", word);
            if (word !== this.lastQuery && word.length >= 2) { // Only search if word has at least 2 characters
                this.lastQuery = word;
                // Try server-based search first, fallback to local nodejs binding
                try {
                    const searchResponse = await serverConnection_1.serverConnection.searchDocuments(word, 10);
                    if (searchResponse.status === 'success' && searchResponse.results.length > 0) {
                        this.lastResults = this.createCompletionItemsFromServer(searchResponse.results);
                    }
                    else {
                        // Fallback to local nodejs binding
                        const results = await (0, index_js_1.searchDocumentsSelectedRole)(word);
                        const parsedResults = JSON.parse(results);
                        this.lastResults = this.createCompletionItems(parsedResults);
                    }
                }
                catch (error) {
                    console.warn('Server search failed, using local nodejs binding:', error);
                    // Fallback to local nodejs binding
                    try {
                        const results = await (0, index_js_1.searchDocumentsSelectedRole)(word);
                        const parsedResults = JSON.parse(results);
                        this.lastResults = this.createCompletionItems(parsedResults);
                    }
                    catch (localError) {
                        console.error('Both server and local search failed:', localError);
                        this.lastResults = [];
                    }
                }
            }
        }
    }
    createCompletionItems(parsedResults) {
        return Object.entries(parsedResults).map(([key, value]) => {
            const item = new vscode.CompletionItem(value.title, vscode.CompletionItemKind.Text);
            item.detail = value.body;
            item.documentation = new vscode.MarkdownString(`[${value.title}](${value.url})`);
            return item;
        });
    }
    createCompletionItemsFromServer(results) {
        return results.map((result) => {
            const item = new vscode.CompletionItem(result.title, vscode.CompletionItemKind.Text);
            item.detail = result.description || result.body || '';
            item.documentation = new vscode.MarkdownString(`[${result.title}](${result.url})`);
            if (result.rank) {
                item.sortText = result.rank.toString().padStart(3, '0');
            }
            return item;
        });
    }
    async provideCompletionItems(document, position, token, context) {
        await this.updateCompletions(document, position);
        return this.lastResults;
    }
    dispose() {
        if (this.updateSubscription) {
            this.updateSubscription.dispose();
        }
    }
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
        const wordRange = document.getWordRangeAtPosition(position) || document.getWordRangeAtPosition(position.translate(0, -1));
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
