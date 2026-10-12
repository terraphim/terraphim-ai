import * as vscode from 'vscode';
import { registerAlternativeControl } from './alternativeControl';
import { CollectionBuilder, core, Store } from '@tomic/lib';
import { getStore } from './helpers/getStore';
import { serverConnection } from './helpers/serverConnection';
import { replaceLinks } from './helpers/linkReplacement';
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
  registerAlternativeControl(context);
  let agent: string | undefined;
  let store: Store | undefined;
  let activeProvider: vscode.Disposable | undefined;

  const disposable = vscode.commands.registerCommand(
    'extension.terraphimCommand',
    async function () {
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
      try {
        store = getStore(agent);
        const results = await get_all_resources(store);
        const replacedText = await replaceLinks(text, JSON.stringify(results));
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
      } catch (error) {
        vscode.window.showErrorMessage(`Terraphim IT could not replace links: ${error instanceof Error ? error.message : String(error)}`);
        return false;
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

  // Preserve the legacy command ID while using the packaged server client.
  // A missing optional NAPI module must never masquerade as active autocomplete.
  const activateServerAutocomplete = async () => {
    const healthCheck = await serverConnection.healthCheck();
    if (!healthCheck.healthy) {
      vscode.window.showWarningMessage(`Terraphim server is not available: ${healthCheck.message}. Please check your configuration and server status.`);
      return false;
    }
    activeProvider?.dispose();
    const provider = vscode.languages.registerCompletionItemProvider(
      { scheme: 'file', language: '*' },
      new TerraphimServerCompletionProvider(),
      ' '
    );
    activeProvider = provider;
    context.subscriptions.push(provider);
    vscode.window.showInformationMessage('Terraphim Server Autocomplete activated');
    return true;
  };
  context.subscriptions.push(vscode.commands.registerCommand(
    'extension.terraphimNapiAutocomplete', activateServerAutocomplete
  ));
  context.subscriptions.push(vscode.commands.registerCommand(
    'extension.terraphimServerAutocomplete', activateServerAutocomplete
  ));

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
