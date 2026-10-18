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
  assert.equal(process.platform, 'darwin', 'This actual isolated native-host regression is macOS-only');
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
  await fs.writeFile(path.join(profile,'User/settings.json'),JSON.stringify({'window.restoreWindows':'all','files.hotExit':'onExitAndWindowClose','window.dialogStyle':'custom'}));
  if (autosave) {
    await fs.mkdir(path.join(profile,'User'),{recursive:true});
    await fs.writeFile(path.join(profile,'User/settings.json'),JSON.stringify({'files.autoSave':'afterDelay','files.autoSaveDelay':200}));
  }
  await fs.mkdir(output, { recursive: true });
  await fs.writeFile(filename, initial, { flag: 'wx' });
  await fs.writeFile(path.join(workspace, 'native.md'), 'Native Markdown control.\n', { flag: 'wx' });
  await fs.writeFile(path.join(output, 'input.md'), initial);
  const receipt = { status: 'RUNNING', scope: 'Actual installed native custom-document SIGKILL hot-exit recovery with passively inspected open context',
    autosaveDuringLiveAddition: autosave, packageSha256: crypto.createHash('sha256').update(await fs.readFile(vsix)).digest('hex'), checks: [] };
  await fs.copyFile(__filename,path.join(output,'executed-harness.cjs'));
  receipt.modes={wholeProcessCrash:true,autosave,split:process.env.TERRAPHIM_HISTORY_SPLIT==='1',external:process.env.TERRAPHIM_HISTORY_EXTERNAL==='1'};
  let inspectorSocket;let app;
  let launchError;
  let browser;
  let page;
  let log;
  let error;
  try {
    const executable = process.env.TERRAPHIM_VSCODE_EXECUTABLE || '/Applications/Visual Studio Code.app/Contents/MacOS/Code';
    const cli = process.env.TERRAPHIM_VSCODE_CLI || '/Applications/Visual Studio Code.app/Contents/Resources/app/bin/code';
    const flags = ['--user-data-dir', profile, '--extensions-dir', extensions, '--disable-gpu',
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
    await page.keyboard.press('Meta+Shift+P');
    await page.getByRole('textbox', {name:'Type the name of a command to run.', exact:true}).fill('>Terraphim: Open Alternative Control');
    await page.getByText('Terraphim: Open Alternative Control', {exact:true}).click();
    let frame;
    await waitFor(async () => {
      frame = page.frames().filter(f => f.url().includes('fake.html')).at(-1);
      return frame && await frame.getByRole('textbox', {name:'Markdown source', exact:true}).isVisible();
    }, 'actual Terraphim webview', 60000);
    let source = frame.getByRole('textbox', {name:'Markdown source', exact:true});
    const toggle = frame.getByTitle('Toggle Write_On mode');
    // Observe the untouched fresh-profile initial state before enabling Write On.
    await waitFor(async()=>(await source.innerText()).trimEnd()===initial.trimEnd(),'fresh native initial body');
    const initialMode={ariaPressed:await toggle.getAttribute('aria-pressed'),counterVisible:await toggle.isVisible(),labVisible:await frame.getByRole('button',{name:'Lab',exact:true}).isVisible(),saveVisible:await frame.getByRole('button',{name:'Save document',exact:true}).isVisible()};
    await fs.writeFile(path.join(output,'00-fresh-profile-initial-mode.json'),JSON.stringify(initialMode,null,2));
    await page.screenshot({path:path.join(output,'00-fresh-profile-initial-mode.png')});
    assert.equal(initialMode.ariaPressed,'false');assert.equal(initialMode.counterVisible,true);assert.equal(initialMode.labVisible,false);assert.equal(initialMode.saveVisible,false);
    await toggle.click();await frame.getByRole('button',{name:'Lab',exact:true}).waitFor({state:'visible'});await frame.getByRole('button',{name:'Save document',exact:true}).waitFor({state:'visible'});
    await page.screenshot({path:path.join(output,'00-counter-click-reveals-controls.png')});
    receipt.checks.push({id:'00-fresh-profile-default-counter-only-and-click-reveals-controls',status:'PASS'});

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
      await page.keyboard.press('Meta+ArrowUp');
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
    await page.keyboard.press('Meta+s');
    const accepted = await capture('01-added-concern',['concern','risk'],'Concern needs mitigation.');

    {

      async function traceRestore(){
       let targets;await waitFor(async()=>{try{targets=await(await fetch(`http://127.0.0.1:${inspectorPort}/json/list`)).json();return targets.length;}catch{return false;}},'restored extension-host inspector',30000);
       inspectorSocket=new WebSocket(targets[0].webSocketDebuggerUrl);await once(inspectorSocket,'open');let next=0;const pending=new Map(),trace=[];
       function call(method,params={}){return new Promise((resolve,reject)=>{const id=++next,timer=setTimeout(()=>{pending.delete(id);reject(Error('Inspector timeout '+method));},10000);pending.set(id,{resolve,reject,timer});inspectorSocket.send(JSON.stringify({id,method,params}));});}
       inspectorSocket.addEventListener('message',async event=>{const message=JSON.parse(event.data);if(message.id){const item=pending.get(message.id);if(item){clearTimeout(item.timer);pending.delete(message.id);message.error?item.reject(Error(JSON.stringify(message.error))):item.resolve(message.result);}}
        if(message.method==='Debugger.paused'){
         try{for(const frame of message.params.callFrames.filter(f=>f.functionName==='openCustomDocument')){
          const result=await call('Debugger.evaluateOnCallFrame',{callFrameId:frame.callFrameId,expression:'JSON.stringify((()=>{const state={options,uri:uri.toString()};try{state.disk=disk}catch{}try{state.recovery=recovery}catch{}try{state.source=source}catch{}return state})())',returnByValue:true});trace.push({location:frame.location,result});await fs.writeFile(path.join(output,'08-open-context-inspector.json'),JSON.stringify(trace,null,2));
         }}finally{await call('Debugger.resume').catch(()=>undefined);}
        }
       });
       await call('Debugger.enable');const lines=(await fs.readFile(path.join(installedRoot,'out/alternativeControl.js'),'utf8')).split('\n');
       const at=lines.findIndex(line=>line.includes('async openCustomDocument('));assert.ok(at>0);
       for(const lineNumber of [at+1,lines.findIndex((line,index)=>index>at&&line.includes('this.checkSize(source)'))]){
        assert.ok(lineNumber>at);await call('Debugger.setBreakpointByUrl',{urlRegex:'alternativeControl\\.js$',lineNumber});
       }
       await call('Runtime.runIfWaitingForDebugger');
      }
      // This is a separate abrupt whole-application crash after healthy explicit
      // resource recovery, using VS Code's real custom-document backup service.

      // Establish a real persisted custom-editor layout using normal app Quit,
      // then verify the same profile actually restores it before fault injection.
      if(inspectorSocket)inspectorSocket.close();inspectorSocket=undefined;
      const seedBrowser=browser;browser=undefined;const seedExit=once(app,'exit');
      await page.keyboard.press('Meta+q');await Promise.race([seedExit,delay(15000).then(()=>{throw Error('Normal layout-seeding Quit did not finish');})]);await seedBrowser.close().catch(()=>undefined);
      app=spawn(executable,[...flags,`--remote-debugging-port=${port}`],{env,stdio:['ignore',log.fd,log.fd]});
      await waitFor(async()=>{try{return(await fetch(`http://127.0.0.1:${port}/json/version`)).ok;}catch{return false;}},'normal persisted-session restart',60000);
      browser=await chromium.connectOverCDP(`http://127.0.0.1:${port}`);await waitFor(async()=>browser.contexts()[0].pages().length>0,'seeded workbench');page=browser.contexts()[0].pages()[0];page.setDefaultTimeout(15000);
      await waitFor(async()=>{for(const candidate of page.frames().filter(f=>f.url().includes('fake.html'))){const input=candidate.getByRole('textbox',{name:'Markdown source',exact:true});if(await input.isVisible()&&(await input.innerText()).startsWith('Concern needs mitigation.')){frame=candidate;source=input;return true;}}return false;},'actual normally persisted custom-editor layout restored',60000);
      const persistedWindowState=JSON.parse(await fs.readFile(path.join(profile,'User/globalStorage/storage.json'),'utf8'));
      await fs.writeFile(path.join(output,'06-pre-crash-persisted-window-state.json'),JSON.stringify(persistedWindowState,null,2));
      receipt.checks.push({id:'06-normal-custom-editor-session-restores-before-fault',status:'PASS'});
      const tail='Unsaved full-process recovery sentence.';
      await source.click();await page.keyboard.press('Meta+ArrowDown');await page.keyboard.type(tail,{delay:15});
      await waitFor(async()=>await page.locator('.tabs-container .tab.dirty').count()>0&&(await source.innerText()).includes(tail),'healthy native accepted unsaved edit');
      let nativeBackup,nativeBackupPath;
      async function backupFiles(directory,depth=0){let entries;try{entries=await fs.readdir(directory,{withFileTypes:true});}catch{return [];}
        const found=[];for(const entry of entries){const full=path.join(directory,entry.name);if(entry.isDirectory()&&depth<8)found.push(...await backupFiles(full,depth+1));else if(entry.isFile())found.push(full);}return found;}
      await waitFor(async()=>{
        for(const file of [...await backupFiles(path.join(profile,'Backups')),...await backupFiles(path.join(profile,'User/workspaceStorage')),...await backupFiles(path.join(profile,'User/globalStorage'))]){
          try{const stat=await fs.stat(file);if(stat.size>2*1024*1024)continue;const content=await fs.readFile(file,'utf8');const at=content.indexOf('{"kind":"terraphim-resource-backup"');if(at<0)continue;
            const value=JSON.parse(content.slice(at));if(value.source?.includes(tail)){nativeBackup=value;nativeBackupPath=file;return true;}}
          catch{}
        }return false;
      },'real VS Code recovery backup contains accepted unsaved source',45000);
      let backupReference,backupReferencePath;
      await waitFor(async()=>{for(const file of await backupFiles(path.join(profile,'Backups'))){try{const value=await fs.readFile(file,'utf8');if(value.includes(nativeBackupPath)&&value.includes('terraphimIt.alternativeControl')){backupReference=value;backupReferencePath=file;return true;}}catch{}}return false;},'native workbench persists custom-editor backup identity',30000);
      await fs.copyFile(backupReferencePath,path.join(output,'06-native-backup-reference.txt'));
      receipt.nativeBackupReference={path:backupReferencePath,backupId:nativeBackupPath,sha256:crypto.createHash('sha256').update(backupReference).digest('hex')};
      assert.equal(await fs.readFile(filename,'utf8'),accepted,'unsaved edit never falsely presented as main-file save');
      await fs.copyFile(nativeBackupPath,path.join(output,'06-actual-native-hot-exit-backup'));
      await fs.writeFile(path.join(output,'06-decoded-native-backup.json'),JSON.stringify(nativeBackup,null,2));
      await page.screenshot({path:path.join(output,'06-before-process-crash.png')});
      receipt.beforeProcessCrashTabs=await page.getByRole('tab').evaluateAll(elements=>elements.map(element=>({text:element.textContent,label:element.getAttribute('aria-label'),class:element.className})));
      assert.ok(receipt.beforeProcessCrashTabs.some(tab=>tab.class?.includes('dirty')),'actual custom working copy remains dirty immediately before fault');
      if(inspectorSocket)inspectorSocket.close();inspectorSocket=undefined;const priorBrowser=browser;browser=undefined;
      const priorExit=once(app,'exit');assert.equal(app.kill('SIGKILL'),true);const [exitCode,exitSignal]=await priorExit;assert.equal(exitSignal,'SIGKILL');
      receipt.processCrashExit={exitCode,exitSignal};await priorBrowser.close().catch(()=>undefined);
      receipt.checks.push({id:'06-actual-whole-app-sigkill-with-native-backup',status:'PASS',signal:'SIGKILL',backupSourceSha256:crypto.createHash('sha256').update(nativeBackup.source).digest('hex')});
      app=spawn(executable,[...flags,`--remote-debugging-port=${port}`,`--inspect-extensions=${inspectorPort}`],{env,stdio:['ignore',log.fd,log.fd]});
      await traceRestore();
      await waitFor(async()=>{try{return(await fetch(`http://127.0.0.1:${port}/json/version`)).ok;}catch{return false;}},'restarted actual VS Code after SIGKILL',60000);
      browser=await chromium.connectOverCDP(`http://127.0.0.1:${port}`);await waitFor(async()=>browser.contexts()[0].pages().length>0,'restarted workbench');page=browser.contexts()[0].pages()[0];page.setDefaultTimeout(15000);
      let nativeRestored=false;
      await waitFor(async()=>{for(const candidate of page.frames().filter(f=>f.url().includes('fake.html'))){const input=candidate.getByRole('textbox',{name:'Markdown source',exact:true});if(await input.isVisible()&&(await input.innerText()).includes(tail)){nativeRestored=true;return true;}}
       return await page.getByRole('button',{name:'Recover backup',exact:true}).count()>0||await page.getByRole('option').filter({hasText:'Recover backup'}).count()>0;
      },'native backup recovery or explicit retained-backup choice',60000);
      if(!nativeRestored){const recoverButton=page.getByRole('button',{name:'Recover backup',exact:true});
       if(await recoverButton.count())await recoverButton.click();else await page.getByRole('option').filter({hasText:'Recover backup'}).first().click();
       receipt.recoveryAdmission='Actual explicit retained-backup choice selected';
      }else receipt.recoveryAdmission='Actual unsaved source restored before a fallback choice was observed; native open-context not inferred';
      receipt.checks.push({id:'07-actual-recovery-admission-observed',status:'PASS',route:receipt.recoveryAdmission});
      await waitFor(async()=>{for(const candidate of page.frames().filter(f=>f.url().includes('fake.html'))){const input=candidate.getByRole('textbox',{name:'Markdown source',exact:true});if(await input.isVisible()&&(await input.innerText()).includes(tail)){frame=candidate;source=input;return true;}}return false;},'native hot-exit restores actual unsaved accepted source',60000);
      assert.equal(await fs.readFile(filename,'utf8'),accepted);
      await page.screenshot({path:path.join(output,'07-restored-after-process-crash.png')});
      await source.click();await page.keyboard.press('Meta+z');await page.keyboard.press('Meta+s');
      await waitFor(async()=>await fs.readFile(filename,'utf8')===nativeBackup.source&&await page.locator('.tabs-container .tab.dirty').count()===0,'fresh recovered history saves exact backup bytes');
      await fs.copyFile(filename,path.join(output,'07-actual-process-recovered-saved.md'));
      const graph=await exportKnowledgeGraph(await fs.readFile(filename,'utf8'),installedAssets);const graphData=JSON.parse(graph);assert.equal(graphData.data.risk.id,graphData.data.concern.id);await fs.writeFile(path.join(output,'07-actual-process-recovered-graph.json'),graph);
      receipt.checks.push({id:'07-native-backup-restored-fresh-history-saved',status:'PASS',sourceSha256:crypto.createHash('sha256').update(await fs.readFile(filename)).digest('hex')});
    }
    receipt.status='PASS';
  } catch (caught) {
    error = caught;
    receipt.status = 'FAIL';
    receipt.error = String(caught.stack || caught);
    if(receipt.nativeBackupReference){for(const [label,file] of Object.entries({nativeReference:receipt.nativeBackupReference.path,providerPayload:receipt.nativeBackupReference.backupId})){try{await fs.copyFile(file,path.join(output,'09-post-restart-'+label));}catch(error){receipt['postRestart'+label]=String(error);}}}
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
