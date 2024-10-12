import * as vscode from 'vscode';
import { CollectionBuilder, core, Store } from '@tomic/lib';
import { getStore } from './helpers/getStore';

import {
  airportOntology,
  type SystemOperatorAnalyticalLens,
} from './ontologies/airportOntology';



export function activate(context: vscode.ExtensionContext) {
  let agent: string | undefined;

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
        // --------- Create a Store ---------.
    const store = getStore(agent);

      // Get the active text editor
      const editor = vscode.window.activeTextEditor;

      if (editor) {
        const document = editor.document;
        // const selection = editor.selection;

        // Get the word within the selection
        let text = document.getText();
        const results = await get_all_resources(store);
        Object.keys(results).forEach(key => {
          text = text.replace(new RegExp(key, 'g'), `[${key}](${results[key]})`);
        });
        editor.edit(editBuilder => {
          editBuilder.replace(
            new vscode.Range(0, 0, editor.document.lineCount, 0),
            text,
          );
        });

        // Show information message with agent if it's set
        if (agent) {
          vscode.window.showInformationMessage(`Terraphim IT executed with agent: ${agent}`);
        } else {
          vscode.window.showInformationMessage('Terraphim IT executed');
        }
      }
    },
  );

  context.subscriptions.push(disposable);
}

async function get_all_resources(store: Store): Promise<{ [key: string]: string }> {
  // search over all atomic server resources
  const itemCollection = new CollectionBuilder(store)
    .setProperty(core.properties.isA)
    .setValue(airportOntology.classes.systemOperatorAnalyticalLens)
    .setSortBy(airportOntology.properties.synonym)
    .setSortDesc(true)
    .build();

  const results: { [key: string]: string } = {};

  for await (const inst of itemCollection) {
    const item = await store.getResource<SystemOperatorAnalyticalLens>(inst);
    console.log(item);
    if (item.props.synonym) {
      // split the synonym by comma and add each synonym as a key
      item.props.synonym.split(',').forEach(synonym => {
        results[synonym] = item.subject;
      });
    }
    results[item.title] = item.subject;
  }
  return results;
}