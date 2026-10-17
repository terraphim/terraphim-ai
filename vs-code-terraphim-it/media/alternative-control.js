/* A VS Code adapter around the shared UI and host-session protocol. */
(() => {
  'use strict';
  const vscode = acquireVsCodeApi();
  const container = document.getElementById('editor');
  const notice = document.getElementById('host-notice');
  const assets = document.body.dataset.assets;
  const hostNotice = new TerraphimHostNotice(notice);
  const exports = new TerraphimExportIntent();
  let editor;
  let bridge;
  let bindings;
  let loading;
  let adopting = false;
  let startupSync;
  // VS Code forwards webview keydowns even after preventDefault. Keep only
  // keys already consumed by this editor local; unhandled keys still reach
  // the workbench. This includes AI alternatives, formatting and stash.
  container.addEventListener('keydown', event => {
    if (event.defaultPrevented && !event.isComposing) event.stopPropagation();
  });
  const serialise = () => {
    const md = editor.markdownEditor;
    const api = md.documentApi();
    md.alignDocumentModel(api);
    return api.save_document();
  };
  function dispatchCommand(type, format = 'markdown') {
    // Save/export must include pending annotation-only and debounced Overflow
    // changes, then wait for their exact native ACK before the host command.
    if (type === 'export' && !exports.begin(format)) return;
    try {
      if (type === 'save' || type === 'export') bridge.submit(serialise());
      bridge.command(type);
    } catch (error) { if (type === 'export') exports.clear(); fail(error); }
  }
  function hostHistoryTarget(target) {
    return TerraphimHostBridge.ownsHistory({
      inSurface:editor.markdownEditor.surface.root.contains(target),
      alternative:target instanceof HTMLInputElement && target.classList.contains('te-alt-input'),
      isNew:target instanceof HTMLInputElement && target.classList.contains('te-alt-input--new'),
      value:target?.value, committed:target?.dataset?.committed,
    });
  }
  function adopt(source, info = {}) {
    const presentation = bindings.host_presentation_source(source);
    // Compare the full canonical source, including every annotation, before
    // retaining the local focused input across an acknowledged EOL roundtrip.
    if (info.canonical && presentation === serialise()) return;
    adopting = true;
    try {
      const surface = editor.markdownEditor.surface;
      const selection = surface.getSelectionOffsets();
      editor.openDocument(presentation);
      const size = surface.getText().length;
      surface.setSelectionOffsets(Math.min(selection.start,size),Math.min(selection.end,size));
      // The host stores annotation/body history; this local stack is only a
      // temporary transaction mechanism for the shared browser controllers.
      surface.resetHistory();
    } finally { adopting = false; }
  }
  function conflict(info) {
    exports.clear();
    container.inert = true;
    vscode.setState({ resource:bridge.current.resource, draft:info.draft });
    hostNotice.blocking();
    notice.replaceChildren(); notice.hidden = false;
    const text = document.createElement('p'); text.textContent = `${info.reason}. Your local draft is retained.`;
    const recover = document.createElement('button'); recover.textContent = 'Open recovery draft';
    recover.addEventListener('click',() => vscode.postMessage({
      protocol:1, type:'recover-draft', resource:bridge.current.resource, session:bridge.current.session,
      revision:bridge.current.revision, sourceHash:bridge.current.sourceHash, source:info.draft,
    }));
    const resume = document.createElement('button'); resume.textContent = 'Use current document';
    resume.addEventListener('click',() => {
      bridge.resume(); container.inert = false; notice.hidden = true; vscode.setState(undefined);
    });
    notice.append(text,recover,resume);
  }
  async function initialize(message) {
    if (bridge || loading) return;
    loading = true;
    try {
      bindings = await TeraphimEditor.loadModule({ glueUrl:assets+'terraphim_editor.js', wasmUrl:assets+'terraphim_editor_bg.wasm' });
      editor = await TeraphimEditor.create(container, {
        bindings, assetBase:assets, allowExternalAssets:true, dirtyTitle:false,
        config:{ historyOwner:{ undo:() => bridge.command('undo'), redo:() => bridge.command('redo') }, hostManaged:true, fileSystemAccess:false, standalone:false, documentKey:message.resource },
      });
      // The bundled local role enables KG alternatives and protected Lab trim.
      // Keep editing available if role loading fails; Lab reports its disabled state.
      try {
        const role = await fetch(assets+'roles/default.json');
        if (!role.ok) throw new Error(`Local role returned ${role.status}`);
        editor.markdownEditor.loadThesaurus(await role.text());
      } catch (error) {
        hostNotice.recoverable(`Terraphim local role unavailable: ${String(error)}`);
      }
      editor.openDocument(bindings.host_presentation_source(message.source),message.resource);
      bridge = new TerraphimHostBridge({
        engine:new bindings.TerraphimHostSession(message.resource,message.session,message.revision,message.source),
        send:m => vscode.postMessage(exports.attach(m)), adopt, conflict,
      });
      const changed = () => {
        if (adopting) return;
        queueMicrotask(() => {
        if (!adopting && !bridge.paused && !bridge.disposed) {
          try {
            const draft = serialise();
            vscode.setState({ resource:bridge.current.resource, draft });
            bridge.submit(draft);
            if (!bridge.pending && !bridge.paused && bridge.latest === bridge.current.source) vscode.setState(undefined);
          } catch (error) { fail(error); }
        }
        });
      };
      if (startupSync) { bridge.receive(startupSync); startupSync = undefined; }
      editor.markdownEditor.surface.onChange(changed);
      // Ghost/revive changes annotations without changing surface text.
      // Submit the native transaction before Save/Undo can cross the host barrier.
      container.addEventListener('te:ghosts-change',changed);
      container.addEventListener('te:overflow-change',changed);
      for (const [eventName, command] of [['te:save','save'],['te:open','open'],['te:markdown','export']]) {
        document.addEventListener(eventName,event => {
          event.preventDefault(); dispatchCommand(command);
        });
      }
      // Retain the graph preview, but route downloads through the acknowledged
      // native document and native save dialog instead of browser blobs.
      document.addEventListener('click',event => {
        const action = event.composedPath().find(node => node instanceof Element &&
          node.matches('.te-files-graph [data-action="download-graph"], .te-files-graph [data-action="download-graph-text"]'));
        if (action) {
          event.preventDefault(); event.stopImmediatePropagation();
          dispatchCommand('export',action.dataset.action === 'download-graph' ? 'knowledge-graph' : 'markdown');
        }
      },true);
      document.addEventListener('keydown',event => {
        if (event.isComposing || !(event.ctrlKey || event.metaKey) || event.altKey) return;
        const key = event.key.toLowerCase();
        const inSurface = hostHistoryTarget(event.target);
        let command;
        if (key === 's' && !event.shiftKey) command = 'save';
        else if (key === 'o' && !event.shiftKey) command = 'open';
        else if (inSurface && key === 'z') command = event.shiftKey ? 'redo' : 'undo';
        else if (inSurface && key === 'y' && !event.shiftKey) command = 'redo';
        if (command) { event.preventDefault(); event.stopImmediatePropagation(); dispatchCommand(command); }
      },true);
      container.addEventListener('beforeinput',event => {
        if (event.inputType !== 'historyUndo' && event.inputType !== 'historyRedo') return;
        if (!hostHistoryTarget(event.target)) return;
        event.preventDefault(); event.stopImmediatePropagation();
        bridge.command(event.inputType === 'historyUndo' ? 'undo' : 'redo');
      },true);
      const retained = vscode.getState();
      if (retained?.resource === message.resource && typeof retained.draft === 'string' && retained.draft !== bridge.current.source) {
        bridge.paused = true; conflict({ draft:retained.draft,reason:'A recovery draft is available' });
      }
    } catch (error) { fail(error); }
  }
  function fail(error) {
    container.inert = true; hostNotice.blocking(); notice.hidden = false;
    notice.textContent = `Terraphim could not complete the operation: ${String(error)}`;
  }
  window.addEventListener('message',event => {
    const message = event.data;
    if (!message || message.protocol !== 1) return;
    if (message.type === 'init') { void initialize(message); return; }
    if (!bridge) {
      if (message.type === 'sync' && (!startupSync || message.revision > startupSync.revision)) startupSync = message;
      return;
    }
    try {
      if (message.type === 'request-export' && message.resource === bridge.current.resource
          && message.session === bridge.current.session && message.format === 'knowledge-graph') {
        dispatchCommand('export','knowledge-graph'); return;
      }
      if (message.type === 'export-request' && message.resource === bridge.current.resource
          && message.session === bridge.current.session) {
        if (message.id !== bridge.commandPending) return;
        try {
          vscode.postMessage({...message,type:'export-result',source:bridge.exportSnapshot(message)});
        } catch (error) {
          vscode.postMessage({...message,type:'export-refused'});
        }
      } else {
        const accepted = bridge.receive(message);
        exports.settle(message,accepted);
        if (accepted && !bridge.paused && message.type === 'reject' && message.reason) {
          hostNotice.recoverable(`Terraphim: ${message.reason}`);
        }
        if (!bridge.pending && !bridge.paused && bridge.latest === bridge.current.source) vscode.setState(undefined);
      }
    } catch (error) { fail(error); }
  });
  window.addEventListener('pagehide',() => { exports.clear(); bridge?.dispose(); editor?.destroy(); });
  vscode.postMessage({ protocol:1,type:'ready' });
})();
