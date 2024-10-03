
import * as vscode from 'vscode';
import { CollectionBuilder, core } from '@tomic/lib';
import { getStore } from './helpers/getStore'; 

import {airportOntology, type SystemOperatorAnalyticalLens} from './airport-ontology/src/ontologies/airportOntology';
// --------- Create a Store ---------.
const store = getStore();

async function get_all_resources(){
	// search over all atomic server resources
		const itemCollection = new CollectionBuilder(store)
		.setProperty(core.properties.isA)
		.setValue(airportOntology.classes.systemOperatorAnalyticalLens)
		.setSortBy(airportOntology.properties.synonym)
		.setSortDesc(true)
		.build();
	
        const results = [];
        for await (const inst of itemCollection) {
		const item = await store.getResource<SystemOperatorAnalyticalLens>(inst);
		results.push({
		id:item.subject,
		title:item.title,
		description:item.subject,
		});
	}
	console.log(results);
		return results;
}

export function activate(context: vscode.ExtensionContext) {
	const disposable = vscode.commands.registerCommand('extension.terraphimCommand', async function() {
		// Get the active text editor
		const editor = vscode.window.activeTextEditor;

		if (editor) {
			const document = editor.document;
			// const selection = editor.selection;

			// Get the word within the selection
			const word = document.getText();
			const reversed = word.split('').reverse().join('');
			const results=await get_all_resources();
			editor.edit(editBuilder => {
				editBuilder.replace(new vscode.Range(0, 0, editor.document.lineCount, 0), reversed);
			});
			editor.insertSnippet(new vscode.SnippetString(results[0].description));
		}
	});

	context.subscriptions.push(disposable);
}