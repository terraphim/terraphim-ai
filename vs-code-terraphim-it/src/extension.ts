import * as vscode from 'vscode';
import { CollectionBuilder, core } from '@tomic/lib';
import { getStore } from './helpers/getStore';

import {
  airportOntology,
  type SystemOperatorAnalyticalLens,
} from './ontologies/airportOntology';
// --------- Create a Store ---------.
const store = getStore();

async function get_all_resources(): Promise<{ [key: string]: string }> {
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

export function activate(context: vscode.ExtensionContext) {
  const disposable = vscode.commands.registerCommand(
    'extension.terraphimCommand',
    async function () {
      // Get the active text editor
      const editor = vscode.window.activeTextEditor;

      if (editor) {
        const document = editor.document;
        // const selection = editor.selection;

        // Get the word within the selection
        let text = document.getText();
        const results = await get_all_resources();
        Object.keys(results).forEach(key => {
          text = text.replace(new RegExp(key, 'g'), `[${key}](${results[key]})`);
        });
        editor.edit(editBuilder => {
          editBuilder.replace(
            new vscode.Range(0, 0, editor.document.lineCount, 0),
            text,
          );
        });
      }
    },
  );

  context.subscriptions.push(disposable);
}
