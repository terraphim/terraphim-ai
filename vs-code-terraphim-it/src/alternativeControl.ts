import * as vscode from 'vscode';
import { createHash, randomUUID } from 'crypto';
import { exportKnowledgeGraph } from '../media/knowledge-graph.cjs';
import { assertSeparateExportTarget } from '../media/export-target.cjs';

const VIEW = 'terraphimIt.alternativeControl';
const LIMIT = 16 * 1024 * 1024;
const hash = (source: string) => createHash('sha256').update(source, 'utf8').digest('hex');
interface HostMessage {
  protocol: number; type: string; resource: string; session: string;
  revision: number; id?: number; beforeHash?: string; sourceHash?: string; source?: string;
  format?: 'markdown' | 'knowledge-graph';
}
interface View { session: string; panel: vscode.WebviewPanel; lastRequest: number; exportPending?: number; }
interface Resource {
  views: Set<View>; queue: Promise<void>;
  inFlight?: { session: string; id: number; source: string };
}
export function registerAlternativeControl(context: vscode.ExtensionContext): void {
  const provider = new AlternativeControlProvider(context.extensionUri);
  context.subscriptions.push(vscode.window.registerCustomEditorProvider(VIEW, provider, {
    supportsMultipleEditorsPerDocument: true,
    webviewOptions: { retainContextWhenHidden: true },
  }));
  context.subscriptions.push(vscode.commands.registerCommand('terraphimIt.openAlternativeControl', async () => {
    const uri = vscode.window.activeTextEditor?.document.uri;
    if (uri) await vscode.commands.executeCommand('vscode.openWith', uri, VIEW);
    else {
      const selected = await vscode.window.showOpenDialog({ canSelectMany: false, filters: { Markdown: ['md'] } });
      if (selected?.[0]) await vscode.commands.executeCommand('vscode.openWith', selected[0], VIEW);
    }
  }));
  context.subscriptions.push(vscode.commands.registerCommand('terraphimIt.exportKnowledgeGraph', () => provider.requestKnowledgeGraphExport()));
}
class AlternativeControlProvider implements vscode.CustomTextEditorProvider {
  private readonly resources = new Map<string, Resource>();
  constructor(private readonly extensionUri: vscode.Uri) {}
  async requestKnowledgeGraphExport(): Promise<void> {
    for (const [resource, state] of this.resources) {
      for (const view of state.views) {
        if (view.panel.active) {
          await view.panel.webview.postMessage({ protocol:1, type:'request-export', resource,
            session:view.session, format:'knowledge-graph' });
          return;
        }
      }
    }
    await vscode.window.showInformationMessage('Open a Markdown document with Terraphim Alternative Control to export its knowledge graph.');
  }
  resolveCustomTextEditor(document: vscode.TextDocument, panel: vscode.WebviewPanel): void {
    const resource = document.uri.toString();
    let state = this.resources.get(resource);
    if (!state) { state = { views: new Set(), queue: Promise.resolve() }; this.resources.set(resource, state); }
    const current = state;
    const view: View = { session: randomUUID(), panel, lastRequest: 0 };
    current.views.add(view);
    const media = vscode.Uri.joinPath(this.extensionUri, 'media');
    panel.webview.options = { enableScripts: true, localResourceRoots: [media] };
    panel.webview.html = html(panel.webview, media);
    let disposed = false;
    const sync = (type = 'sync', extra: Record<string, unknown> = {}) => panel.webview.postMessage({
      protocol: 1, type, resource, session: view.session, revision: document.version,
      source: document.getText(), ...extra,
    });
    const writeExport = async (message: HostMessage, output: string, format: 'markdown' | 'knowledge-graph') => {
      let terminal = 'command-complete';
      let reason: string | undefined;
      const unchanged = () => !disposed && !document.isClosed && document.version === message.revision
        && hash(document.getText()) === message.sourceHash;
      try {
        if (typeof output !== 'string' || Buffer.byteLength(output,'utf8') > LIMIT) throw new Error('Invalid export output');
        if (!unchanged()) throw new Error('Document changed while preparing export');
        const graph = format === 'knowledge-graph';
        const target = await vscode.window.showSaveDialog({
          filters:graph ? { 'Knowledge graph':['json'] } : { Markdown:['md'] },
          defaultUri:document.uri.with({ path:document.uri.path.replace(/\.md$/i,'') + (graph ? '.knowledge-graph.json' : '.clean.md') }),
        });
        if (!target || disposed) return;
        if (!unchanged()) throw new Error('Document changed while choosing export destination');
        await assertSeparateExportTarget(resource,target.toString(),vscode.workspace.textDocuments.filter(d => d.isDirty).map(d => d.uri.toString()));
        if (!unchanged()) throw new Error('Document changed while checking export destination');
        if (target.toString() === resource || vscode.workspace.textDocuments.some(d => d.uri.toString() === target.toString() && d.isDirty)) {
          throw new Error('Choose a separate output file with no unsaved changes.');
        }
        await vscode.workspace.fs.writeFile(target,Buffer.from(output,'utf8'));
        await vscode.window.showTextDocument(await vscode.workspace.openTextDocument(target));
      } catch (error) { terminal = 'reject'; reason = String(error); }
      finally { await sync(terminal, { id:message.id, reason }); }
    };
    const changed = vscode.workspace.onDidChangeTextDocument(event => {
      if (event.document.uri.toString() !== resource || event.contentChanges.length === 0) return;
      const origin = current.inFlight?.source === document.getText() ? current.inFlight : undefined;
      void sync('sync', { originSession: origin?.session, originId: origin?.id });
    });
    const listener = panel.webview.onDidReceiveMessage((value: unknown) => {
      if (!value || typeof value !== 'object') return;
      const message = value as HostMessage;
      if (message.type === 'ready') { void sync('init'); return; }
      if (message.protocol !== 1 || message.resource !== resource || message.session !== view.session) return;
      current.queue = current.queue.then(async () => {
        if (disposed || document.isClosed) return;
        if (['export-result','export-refused'].includes(message.type) && message.id !== view.exportPending) return;
        if (message.type === 'export-refused') {
          view.exportPending = undefined;
          await sync('reject',{id:message.id,reason:'Export snapshot changed before it could be produced'}); return;
        }
        const source = document.getText();
        const expected = message.type === 'proposal' ? message.beforeHash : message.sourceHash;
        if (message.revision !== document.version || expected !== hash(source)) {
          if (message.type === 'export-result') view.exportPending = undefined;
          await sync('reject', { id: message.id, reason: 'Document changed before this operation' }); return;
        }
        if (message.type === 'proposal') {
          if (typeof message.source !== 'string' || Buffer.byteLength(message.source,'utf8') > LIMIT
              || !Number.isSafeInteger(message.id) || (message.id ?? 0) <= view.lastRequest
              || hash(message.source) !== message.sourceHash) {
            await sync('reject', { id: message.id, reason: 'Invalid text proposal' }); return;
          }
          if (message.source === source) { await sync('reject', { id: message.id, reason: 'No document change' }); return; }
          view.lastRequest = message.id!;
          const edit = new vscode.WorkspaceEdit();
          edit.replace(document.uri, new vscode.Range(document.positionAt(0), document.positionAt(source.length)), message.source);
          current.inFlight = { session: view.session, id: message.id!, source: message.source };
          try {
            const applied = await vscode.workspace.applyEdit(edit);
            if (applied && document.getText() === message.source) await sync('ack', { id: message.id });
            else await sync('reject', { id: message.id, reason: 'Host did not produce the exact proposed output' });
          } finally { current.inFlight = undefined; }
        } else if (message.type === 'recover-draft') {
          if (typeof message.source !== 'string' || Buffer.byteLength(message.source,'utf8') > LIMIT) return;
          const recovered = await vscode.workspace.openTextDocument({ language:'markdown', content:message.source });
          if (!disposed) await vscode.commands.executeCommand('vscode.openWith', recovered.uri, VIEW);
        } else if (message.type === 'export-result') {
          // Keep the command barrier until the destination dialog and output
          // write finish, including cancellation and refusal paths.
          view.exportPending = undefined;
          await writeExport(message,message.source!, 'markdown');
        } else if (message.type === 'undo' || message.type === 'redo') {
          // These commands route through the focused editor. A queued request
          // must never alter a different document after the user switches tabs.
          if (!panel.active || !vscode.window.state.focused) {
            await sync('reject', { id:message.id, reason:'Return focus to this editor before changing its history' });
            return;
          }
          await vscode.commands.executeCommand(message.type);
        } else if (message.type === 'save') {
          if (!await document.save()) throw new Error('Document save was cancelled or refused');
        }
        else if (message.type === 'open') {
          const selected = await vscode.window.showOpenDialog({ canSelectMany: false, filters: { Markdown: ['md'] } });
          if (selected?.[0] && !disposed) await vscode.commands.executeCommand('vscode.openWith', selected[0], VIEW);
        } else if (message.type === 'export') {
          if (!Number.isSafeInteger(message.id) || (message.id ?? 0) <= view.lastRequest
              || !['markdown','knowledge-graph'].includes(message.format || 'markdown')) {
            await sync('reject',{id:message.id,reason:'Invalid export request'}); return;
          }
          view.lastRequest = message.id!;
          if (message.format === 'knowledge-graph') {
            const output = await exportKnowledgeGraph(source,vscode.Uri.joinPath(media,'editor').fsPath);
            await writeExport(message,output,'knowledge-graph');
          } else {
            view.exportPending = message.id;
            await panel.webview.postMessage({
              protocol:1, type:'export-request', id:message.id, resource, session:view.session,
              revision:document.version, sourceHash:hash(document.getText()),
            });
          }
        }
        if (['undo','redo','save','open'].includes(message.type)) {
          await sync('command-complete', { id:message.id });
        }
      }).catch(error => { void sync('reject', { id: message.id, reason: String(error) }); });
    });
    panel.onDidDispose(() => {
      disposed = true; listener.dispose(); changed.dispose(); current.views.delete(view);
      if (current.views.size === 0) this.resources.delete(resource);
    });
  }
}
function escape(value: string): string {
  return value.replace(/[&<>"']/g, c => ({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]!));
}
function html(webview: vscode.Webview, media: vscode.Uri): string {
  const assets = webview.asWebviewUri(vscode.Uri.joinPath(media,'editor')).toString() + '/';
  const script = webview.asWebviewUri(vscode.Uri.joinPath(media,'alternative-control.js')).toString();
  const hostStyle = webview.asWebviewUri(vscode.Uri.joinPath(media,'alternative-control.css')).toString();
  const intent = webview.asWebviewUri(vscode.Uri.joinPath(media,'export-intent.js')).toString();
  const nonce = randomUUID();
  return `<!DOCTYPE html><html><head><meta charset="utf-8">
<meta http-equiv="Content-Security-Policy" content="default-src 'none'; img-src ${webview.cspSource} data:; font-src ${webview.cspSource} data:; style-src ${webview.cspSource} 'unsafe-inline'; script-src ${webview.cspSource} 'nonce-${nonce}' 'wasm-unsafe-eval'; connect-src ${webview.cspSource}; worker-src ${webview.cspSource} blob:;">
<link rel="stylesheet" href="${escape(assets)}terraphim-editor.min.css">
<link rel="stylesheet" href="${escape(hostStyle)}">
</head><body data-assets="${escape(assets)}"><div id="host-notice" role="alert" hidden></div><div id="editor"></div>
<script nonce="${nonce}" src="${escape(assets)}terraphim-editor.min.js"></script>
<script nonce="${nonce}" src="${escape(assets)}host-session.js"></script>
<script nonce="${nonce}" src="${escape(intent)}"></script>
<script nonce="${nonce}" src="${escape(script)}"></script></body></html>`;
}
