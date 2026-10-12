/* Real installed VS Code document/view lifecycle. Does not qualify webview rendering. */
const vscode = require('vscode');
const fs = require('node:fs/promises');
const assert = require('node:assert/strict');
const crypto = require('node:crypto');
const path = require('node:path');
const digest = value => crypto.createHash('sha256').update(value).digest('hex');
const waitFor = async predicate => {
  const deadline = Date.now() + 10000;
  while (!predicate()) {
    assert.ok(Date.now() < deadline, 'native host condition timed out');
    await new Promise(resolve => setTimeout(resolve, 25));
  }
};
exports.run = async () => {
  const output = process.env.TERRAPHIM_NATIVE_LIFECYCLE_OUTPUT;
  assert.ok(output && path.isAbsolute(output), 'explicit isolated output directory required');
  await fs.mkdir(output); // Refuse reuse: retain every earlier output and failure receipt.
  const report = {scope:'Native VS Code document persistence, history, custom-view lifecycle and packaged Rust/WASM graph export from saved files; graph export is a core helper call, not a webview download or save-dialog interaction. Webview contents, save cancellation, all41 and upgrade/rollback unqualified', checks:[]};
  const uri = vscode.Uri.file(path.join(output,'actual-native-document.md'));
  const initial = 'Decision: review Café 😀.\r\nRisk: lost context.\r\n';
  const changed = 'Judgment: review Café 😀.\r\nRisk: lost context.\r\n';
  let document;
  try {
    const extension = vscode.extensions.getExtension('applied-knowledge-systems.terraphim-it');
    assert.ok(extension); await extension.activate();
    report.extensionPath = extension.extensionPath;
    report.assetLockSha256 = digest(await fs.readFile(path.join(extension.extensionPath,'media/editor-build.json')));
    await vscode.workspace.fs.writeFile(uri,Buffer.from(initial));
    document = await vscode.workspace.openTextDocument(uri);
    const view = 'terraphimIt.alternativeControl';
    await vscode.commands.executeCommand('vscode.openWith',uri,view,vscode.ViewColumn.One);
    await vscode.commands.executeCommand('vscode.openWith',uri,view,vscode.ViewColumn.Two);
    const customTabs = () => vscode.window.tabGroups.all.flatMap(group=>group.tabs)
      .filter(tab=>tab.input?.uri?.toString()===uri.toString() && tab.input?.viewType===view);
    await waitFor(()=>customTabs().length===2);
    assert.equal(document.getText(),initial);
    report.checks.push('two actual custom-editor tabs on the same native document, source unchanged');
    await vscode.window.showTextDocument(document,{viewColumn:vscode.ViewColumn.One});
    const edit = new vscode.WorkspaceEdit();
    edit.replace(uri,new vscode.Range(document.positionAt(0),document.positionAt(initial.length)),changed);
    assert.equal(await vscode.workspace.applyEdit(edit),true);
    assert.equal(document.getText(),changed); assert.equal(await document.save(),true);
    assert.equal(await fs.readFile(uri.fsPath,'utf8'),changed);
    await fs.copyFile(uri.fsPath,path.join(output,'saved-change.md'));
    report.checks.push('real native edit and save preserve Unicode and CRLF bytes');
    const assets = path.join(extension.extensionPath,'media/editor');
    const glue = await fs.readFile(path.join(assets,'terraphim_editor.js'));
    const bindings = await import('data:text/javascript;base64,'+glue.toString('base64')+'#native-graph-input');
    await bindings.default({module_or_path:await fs.readFile(path.join(assets,'terraphim_editor_bg.wasm'))});
    const briefUri = vscode.Uri.file(path.join(output,'risk-concern.md'));
    const brief = 'Risk: supplier delay. Decision: retain a backup supplier.\n';
    await vscode.workspace.fs.writeFile(briefUri,Buffer.from(brief));
    const briefDocument = await vscode.workspace.openTextDocument(briefUri);
    const engine = new bindings.TerraphimHostSession(briefUri.toString(),'native-synonym-export',briefDocument.version,brief);
    try {
      const snapshot = JSON.parse(engine.snapshot());
      const proposal = JSON.parse(engine.propose(JSON.stringify({...snapshot,id:1,command:'alt_create',
        args:{kind:'word',start:0,end:4,text:'Concern'}})));
      const annotate = new vscode.WorkspaceEdit();
      annotate.replace(briefUri,new vscode.Range(briefDocument.positionAt(0),briefDocument.positionAt(brief.length)),proposal.source);
      assert.equal(await vscode.workspace.applyEdit(annotate),true);
      engine.acknowledge(1,briefDocument.version,briefDocument.getText());
      assert.equal(await briefDocument.save(),true);
      const savedBrief = await fs.readFile(briefUri.fsPath,'utf8');
      assert.equal(savedBrief,proposal.source);
      const committed = JSON.parse(engine.snapshot());
      assert.equal(committed.projection.annotations.synonymGroups[0].markdown,'synonyms:: Risk, Concern\n');
      const {exportKnowledgeGraph} = require(path.join(extension.extensionPath,'media/knowledge-graph.cjs'));
      const graphUri = vscode.Uri.file(path.join(output,'risk-concern.knowledge-graph.json'));
      await vscode.workspace.fs.writeFile(graphUri,Buffer.from(await exportKnowledgeGraph(savedBrief,assets)));
      const savedGraph = JSON.parse(await fs.readFile(graphUri.fsPath,'utf8'));
      assert.deepEqual(Object.keys(savedGraph.data).sort(),['concern','risk']);
      assert.equal(savedGraph.data.risk.id,savedGraph.data.concern.id);
      assert.equal(savedGraph.data.risk.nterm,savedGraph.data.concern.nterm);
      assert.equal(await fs.readFile(briefUri.fsPath,'utf8'),savedBrief);
      report.checks.push('installed Rust/WASM creates synonyms:: Risk, Concern; actual native document saves annotations; packaged host helper exports saved JSON with one shared concept and preserves source');
    } finally {engine.free();}
    assert.ok(vscode.window.state.focused,'Native undo/redo acceptance requires a focused VS Code window; the locked or background desktop cannot qualify history');
    await vscode.commands.executeCommand('workbench.action.focusActiveEditorGroup');
    await vscode.commands.executeCommand('undo'); await waitFor(()=>document.getText()===initial);
    assert.equal(await document.save(),true); assert.equal(await fs.readFile(uri.fsPath,'utf8'),initial);
    await fs.copyFile(uri.fsPath,path.join(output,'saved-undo.md'));
    await vscode.commands.executeCommand('workbench.action.focusActiveEditorGroup');
    await vscode.commands.executeCommand('redo'); await waitFor(()=>document.getText()===changed);
    assert.equal(await document.save(),true); assert.equal(await fs.readFile(uri.fsPath,'utf8'),changed);
    await fs.copyFile(uri.fsPath,path.join(output,'saved-redo.md'));
    report.checks.push('native undo/redo each saved as exact actual bytes');
    assert.equal(await vscode.window.tabGroups.close(customTabs()),true);
    await waitFor(()=>customTabs().length===0);
    await vscode.commands.executeCommand('vscode.openWith',uri,view,vscode.ViewColumn.Two);
    await waitFor(()=>customTabs().length===1);
    assert.equal(document.getText(),changed);
    report.checks.push('custom views dispose and reopen without modifying committed native source');
    report.result='PASS';
  } catch(error) {report.result='FAIL';report.error=String(error.stack || error);report.actualSourceAtFailure=document?.getText();report.ownDocumentDirty=document?.isDirty;report.activeOwnNativeEditor=vscode.window.activeTextEditor?.document.uri.toString()===uri.toString();throw error;}
  finally {
    report.windowFocused = vscode.window.state.focused;
    report.outputs = {};
    for (const name of ['saved-change.md','saved-undo.md','saved-redo.md','risk-concern.md','risk-concern.knowledge-graph.json']) {
      try {
        const bytes = await fs.readFile(path.join(output,name));
        report.outputs[name]={bytes:bytes.length,sha256:digest(bytes)};
      } catch(error) {if(error.code !== 'ENOENT') throw error;}
    }
    await fs.writeFile(path.join(output,'receipt.json'),JSON.stringify(report,null,2)+'\n');
  }
};
