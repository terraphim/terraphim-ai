'use strict';

import * as vscode from 'vscode';

export function activate(context: vscode.ExtensionContext) {
	const disposable = vscode.commands.registerCommand('extension.terraphimIt', function() {
		// Get the active text editor
		const editor = vscode.window.activeTextEditor;

		if (editor) {
			const document = editor.document;
			// const selection = editor.selection;

			// Get the word within the selection
			const word = document.getText();
			const reversed = word.split('').reverse().join('');
			editor.edit(editBuilder => {
				editBuilder.replace(new vscode.Range(0, 0, editor.document.lineCount, 0), reversed);
			});
		}
	});

	context.subscriptions.push(disposable);
}