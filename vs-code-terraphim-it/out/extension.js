"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.activate = activate;
const vscode = require("vscode");
const alternativeControl_1 = require("./alternativeControl");
const lib_1 = require("@tomic/lib");
const getStore_1 = require("./helpers/getStore");
const serverConnection_1 = require("./helpers/serverConnection");
const linkReplacement_1 = require("./helpers/linkReplacement");
const completionCache_1 = require("./helpers/completionCache");
const airportOntology_1 = require("./ontologies/airportOntology");
function activate(context) {
    (0, alternativeControl_1.registerAlternativeControl)(context);
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
        let agent = config.get('agent');
        // Read current settings on every command, including changes since activation.
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
        try {
            const store = (0, getStore_1.getStore)(agent);
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
        try {
            (0, getStore_1.getStore)(); // Validate current credentials before claiming activation.
        }
        catch (error) {
            vscode.window.showErrorMessage(`Cannot enable Atomic autocomplete: ${error instanceof Error ? error.message : 'invalid configuration'}`);
            return false;
        }
        // Deactivate the existing provider if any
        if (activeProvider) {
            activeProvider.dispose();
            activeProvider = undefined;
        }
        // Register the completion provider
        const provider = vscode.languages.registerCompletionItemProvider({ scheme: 'file', language: '*' }, new TerraphimCompletionProvider(), ' ' // Trigger on space
        );
        activeProvider = provider;
        context.subscriptions.push(provider);
        vscode.window.showInformationMessage('Terraphim AI Autocomplete activated');
        return true;
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
            if (event.affectsConfiguration('terraphimIt.serverUrl') || event.affectsConfiguration('terraphimIt.enableLocalServer')) {
                try {
                    serverConnection_1.serverConnection.getServerUrl();
                }
                catch (error) {
                    vscode.window.showWarningMessage(`Terraphim server configuration: ${error instanceof Error ? error.message : 'invalid server URL'}`);
                    return;
                }
            }
            vscode.window.showInformationMessage('Terraphim IT configuration updated');
        }
    });
    context.subscriptions.push(configChangeDisposable);
}
class TerraphimCompletionProvider {
    async provideCompletionItems(document, position, token, context) {
        const linePrefix = document.lineAt(position).text.substr(0, position.character);
        if (!linePrefix.endsWith(' ')) {
            return [];
        }
        try {
            // Check identity after loading too: settings may change during the request.
            const store = (0, getStore_1.getStore)();
            const results = await get_all_resources(store);
            if ((0, getStore_1.getStore)() !== store)
                return [];
            return Object.entries(results.data).map(([key, value]) => {
                const item = new vscode.CompletionItem(key, vscode.CompletionItemKind.Text);
                item.detail = value.description;
                item.documentation = new vscode.MarkdownString(`[${value.nterm}](${value.url})`);
                return item;
            });
        }
        catch (error) {
            vscode.window.showWarningMessage(`Atomic autocomplete could not load suggestions: ${error instanceof Error ? error.message : 'check endpoint and agent settings'}`);
            return [];
        }
    }
}
class TerraphimServerCompletionProvider {
    constructor() {
        this.cache = new completionCache_1.CompletionCache();
    }
    async provideCompletionItems(document, position, token, context) {
        const revision = serverConnection_1.serverConnection.getConfigurationRevision();
        const wordRange = document.getWordRangeAtPosition(position) || (position.character > 0 ? document.getWordRangeAtPosition(position.translate(0, -1)) : undefined);
        if (!wordRange) {
            return [];
        }
        const word = document.getText(wordRange);
        if (word.length < 2) {
            return [];
        }
        const { request, cached } = this.cache.begin(word, revision);
        if (cached)
            return cached;
        let results = [];
        try {
            // Try autocomplete endpoint first
            const autocompleteResponse = await serverConnection_1.serverConnection.getAutocompleteSuggestions(word, 10);
            // Do not send the old query to a newly selected endpoint in the fallback.
            if (!this.cache.isCurrent(request, serverConnection_1.serverConnection.getConfigurationRevision()))
                return [];
            if (autocompleteResponse.status === 'success' && autocompleteResponse.suggestions.length > 0) {
                results = autocompleteResponse.suggestions.map(suggestion => {
                    const item = new vscode.CompletionItem(suggestion.term, vscode.CompletionItemKind.Text);
                    item.sortText = suggestion.score.toString().padStart(8, '0');
                    return item;
                });
            }
            else {
                // Fallback to search endpoint
                const searchResponse = await serverConnection_1.serverConnection.searchDocuments(word, 10);
                if (searchResponse.status === 'success' && searchResponse.results.length > 0) {
                    results = searchResponse.results.map(result => {
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
                    results = [];
                }
            }
        }
        catch (error) {
            console.error('Server completion failed:', error);
            results = [];
        }
        return this.cache.commit(request, serverConnection_1.serverConnection.getConfigurationRevision(), results);
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