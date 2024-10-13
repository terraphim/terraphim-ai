import * as vscode from 'vscode';
import { CollectionBuilder, core, Store } from '@tomic/lib';
import { getStore } from './helpers/getStore';
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

      // Register the completion provider
      const provider = vscode.languages.registerCompletionItemProvider(
        { scheme: 'file', language: '*' },
        new TerraphimCompletionProvider(store),
        ' ' // Trigger on space
      );

      context.subscriptions.push(provider);
      vscode.window.showInformationMessage('Terraphim AI Autocomplete activated');
    }
  );

  context.subscriptions.push(autocompleteDisposable);

  const napiAutocompleteDisposable = vscode.commands.registerCommand(
    'extension.terraphimNapiAutocomplete',
    async function () {


      // Register the completion provider
      const provider = vscode.languages.registerCompletionItemProvider(
        { scheme: 'file', language: '*' },
        new TerraphimNapiCompletionProvider(),
        ' ' // Trigger on space
      );

      context.subscriptions.push(provider);
      vscode.window.showInformationMessage('Terraphim Napi Autocomplete activated');
    }
  );

  context.subscriptions.push(napiAutocompleteDisposable);
  
}

class TerraphimNapiCompletionProvider implements vscode.CompletionItemProvider {
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

    const results = await searchDocumentsSelectedRole(linePrefix);
    const parsedResults = JSON.parse(results);
    console.log(parsedResults);
    return Object.entries(parsedResults).map(([key, value]) => {
      const item = new vscode.CompletionItem(value.title, vscode.CompletionItemKind.Text);
      item.detail = value.body as string;
      item.documentation = new vscode.MarkdownString(`[${value.title}](${value.url})`);
      return item;
    });
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