import * as vscode from 'vscode';
import { CollectionBuilder, core, Store } from '@tomic/lib';
import { getStore, getUpdatedStore } from './helpers/getStore';
import { serverConnection } from './helpers/serverConnection';
import { replace_links } from '../rust-lib/pkg';
import { searchDocumentsSelectedRole } from 'terraphim_ai_nodejs/index.js';
import {
  airportOntology,
  type SystemOperatorAnalyticalLens,
} from './ontologies/airportOntology';

interface EngineeringData {
  name: string;
  data: {
    [key: string]: {
      id: number;
      nterm: string;
      url: string;
      description: string;
    };
  };
}

export function activate(context: vscode.ExtensionContext) {
  let agent: string | undefined;
  let store: Store | undefined;
  let activeProvider: vscode.Disposable | undefined;

  const disposable = vscode.commands.registerCommand(
    'extension.terraphimCommand',
    async function () {
      // Get the configuration
      const config = vscode.workspace.getConfiguration('terraphimIt');

      // Only ask for agent if it's not set and hasn't been asked before
      if (!agent) {
        agent = config.get<string>('agent');
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

      store = getStore(agent);

      const editor = vscode.window.activeTextEditor;

      if (editor) {
        const document = editor.document;
        // const selection = editor.selection;

        // Get the word within the selection
        const text = document.getText();
        const results = await get_all_resources(store);
        const thesaurus = JSON.stringify(results);
        const replacedText = await replace_links(text, thesaurus);
        editor.edit(editBuilder => {
          editBuilder.replace(
            new vscode.Range(0, 0, editor.document.lineCount, 0),
            replacedText.toString(),
          );
        });

        // Show information message with agent if it's set
        if (agent) {
          vscode.window.showInformationMessage(`Terraphim IT executed with agent starting with: ${agent.substring(0, 5)}`);
        } else {
          vscode.window.showInformationMessage('Terraphim IT executed');
        }
      }
    },
  );

  context.subscriptions.push(disposable);
  // Register the Terraphim AI Autocomplete command
  const autocompleteDisposable = vscode.commands.registerCommand(
    'extension.terraphimAIAutocomplete',
    async function () {
      if (!store) {
        store = getStore(agent);
      }
      // Deactivate the existing provider if any
      if (activeProvider) {
        activeProvider.dispose();
        activeProvider = undefined;
      }
      // Register the completion provider
      const provider = vscode.languages.registerCompletionItemProvider(
        { scheme: 'file', language: '*' },
        new TerraphimCompletionProvider(store),
        ' ' // Trigger on space
      );
      activeProvider = provider;
      context.subscriptions.push(provider);
      vscode.window.showInformationMessage('Terraphim AI Autocomplete activated');
    }
  );

  context.subscriptions.push(autocompleteDisposable);

  const napiAutocompleteDisposable = vscode.commands.registerCommand(
    'extension.terraphimNapiAutocomplete',
    async function () {
      // Deactivate the existing provider if any
      if (activeProvider) {
        activeProvider.dispose();
        activeProvider = undefined;
      }

      // Register the completion provider
      const provider = vscode.languages.registerCompletionItemProvider(
        { scheme: 'file', language: '*' },
        new TerraphimNapiCompletionProvider(),
        ' ' // Trigger on space
      );

      activeProvider = provider;
      context.subscriptions.push(provider);
      vscode.window.showInformationMessage('Terraphim Napi Autocomplete activated');
    }
  );

  context.subscriptions.push(napiAutocompleteDisposable);

  // Register the Terraphim Server Autocomplete command
  const serverAutocompleteDisposable = vscode.commands.registerCommand(
    'extension.terraphimServerAutocomplete',
    async function () {
      // Deactivate the existing provider if any
      if (activeProvider) {
        activeProvider.dispose();
        activeProvider = undefined;
      }

      // Check server health first
      const healthCheck = await serverConnection.healthCheck();
      if (!healthCheck.healthy) {
        vscode.window.showWarningMessage(`Terraphim server is not available: ${healthCheck.message}. Please check your configuration and server status.`);
        return;
      }

      // Register the completion provider
      const provider = vscode.languages.registerCompletionItemProvider(
        { scheme: 'file', language: '*' },
        new TerraphimServerCompletionProvider(),
        ' ' // Trigger on space
      );

      activeProvider = provider;
      context.subscriptions.push(provider);
      vscode.window.showInformationMessage('Terraphim Server Autocomplete activated');
    }
  );

  context.subscriptions.push(serverAutocompleteDisposable);

  // Register the Server Health Check command
  const healthCheckDisposable = vscode.commands.registerCommand(
    'extension.terraphimHealthCheck',
    async function () {
      vscode.window.showInformationMessage('Checking server health...');

      const testResults = await serverConnection.testConnection();

      const terraphimStatus = testResults.terraphim.healthy ? '✅' : '❌';
      const atomicStatus = testResults.atomic.healthy ? '✅' : '❌';

      const message = `Server Status:\n${terraphimStatus} Terraphim Server: ${testResults.terraphim.message}\n${atomicStatus} Atomic Server: ${testResults.atomic.message}`;

      if (testResults.terraphim.healthy && testResults.atomic.healthy) {
        vscode.window.showInformationMessage(message);
      } else {
        vscode.window.showWarningMessage(message);
      }
    }
  );

  context.subscriptions.push(healthCheckDisposable);

  // Listen for configuration changes
  const configChangeDisposable = vscode.workspace.onDidChangeConfiguration(event => {
    if (event.affectsConfiguration('terraphimIt')) {
      serverConnection.updateConfiguration();
      vscode.window.showInformationMessage('Terraphim IT configuration updated');
    }
  });

  context.subscriptions.push(configChangeDisposable);

}

class TerraphimNapiCompletionProvider implements vscode.CompletionItemProvider {
  private lastQuery: string = '';
  private lastResults: vscode.CompletionItem[] = [];
  private updateSubscription: vscode.Disposable | null = null;

  constructor() {
    this.subscribeForUpdates();
  }

  private subscribeForUpdates() {
    if (this.updateSubscription) {
      this.updateSubscription.dispose();
    }
    this.updateSubscription = vscode.workspace.onDidChangeTextDocument(this.onDocumentChange.bind(this));
  }

  private onDocumentChange(event: vscode.TextDocumentChangeEvent) {
    const activeEditor = vscode.window.activeTextEditor;
    if (activeEditor && event.document === activeEditor.document) {
      this.updateCompletions(activeEditor.document, activeEditor.selection.active);
    }
  }

  private async updateCompletions(document: vscode.TextDocument, position: vscode.Position) {
    const wordRange = document.getWordRangeAtPosition(position) || document.getWordRangeAtPosition(position.translate(0, -1));
    if (wordRange) {
      const word = document.getText(wordRange);
      console.log("word", word);
      if (word !== this.lastQuery && word.length >= 2) { // Only search if word has at least 2 characters
        this.lastQuery = word;

        // Try server-based search first, fallback to local nodejs binding
        try {
          const searchResponse = await serverConnection.searchDocuments(word, 10);
          if (searchResponse.status === 'success' && searchResponse.results.length > 0) {
            this.lastResults = this.createCompletionItemsFromServer(searchResponse.results);
          } else {
            // Fallback to local nodejs binding
            const results = await searchDocumentsSelectedRole(word);
            const parsedResults = JSON.parse(results);
            this.lastResults = this.createCompletionItems(parsedResults);
          }
        } catch (error) {
          console.warn('Server search failed, using local nodejs binding:', error);
          // Fallback to local nodejs binding
          try {
            const results = await searchDocumentsSelectedRole(word);
            const parsedResults = JSON.parse(results);
            this.lastResults = this.createCompletionItems(parsedResults);
          } catch (localError) {
            console.error('Both server and local search failed:', localError);
            this.lastResults = [];
          }
        }
      }
    }
  }

  private createCompletionItems(parsedResults: any): vscode.CompletionItem[] {
    return Object.entries(parsedResults).map(([key, value]: [string, any]) => {
      const item = new vscode.CompletionItem(value.title, vscode.CompletionItemKind.Text);
      item.detail = value.body as string;
      item.documentation = new vscode.MarkdownString(`[${value.title}](${value.url})`);
      return item;
    });
  }

  private createCompletionItemsFromServer(results: any[]): vscode.CompletionItem[] {
    return results.map((result: any) => {
      const item = new vscode.CompletionItem(result.title, vscode.CompletionItemKind.Text);
      item.detail = result.description || result.body || '';
      item.documentation = new vscode.MarkdownString(`[${result.title}](${result.url})`);
      if (result.rank) {
        item.sortText = result.rank.toString().padStart(3, '0');
      }
      return item;
    });
  }

  async provideCompletionItems(
    document: vscode.TextDocument,
    position: vscode.Position,
    token: vscode.CancellationToken,
    context: vscode.CompletionContext
  ): Promise<vscode.CompletionItem[] | vscode.CompletionList> {
    await this.updateCompletions(document, position);
    return this.lastResults;
  }

  dispose() {
    if (this.updateSubscription) {
      this.updateSubscription.dispose();
    }
  }
}

class TerraphimCompletionProvider implements vscode.CompletionItemProvider {
  private store: Store;

  constructor(store: Store) {
    this.store = store;
  }

  async provideCompletionItems(
    document: vscode.TextDocument,
    position: vscode.Position,
    token: vscode.CancellationToken,
    context: vscode.CompletionContext
  ): Promise<vscode.CompletionItem[] | vscode.CompletionList> {
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

class TerraphimServerCompletionProvider implements vscode.CompletionItemProvider {
  private lastQuery: string = '';
  private lastResults: vscode.CompletionItem[] = [];

  async provideCompletionItems(
    document: vscode.TextDocument,
    position: vscode.Position,
    token: vscode.CancellationToken,
    context: vscode.CompletionContext
  ): Promise<vscode.CompletionItem[] | vscode.CompletionList> {
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
        const autocompleteResponse = await serverConnection.getAutocompleteSuggestions(word, 10);
        if (autocompleteResponse.status === 'success' && autocompleteResponse.suggestions.length > 0) {
          this.lastResults = autocompleteResponse.suggestions.map(suggestion => {
            const item = new vscode.CompletionItem(suggestion.term, vscode.CompletionItemKind.Text);
            item.sortText = suggestion.score.toString().padStart(8, '0');
            return item;
          });
        } else {
          // Fallback to search endpoint
          const searchResponse = await serverConnection.searchDocuments(word, 10);
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
          } else {
            this.lastResults = [];
          }
        }
      } catch (error) {
        console.error('Server completion failed:', error);
        this.lastResults = [];
      }
    }

    return this.lastResults;
  }
}



async function get_all_resources(store: Store): Promise<EngineeringData> {
  // search over all atomic server resources
  const itemCollection = new CollectionBuilder(store)
    .setProperty(core.properties.isA)
    .setValue(airportOntology.classes.systemOperatorAnalyticalLens)
    .setSortBy(airportOntology.properties.synonym)
    .setSortDesc(true)
    .build();



  const results: EngineeringData  = {name: "Engineering", data: {}};
  let counter = 1;
  for await (const inst of itemCollection) {
    const item = await store.getResource<SystemOperatorAnalyticalLens>(inst);
    // console.log(item);
    if (item.props.synonym) {
      // split the synonym by comma and add each synonym as a key
      item.props.synonym.split(',').forEach(synonym => {
        results.data[synonym] = {id: counter, nterm: item.props.name, url: item.subject, description: item.props.description};
        counter++;
      });
    }
    results.data[item.props.name] = {id: counter, nterm: item.props.name, url: item.subject, description: item.props.description};
    counter++;
  }
  // console.log(results);
  return results;
}
