/* Integration smoke in the actual installed VS Code extension host, no substitutes. */
const vscode = require('vscode');
const fs = require('node:fs/promises');
const assert = require('node:assert/strict');
exports.run = async () => {
  const report = { qualification:'SMOKE ONLY; webview interaction/all 41 unverified', checks:[] };
  try {
    const extension = vscode.extensions.getExtension('applied-knowledge-systems.terraphim-it');
    assert.ok(extension,'existing extension discovered');
    await extension.activate();
    assert.equal(extension.isActive,true); report.checks.push('existing extension activates');
    const commands = await vscode.commands.getCommands(true);
    assert.ok(commands.includes('terraphimIt.openAlternativeControl'));
    report.checks.push('custom editor command registered');
    const source = 'a banana. Café 😀\r\nSecond line.\r\n';
    const uri = vscode.Uri.file('/private/tmp/terraphim-vscode-native-smoke-20261011.md');
    await vscode.workspace.fs.writeFile(uri,Buffer.from(source));
    const document = await vscode.workspace.openTextDocument(uri);
    await vscode.window.showTextDocument(document);
    assert.equal(document.getText(),source); report.checks.push('native document preserves exact Unicode CRLF');
    await vscode.commands.executeCommand('vscode.openWith',uri,'terraphimIt.alternativeControl');
    assert.equal(document.getText(),source); report.checks.push('registered custom provider opens without changing source');
    report.result = 'PASS';
  } catch(error) { report.result='FAIL'; report.error=String(error.stack || error); throw error; }
  finally { await fs.writeFile('/private/tmp/terraphim-vscode-native-smoke-20261011.json',JSON.stringify(report,null,2)); }
};
