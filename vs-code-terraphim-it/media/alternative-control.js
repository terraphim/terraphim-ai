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
  let historyGeneration=0;
  let acceptedCheckpoint;
  let incomingCheckpoint;
  let historyPending;
  let activeView=false;
  let viewEpoch=0;
  let namedHistory;
  const commands=[];
  const flushRequests=new Set();
  const hostStages=new Map();
  let hostStageTimer;
  const clearHostStages=()=>{clearTimeout(hostStageTimer);hostStages.clear();};
  function sendWithCheckpoint(message) {
    if(message.type!=='proposal' && message.type!=='history-update') {vscode.postMessage(message);return;}
    const saved=checkpoint(),encoded=TerraphimHistoryCheckpoint.chunks(saved),token=crypto.randomUUID();
    for(let index=0;index<encoded.parts.length;index++) vscode.postMessage({protocol:1,type:'checkpoint-chunk',
      resource:bridge.current.resource,session:bridge.current.session,revision:bridge.current.revision,
      sourceHash:bridge.current.sourceHash,historyGeneration,token,index,total:encoded.parts.length,bytes:encoded.bytes,text:encoded.parts[index]});
    const command=namedHistory;namedHistory=undefined;
    vscode.postMessage({...message,checkpointToken:token,historyGeneration,...(command||{})});
  }
  function hostChunk(message) {
    if(typeof message.token!=='string'||!Number.isSafeInteger(message.index)||!Number.isSafeInteger(message.total)||!Number.isSafeInteger(message.bytes)
      ||typeof message.text!=='string'||message.text.length>256*1024||message.total<1||message.index<0||message.bytes<0||message.bytes>503*16*1024*1024)throw new Error('Invalid host checkpoint chunk');
    if(message.index===0){clearHostStages();hostStages.set(message.token,{...message,parts:[],size:0});hostStageTimer=setTimeout(clearHostStages,60000);}
    const staged=hostStages.get(message.token);
    if(!staged||staged.parts.length!==message.index||staged.total!==message.total||staged.bytes!==message.bytes
      ||staged.resource!==message.resource||staged.session!==message.session||staged.revision!==message.revision||staged.historyGeneration!==message.historyGeneration)throw new Error('Host checkpoint sequence mismatch');
    staged.size+=new TextEncoder().encode(message.text).length;
    if(staged.size>staged.bytes+staged.total*2)throw new Error('Host checkpoint exceeds declared size');
    staged.parts.push(message.text);
  }
  function attachHostCheckpoint(message) {
    if(!message.checkpointToken)return;
    const staged=hostStages.get(message.checkpointToken);clearHostStages();
    if(!staged||staged.parts.length!==staged.total||staged.resource!==message.resource||staged.session!==message.session
      ||staged.revision!==message.revision||staged.historyGeneration!==message.historyGeneration)throw new Error('Missing host checkpoint');
    const json=staged.parts.join('');if(new TextEncoder().encode(json).length!==staged.bytes)throw new Error('Host checkpoint byte count mismatch');
    message.checkpoint=JSON.parse(json);
  }
  const checkpoint=()=>TerraphimHistoryCheckpoint.capture(editor.markdownEditor.surface,serialise(),editor.markdownEditor.altPanel);
  const fingerprint=value=>JSON.stringify(value);
  function drive() {
    if (!bridge || bridge.paused || bridge.disposed || bridge.pending || bridge.commandPending) return;
    const saved=checkpoint();
    if (fingerprint(saved)!==acceptedCheckpoint) {
      const id=++bridge.sequence;
      historyPending=id; bridge.commandPending=id;
      sendWithCheckpoint({protocol:1,type:'history-update',id,resource:bridge.current.resource,
        session:bridge.current.session,revision:bridge.current.revision,sourceHash:bridge.current.sourceHash,
        historyGeneration});return;
    }
    for (const nonce of flushRequests) vscode.postMessage({protocol:1,type:'flush-complete',nonce,
      resource:bridge.current.resource,session:bridge.current.session,revision:bridge.current.revision,
      sourceHash:bridge.current.sourceHash,historyGeneration});
    flushRequests.clear();
    if(commands.length) bridge.command(commands.shift());
  }
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
      commands.push(type); drive();
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
      if (incomingCheckpoint) TerraphimHistoryCheckpoint.restore(surface,incomingCheckpoint,presentation,editor.markdownEditor);
      acceptedCheckpoint=fingerprint(checkpoint());
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
      if (info.resourceConflict) { vscode.postMessage({protocol:1,type:'resolve-conflict',resource:bridge.current.resource,session:bridge.current.session,revision:bridge.current.revision,sourceHash:bridge.current.sourceHash}); return; }
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
        config:{ hostManaged:true, fileSystemAccess:false, standalone:false, documentKey:message.resource },
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
      activeView=message.active;viewEpoch=message.viewEpoch;
      historyGeneration=message.historyGeneration; incomingCheckpoint=message.checkpoint;
      if(incomingCheckpoint) TerraphimHistoryCheckpoint.restore(editor.markdownEditor.surface,incomingCheckpoint,serialise(),editor.markdownEditor);
      acceptedCheckpoint=fingerprint(checkpoint());
      bridge = new TerraphimHostBridge({
        engine:new bindings.TerraphimHostSession(message.resource,message.session,message.revision,message.source),
        send:m => sendWithCheckpoint(exports.attach(m)), adopt, conflict,
      });
      const changed = () => {
        if (adopting) return;
        queueMicrotask(() => {
        if (!adopting && !bridge.paused && !bridge.disposed) {
          try {
            const draft = serialise();
            vscode.setState({ resource:bridge.current.resource, draft });
            bridge.submit(draft); drive();
            if (!bridge.pending && !bridge.paused && bridge.latest === bridge.current.source) vscode.setState(undefined);
          } catch (error) { fail(error); }
        }
        });
      };
      if (startupSync) { bridge.receive(startupSync); startupSync = undefined; }
      editor.markdownEditor.surface.onChange(changed);
      // Existing methods also change history without changing source (fold,
      // cancellation's forgetRedo). Transport those changes after the whole
      // synchronous controller operation, not half of its transaction.
      const surface=editor.markdownEditor.surface;
      for(const name of ['record','squashHistory','forgetRedo','trimHistory']) {
        const original=surface[name]; surface[name]=function(...args) {
          const result=original.apply(this,args); changed(); return result;
        };
      }
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
        // Surface and alternatives-panel retain their existing local history.
        void inSurface;
        if (command) { event.preventDefault(); event.stopImmediatePropagation(); dispatchCommand(command); }
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
    try {if(message.type==='checkpoint-chunk'){hostChunk(message);return;}attachHostCheckpoint(message);}catch(error){fail(error);return;}
    if (message.type === 'init') { void initialize(message); return; }
    if (!bridge) {
      if (message.type === 'sync' && (!startupSync || message.revision > startupSync.revision)) startupSync = message;
      return;
    }
    try {
      if(message.type==='view-state' && message.resource===bridge.current.resource && message.session===bridge.current.session) {activeView=message.active;viewEpoch=message.viewEpoch;return;}
      if (message.type === 'flush-request' && message.resource===bridge.current.resource && message.session===bridge.current.session) {
        if(bridge.paused) {vscode.postMessage({...message,type:'flush-refused'});return;}
        flushRequests.add(message.nonce); bridge.submit(serialise()); drive(); return;
      }
      if (message.type === 'resource-conflict' && message.resource===bridge.current.resource && message.session===bridge.current.session) {
        bridge.paused=true;conflict({draft:serialise(),reason:message.reason,resourceConflict:true});return;
      }
      if (message.type === 'request-history' && message.resource===bridge.current.resource && message.session===bridge.current.session && ['undo','redo'].includes(message.command)) {
        if(!activeView || viewEpoch!==message.viewEpoch || document.visibilityState==='hidden')return;
        namedHistory={historyCommandNonce:message.historyCommandNonce,viewEpoch:message.viewEpoch};
        editor.markdownEditor.altPanel?.finishLive();const applied=editor.markdownEditor.surface[message.command]();
        if(!applied)namedHistory=undefined;
        editor.markdownEditor.altPanel?.flush();bridge.submit(serialise());drive();return;
      }
      if (message.type === 'history-ack' && message.id===historyPending && message.resource===bridge.current.resource && message.session===bridge.current.session) {
        if(message.revision!==bridge.current.revision || message.source!==bridge.current.source || message.historyGeneration<=historyGeneration) throw new Error('Invalid history acknowledgement');
        historyGeneration=message.historyGeneration;acceptedCheckpoint=fingerprint(message.checkpoint);
        historyPending=undefined;bridge.commandPending=null;bridge.flush();drive();return;
      }
      if (message.type === 'request-export'  && message.resource === bridge.current.resource
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
        if(message.resource!==bridge.current.resource || message.session!==bridge.current.session) return;
        const newerHistory=Number.isSafeInteger(message.historyGeneration)&&message.historyGeneration>historyGeneration;
        incomingCheckpoint=message.checkpoint;
        if(message.type==='reject' && historyPending===message.id) {
          historyPending=undefined;bridge.commandPending=null;bridge.paused=true;
          conflict({draft:serialise(),reason:message.reason||'History changed before this operation'});return;
        }
        if(newerHistory && message.type==='sync' && message.revision===bridge.current.revision && message.source===bridge.current.source) {
          if(bridge.pending || bridge.latest!==bridge.current.source || historyPending || fingerprint(checkpoint())!==acceptedCheckpoint) {
            bridge.paused=true;conflict({draft:serialise(),reason:'Another view changed the document history'});return;
          }
          editor.markdownEditor.altPanel?.finishLive();
          if(message.checkpoint) TerraphimHistoryCheckpoint.restore(editor.markdownEditor.surface,message.checkpoint,serialise(),editor.markdownEditor);
          historyGeneration=message.historyGeneration;acceptedCheckpoint=fingerprint(checkpoint());return;
        }
        if(newerHistory) historyGeneration=message.historyGeneration;
        if(message.type==='ack') acceptedCheckpoint=fingerprint(message.checkpoint);
        if(message.reverted) {
          // Explicit Revert/recovery choice discards the old draft and its
          // pending messages, unlike unsolicited external synchronization.
          if(bridge.pending)bridge.engine.reject(bridge.pending.proposal.id);
          bridge.pending=null;bridge.latest=bridge.current.source;bridge.commandPending=null;bridge.barriers=[];
          historyPending=undefined;commands.length=0;bridge.paused=false;container.inert=false;notice.hidden=true;
        }
        const accepted = bridge.receive(message);
        if(accepted && message.type==='reject' && historyPending===message.id) {historyPending=undefined;}
        drive();
        exports.settle(message,accepted);
        if (accepted && !bridge.paused && message.type === 'reject' && message.reason) {
          hostNotice.recoverable(`Terraphim: ${message.reason}`);
        }
        if (!bridge.pending && !bridge.paused && bridge.latest === bridge.current.source) vscode.setState(undefined);
      }
    } catch (error) { fail(error); }
  });
  window.addEventListener('pagehide',() => { clearHostStages();exports.clear(); bridge?.dispose(); editor?.destroy(); });
  vscode.postMessage({ protocol:1,type:'ready' });
})();
