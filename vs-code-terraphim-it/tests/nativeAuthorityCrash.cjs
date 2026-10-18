/* Actual installed native-authority VS Code UI regression; no injected editor state. */
'use strict';
const assert = require('node:assert/strict');
const fs = require('node:fs/promises');
const os = require('node:os');
const path = require('node:path');
const crypto = require('node:crypto');
const net = require('node:net');
const { spawn, execFileSync } = require('node:child_process');
const { once } = require('node:events');
const { setTimeout: delay } = require('node:timers/promises');
const { chromium } = require('playwright-core');

async function waitFor(check, description, timeout = 15000) {
  const until = Date.now() + timeout;
  while (Date.now() < until) {
    if (await check()) return;
    await delay(100);
  }
  throw new Error('Timed out: ' + description);
}

(async () => {
  assert.equal(process.platform, 'linux', 'This visible-host CI regression is Linux-only');
  const root = path.resolve(__dirname, '..');
  const run = await fs.mkdtemp(path.join(os.tmpdir(), 'ts-'));
  const output = path.resolve(process.env.TERRAPHIM_HISTORY_OUTPUT || path.join(root, 'native-crash-results', path.basename(run)));
  const profile = path.join(run, 'profile');
  const extensions = path.join(run, 'extensions');
  const workspace = path.join(run, 'workspace');
  const filename = path.join(workspace, 'history.md');
  const initial = 'Risk needs mitigation.\n\nThe editor is basically the only target here today.\n';
  const vsix = path.resolve(process.env.TERRAPHIM_HISTORY_VSIX || path.join(root, 'terraphim-it.vsix'));
  await fs.mkdir(workspace);
  const autosave = process.env.TERRAPHIM_HISTORY_AUTOSAVE === '1';
  await fs.mkdir(path.join(profile,'User'),{recursive:true});
  await fs.writeFile(path.join(profile,'User/settings.json'),JSON.stringify({'window.restoreWindows':'all','files.hotExit':'onExitAndWindowClose'}));
  if (autosave) {
    await fs.mkdir(path.join(profile,'User'),{recursive:true});
    await fs.writeFile(path.join(profile,'User/settings.json'),JSON.stringify({'files.autoSave':'afterDelay','files.autoSaveDelay':200}));
  }
  await fs.mkdir(output, { recursive: true });
  await fs.writeFile(filename, initial, { flag: 'wx' });
  await fs.writeFile(path.join(workspace, 'native.md'), 'Native Markdown control.\n', { flag: 'wx' });
  await fs.writeFile(path.join(output, 'input.md'), initial);
  const receipt = { status: 'RUNNING', scope: 'Actual installed VS Code resource-worker crash, received raw-draft archive, explicit fresh-history recovery',
    autosaveDuringLiveAddition: autosave, packageSha256: crypto.createHash('sha256').update(await fs.readFile(vsix)).digest('hex'), checks: [] };
  await fs.copyFile(__filename,path.join(output,'executed-harness.cjs'));
  receipt.modes={autosave,split:process.env.TERRAPHIM_HISTORY_SPLIT==='1',external:process.env.TERRAPHIM_HISTORY_EXTERNAL==='1'};
  let inspectorSocket;let app;
  let launchError;
  let browser;
  let page;
  let log;
  let error;
  try {
    const executable = process.env.TERRAPHIM_VSCODE_EXECUTABLE || await require('@vscode/test-electron').downloadAndUnzipVSCode('1.126.0');
    const cli = path.join(path.dirname(executable), 'bin', 'code');
    const flags = ['--user-data-dir', profile, '--extensions-dir', extensions, '--no-sandbox', '--disable-gpu',
      '--skip-welcome', '--skip-release-notes', '--disable-workspace-trust'];
    const env = { ...process.env };
    delete env.ELECTRON_RUN_AS_NODE;
    receipt.hostVersion = execFileSync(cli, [...flags, '--version'], { env, encoding:'utf8', timeout:60000 }).trim();
    const install = execFileSync(cli, [...flags, '--install-extension', vsix], { env, encoding:'utf8', timeout:60000 });
    await fs.writeFile(path.join(output, 'install.txt'), install);
    const reservation = net.createServer();
    reservation.listen(0, '127.0.0.1');
    await once(reservation, 'listening');
    const port = reservation.address().port;
    await new Promise(resolve => reservation.close(resolve));
    const debugReservation=net.createServer();debugReservation.listen(0,'127.0.0.1');await once(debugReservation,'listening');const inspectorPort=debugReservation.address().port;await new Promise(resolve=>debugReservation.close(resolve));
    log = await fs.open(path.join(output, 'host.log'), 'wx');
    app = spawn(executable, [...flags, `--remote-debugging-port=${port}`,`--inspect-extensions=${inspectorPort}`, workspace, filename],
      { env, stdio:['ignore', log.fd, log.fd] });
    app.once('error', caught => { launchError = caught; });
    await waitFor(async () => {
      if (launchError) throw launchError;
      if (app.exitCode !== null) throw new Error(`VS Code exited before CDP: ${app.exitCode}`);
      try {
        const response = await fetch(`http://127.0.0.1:${port}/json/version`, { signal:AbortSignal.timeout(1000) });
        return response.ok;
      } catch { return false; }
    }, 'actual host CDP', 60000);
    browser = await chromium.connectOverCDP(`http://127.0.0.1:${port}`);
    await waitFor(async () => browser.contexts()[0].pages().length > 0, 'workbench page');
    page = browser.contexts()[0].pages()[0];
    page.setDefaultTimeout(15000);
    await page.bringToFront();
    // VS Code's debug host may wait for its initial debugger handshake.
    let bootTargets;await waitFor(async()=>{try{bootTargets=await(await fetch(`http://127.0.0.1:${inspectorPort}/json/list`)).json();return bootTargets.length>0;}catch{return false;}},'debug host initial endpoint',30000);
    const bootSocket=new WebSocket(bootTargets[0].webSocketDebuggerUrl);await once(bootSocket,'open');
    bootSocket.send(JSON.stringify({id:1,method:'Runtime.runIfWaitingForDebugger'}));
    await once(bootSocket,'message');bootSocket.close();

    await page.getByRole('tab', { name:'history.md', exact:true }).click();
    await page.keyboard.press('Control+Shift+P');
    await page.getByRole('textbox', {name:'Type the name of a command to run.', exact:true}).fill('>Terraphim: Open Alternative Control');
    await page.getByText('Terraphim: Open Alternative Control', {exact:true}).click();
    let frame;
    await waitFor(async () => {
      frame = page.frames().filter(f => f.url().includes('fake.html')).at(-1);
      return frame && await frame.getByRole('textbox', {name:'Markdown source', exact:true}).isVisible();
    }, 'actual Terraphim webview', 60000);
    let source = frame.getByRole('textbox', {name:'Markdown source', exact:true});
    const toggle = frame.getByTitle('Toggle Write_On mode');
    if (await toggle.getAttribute('aria-pressed') !== 'true') await toggle.click();

    const installedNames=(await fs.readdir(extensions,{withFileTypes:true})).filter(entry=>entry.isDirectory()&&entry.name.startsWith('applied-knowledge-systems.terraphim-it-'));
    assert.equal(installedNames.length,1,'Use only the package installed into this isolated profile');
    const installedRoot=path.join(extensions,installedNames[0].name);
    const installedManifest=JSON.parse(await fs.readFile(path.join(installedRoot,'package.json'),'utf8'));
    assert.equal(installedManifest.publisher,'applied-knowledge-systems');
    assert.equal(installedManifest.name,'terraphim-it');
    receipt.installedVersion=installedManifest.version;
    receipt.installedPayload=await new Promise((resolve,reject)=>{
      require('yauzl').open(vsix,{lazyEntries:true},(error,zip)=>{
        if(error)return reject(error);let matchedFiles=0,installerManifest;
        zip.on('error',reject);zip.on('end',()=>resolve({matchedFiles,installerManifest,comparison:'all payload files byte-match; package.json semantics match except installer-owned __metadata and formatting'}));
        zip.on('entry',entry=>{
          if(!entry.fileName.startsWith('extension/')||entry.fileName.endsWith('/')){zip.readEntry();return;}
          const relative=entry.fileName.slice('extension/'.length);
          if(path.posix.normalize(relative)!==relative||relative.startsWith('../')||path.isAbsolute(relative)){zip.close();reject(new Error('Unsafe package entry'));return;}
          zip.openReadStream(entry,(readError,stream)=>{
            if(readError){zip.close();reject(readError);return;}const parts=[];
            stream.on('error',reject);stream.on('data',part=>parts.push(part));stream.on('end',()=>{
              fs.readFile(path.join(installedRoot,relative)).then(async actual=>{const expected=Buffer.concat(parts);
                if(relative==='package.json'){
                  const installed=JSON.parse(actual),original=JSON.parse(expected);assert.equal(Object.hasOwn(original,'__metadata'),false,'package must not preclaim installer metadata');
                  installerManifest={actualSha256:crypto.createHash('sha256').update(actual).digest('hex'),packageSha256:crypto.createHash('sha256').update(expected).digest('hex'),metadata:installed.__metadata};
                  delete installed.__metadata;assert.deepEqual(installed,original,'only installer metadata/formatting may differ');
                  await fs.writeFile(path.join(output,'installed-package.json'),actual);await fs.writeFile(path.join(output,'vsix-package.json'),expected);
                }else {assert.equal(crypto.createHash('sha256').update(actual).digest('hex'),crypto.createHash('sha256').update(expected).digest('hex'),relative);matchedFiles++;}zip.readEntry();}).catch(caught=>{zip.close();reject(caught);});
            });
          });
        });zip.readEntry();
      });
    });
    const installedAssets=path.join(installedRoot,'media/editor');
    const { exportKnowledgeGraph } = require(path.join(installedRoot,'media/knowledge-graph.cjs'));
    async function capture(name, expectedTerms, expectedBody) {
      let saved;
      try { await waitFor(async () => {
        saved = await fs.readFile(filename, 'utf8');
        const graph = JSON.parse(await exportKnowledgeGraph(saved, installedAssets));
        return (await source.innerText()).split('\n')[0] === expectedBody &&
          await page.locator('.tabs-container .tab.dirty').count() === 0 &&
          saved.split('\n')[0] === expectedBody &&
          JSON.stringify(Object.keys(graph.data).sort()) === JSON.stringify(expectedTerms);
      }, name + ' actual saved Markdown and graph');
      } catch(error) {
        const observed = await fs.readFile(filename,'utf8');
        await fs.writeFile(path.join(output,name+'.observed.md'),observed);
        await fs.writeFile(path.join(output,name+'.observed-graph.json'),await exportKnowledgeGraph(observed,installedAssets));
        throw error;
      }
      await fs.writeFile(path.join(output, name + '.md'), saved);
      const graph = JSON.parse(await exportKnowledgeGraph(saved, installedAssets));
      await fs.writeFile(path.join(output, name + '.knowledge-graph.json'), JSON.stringify(graph,null,2)+'\n');
      if (expectedTerms.length) {
        assert.equal(new Set(Object.values(graph.data).map(term=>term.id)).size,1);
        assert.equal(new Set(Object.values(graph.data).map(term=>term.nterm)).size,1);
      }
      await page.screenshot({path:path.join(output,name+'.png')});
      receipt.checks.push({id:name,status:'PASS',sourceSha256:crypto.createHash('sha256').update(saved).digest('hex'),terms:expectedTerms});
      return saved;
    }
    async function focusView(targetFrame) {
      const textbox=targetFrame.getByRole('textbox',{name:'Markdown source',exact:true});
      const box=await textbox.boundingBox();
      assert.ok(box,'Target custom view must be visible before choosing its native group');
      const groups=page.locator('.editor-group-container');
      let selected=false;
      for(let index=0;index<await groups.count();index++) {
        const group=groups.nth(index), bounds=await group.boundingBox();
        const x=box.x+box.width/2,y=box.y+Math.min(12,box.height/2);
        if(bounds&&x>=bounds.x&&x<=bounds.x+bounds.width&&y>=bounds.y&&y<=bounds.y+bounds.height) {
          await group.locator('.tab').last().click();selected=true;break;
        }
      }
      assert.equal(selected,true,'Find actual native tab group containing the observed custom view');
      await textbox.click({position:{x:12,y:12}});
    }
    async function chooseRisk() {
      await page.keyboard.press('Escape');
      await source.click({position:{x:12,y:12}});
      await page.keyboard.press('Control+Home');
      await page.keyboard.down('Shift');
      for(let i=0;i<4;i++) await page.keyboard.press('ArrowRight');
      await page.keyboard.up('Shift');
      assert.equal(await frame.evaluate(()=>getSelection().toString()),'Risk');
      await page.keyboard.press('Control+Shift+A');
      await frame.locator('.te-alt-input--new').waitFor({state:'visible'});
    }
    await chooseRisk();
    await frame.locator('.te-alt-input--new').click();
    if (autosave) {
      let partial='';
      for (const character of 'Concern') {
        partial+=character;
        await page.keyboard.type(character);
        await waitFor(async()=> (await fs.readFile(filename,'utf8')).split('\n')[0]===partial+' needs mitigation.','actual autosave '+partial);
        await fs.copyFile(filename,path.join(output,'autosave-'+partial.length+'.md'));
      }
    } else await page.keyboard.type('Concern',{delay:100});
    await page.keyboard.press('Enter');
    await page.keyboard.press('Control+s');
    const accepted = await capture('01-added-concern',['concern','risk'],'Concern needs mitigation.');

    // Read-only debugger discovery, then an actual exit of only this resource's
    // worker thread. No product hook, fabricated RPC reply or extension-host kill.
    let inspectorTargets;
    await waitFor(async()=>{try{inspectorTargets=await (await fetch(`http://127.0.0.1:${inspectorPort}/json/list`)).json();return inspectorTargets.length>0;}catch{return false;}},'official extension-host inspector');
    const extensionTarget=inspectorTargets[0];
    inspectorSocket=new WebSocket(extensionTarget.webSocketDebuggerUrl);await once(inspectorSocket,'open');
    let debugId=0;const debugPending=new Map(),workerEvents=[],workerReplies=new Map();
    inspectorSocket.addEventListener('message',event=>{
      const packet=JSON.parse(event.data);
      if(packet.id){const pending=debugPending.get(packet.id);if(pending){debugPending.delete(packet.id);packet.error?pending.reject(new Error(JSON.stringify(packet.error))):pending.resolve(packet.result);}}
      if(packet.method==='NodeWorker.attachedToWorker')workerEvents.push(packet.params);
      if(packet.method==='NodeWorker.receivedMessageFromWorker'){
        const message=JSON.parse(packet.params.message);const pending=workerReplies.get(packet.params.sessionId+':'+message.id);
        if(pending){workerReplies.delete(packet.params.sessionId+':'+message.id);message.error?pending.reject(new Error(JSON.stringify(message.error))):pending.resolve(message.result);}
      }
    });
    function debug(method,params={}){return new Promise((resolve,reject)=>{const id=++debugId;debugPending.set(id,{resolve,reject});inspectorSocket.send(JSON.stringify({id,method,params}));});}
    function workerEvaluate(sessionId,expression){const id=++debugId;return new Promise((resolve,reject)=>{
      workerReplies.set(sessionId+':'+id,{resolve,reject});debug('NodeWorker.sendMessageToWorker',{sessionId,message:JSON.stringify({id,method:'Runtime.evaluate',params:{expression,returnByValue:true}})}).catch(reject);
    });}
    await debug('NodeWorker.enable',{waitForDebuggerOnStart:false});
    await waitFor(async()=>workerEvents.length>0,'real worker debugger attachment');
    let resourceWorker,workerIdentity;const discoveredWorkers=[];
    await delay(300);
    for(const attached of workerEvents){
      const result=await workerEvaluate(attached.sessionId,"JSON.stringify({pid:process.pid,threadId:process.getBuiltinModule('worker_threads').threadId,data:process.getBuiltinModule('worker_threads').workerData})");
      discoveredWorkers.push({attached,result});
      if(result.result?.value){const identity=JSON.parse(result.result.value);if(identity.data?.resource?.endsWith('/history.md')){resourceWorker=attached;workerIdentity=identity;break;}}
    }
    await fs.writeFile(path.join(output,'debugger-discovery.json'),JSON.stringify(discoveredWorkers,null,2));
    assert.ok(resourceWorker,'Debugger must identify the real history.md native resource worker');
    assert.ok(workerIdentity.threadId>0);assert.equal(workerIdentity.data.source.split('\n')[0],'Risk needs mitigation.');
    receipt.debugger={extensionTarget:extensionTarget.title,worker:resourceWorker.workerInfo,identity:workerIdentity,termination:'NodeWorker inspector Runtime.evaluate process.exit(73)'};
    await page.keyboard.press('Escape');
    if(await toggle.getAttribute('aria-pressed')==='true')await toggle.click();
    await source.click();await page.keyboard.press('Control+Shift+l');
    await frame.getByRole('button',{name:'Blocks',exact:true}).click();
    await frame.locator('.te-block').first().dblclick();
    const raw=frame.locator('.te-block-editor');
    await raw.waitFor({state:'visible'});
    const pendingText='Pending crash draft — do not discard';
    await raw.fill(pendingText);
    await waitFor(async()=>await page.locator('.tabs-container .tab.dirty').count()>0,'raw Blocks draft marks native resource dirty');
    assert.equal(await fs.readFile(filename,'utf8'),accepted,'raw pending draft is not a main-file save');
    await page.screenshot({path:path.join(output,'02-before-worker-crash.png')});
    await debug('NodeWorker.sendMessageToWorker',{sessionId:resourceWorker.sessionId,message:JSON.stringify({id:++debugId,method:'Runtime.evaluate',params:{expression:'process.exit(73)'}})});
    await frame.getByRole('button',{name:'Recover current file with fresh history',exact:true}).waitFor({state:'visible'});
    await page.screenshot({path:path.join(output,'03-worker-crashed-recovery-required.png')});
    assert.match(await frame.locator('#host-notice').innerText(),/exited \(73\)/);
    assert.equal(await frame.locator('#editor').getAttribute('inert'),'');
    const workerCount=workerEvents.length;await delay(1500);assert.equal(workerEvents.length,workerCount,'failed resource worker does not automatically restart');
    assert.equal(await fs.readFile(filename,'utf8'),accepted);
    receipt.checks.push({id:'02-real-resource-worker-exit',status:'PASS',exitCode:73,noAutomaticRestart:true,extensionHostResponsive:true});
    await frame.getByRole('button',{name:'Open recovery draft',exact:true}).click();
    const archivesDir=path.join(profile,'User/globalStorage/applied-knowledge-systems.terraphim-it/native-authority-recovery');
    let archive,archivePath;
    await waitFor(async()=>{try{for(const name of await fs.readdir(archivesDir)){const candidate=path.join(archivesDir,name);const value=JSON.parse(await fs.readFile(candidate,'utf8'));if(value.drafts?.some(d=>d.draft.includes(pendingText))){archive=value;archivePath=candidate;return true;}}}catch{}return false;},'actual received pending draft recovery archive');
    assert.equal(archive.source,accepted);assert.ok(archive.drafts.length>0);
    await fs.copyFile(archivePath,path.join(output,'04-actual-recovery-archive.json'));
    receipt.checks.push({id:'03-actual-recovery-archive-retains-accepted-and-pending',status:'PASS',sourceSha256:crypto.createHash('sha256').update(archive.source).digest('hex'),drafts:archive.drafts.length});
    await page.getByRole('tab',{name:'history.md',exact:true}).last().click();
    await frame.getByRole('button',{name:'Recover current file with fresh history',exact:true}).click();
    await waitFor(async()=>await frame.locator('#editor').getAttribute('inert')===null&&/Recovered with fresh Undo history/.test(await frame.locator('#host-notice').innerText()),'explicit recovery initializes new authority');
    assert.match(await frame.locator('#editor').innerText(),/Pending crash draft/);
    await frame.locator('#host-notice button').last().click();
    await frame.getByRole('button',{name:'Discard',exact:true}).click();
    await frame.getByRole('button',{name:'Text',exact:true}).click();
    source=frame.getByRole('textbox',{name:'Markdown source',exact:true});
    await source.click();await page.keyboard.press('Control+z');await page.keyboard.press('Control+s');
    assert.equal(await capture('05-explicit-recovery-fresh-history',['concern','risk'],'Concern needs mitigation.'),accepted);
    assert.ok(workerEvents.length>workerCount,'new worker begins only after explicit recovery');
    receipt.resourceWorkerRecovery='PASS';
    receipt.checks.push({id:'04-explicit-recovery-no-prior-session-undo',status:'PASS',pendingDraftRemainsInRecoveryArchive:true});

    receipt.status='PASS';
  } catch (caught) {
    error = caught;
    receipt.status = 'FAIL';
    receipt.error = String(caught.stack || caught);
    await fs.writeFile(path.join(output,'failure-saved.md'),await fs.readFile(filename,'utf8')).catch(()=>{});
    if(page) {
      const visible=page.frames().filter(f=>f.url().includes('fake.html')).at(-1);
      if(visible) await visible.getByRole('textbox',{name:'Markdown source',exact:true}).innerText().then(text=>fs.writeFile(path.join(output,'failure-visible-source.txt'),text)).catch(()=>{});
    }
    if(page) await fs.writeFile(path.join(output,'failure-native-tabs.json'),JSON.stringify(await page.getByRole('tab').evaluateAll(elements=>elements.map(element=>({text:element.textContent,label:element.getAttribute('aria-label'),title:element.getAttribute('title'),class:element.className}))),null,2)).catch(()=>{});
    if (page) await page.screenshot({path:path.join(output,'failure.png')}).catch(() => {});
  } finally {
    if(inspectorSocket)inspectorSocket.close();
    if (browser) await browser.close();
    if (app && app.pid && app.exitCode === null) {
      const exited = once(app,'exit');
      app.kill('SIGTERM');
      const stop = setTimeout(() => app.kill('SIGKILL'), 10000);
      await exited;
      clearTimeout(stop);
    }
    if (log) await log.close();
    await fs.writeFile(path.join(output,'receipt.json'),JSON.stringify(receipt,null,2)+'\n');
    console.log(JSON.stringify({status:receipt.status,output,checks:receipt.checks.length}));
  }
  if (error) throw error;
})().catch(error => { console.error(error); process.exitCode=1; });
