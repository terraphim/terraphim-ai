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
const { chromium, _electron } = require('playwright-core');

async function waitFor(check, description, timeout = 15000) {
  const until = Date.now() + timeout;
  while (Date.now() < until) {
    if (await check()) return;
    await delay(100);
  }
  throw new Error('Timed out: ' + description);
}

(async () => {
  assert.equal(process.platform, 'darwin', 'This actual native-host harness is macOS-only');
  const selectedScenarioPlan=require('./nativeScenarioPreflight.cjs').nativeScenarioPreflight(process.env);
  if(selectedScenarioPlan.groups.length){const matrix=JSON.parse(await fs.readFile(process.env.TERRAPHIM_MATRIX_TEMPLATE,'utf8'));assert.ok(Array.isArray(matrix.rows)&&matrix.rows.length===41&&new Set(matrix.rows.map(r=>r.id)).size===41,'Native preflight refused: matrix must contain41unique requirement rows');}
  const root = path.resolve(__dirname, '..');
  const run = await fs.mkdtemp(path.join(os.tmpdir(), 'ts-'));
  const output = path.resolve(process.env.TERRAPHIM_HISTORY_OUTPUT || path.join(root, 'native-results', path.basename(run)));
  const profile = path.join(run, 'profile');
  const extensions = path.join(run, 'extensions');
  const workspace = path.join(run, 'workspace');
  const filename = path.join(workspace, 'history.md');
  const initial = process.env.TERRAPHIM_MAC_GHOST_REOPEN_INPUT ? await fs.readFile(process.env.TERRAPHIM_MAC_GHOST_REOPEN_INPUT,'utf8') : 'Risk needs mitigation.\n\nThe editor is basically the only target here today.\n' + (process.env.TERRAPHIM_MAC_ALL41==='1' ? '\n# Launch decision\n\nThe team will review the supplier plan on Monday.\n\nWe keep a paperclip beside the project brief. Another choice is available.\n\nThis is teh teh plan , with alot of detail. The team maybe basically really just sort of hopes the plan might possibly work. This bloody awesome solution is super cool. Although the team, which (because the plan (while incomplete) is changing) has concerns, will proceed; however, unless the supplier agrees, the launch cannot continue: this is conditional. In order to facilitate the implementation of the appropriate operational mechanism, which may have been considered by the relevant stakeholders in relation to the proposed initiative, the committee will undertake a comprehensive review of every single aspect of the process before making any kind of final decision about whether to proceed with the project.\n' : '');
  const vsix = path.resolve(process.env.TERRAPHIM_HISTORY_VSIX || path.join(root, 'terraphim-it.vsix'));
  await fs.mkdir(workspace);
  const autosave = process.env.TERRAPHIM_HISTORY_AUTOSAVE === '1';
  {
    await fs.mkdir(path.join(profile,'User'),{recursive:true});
    await fs.writeFile(path.join(profile,'User/settings.json'),JSON.stringify({'update.mode':'none','files.simpleDialog.enable':true,...((process.env.TERRAPHIM_MAC_CLIPBOARD_FOCUS_GATE==='1'||process.env.TERRAPHIM_MAC_ELECTRON_FOCUS==='1')?{'window.title':'Terraphim Clipboard Verification ${activeEditorShort}'}:{}),...(autosave?{'files.autoSave':'afterDelay','files.autoSaveDelay':200}:{})}));
  }
  await fs.mkdir(output, { recursive: true });
  await fs.writeFile(filename, initial, { flag: 'wx' });
  await fs.writeFile(path.join(workspace, 'native.md'), 'Native Markdown control.\n', { flag: 'wx' });
  await fs.writeFile(path.join(output, 'input.md'), initial);
  const receipt = { status: 'RUNNING', scope: 'Actual isolated macOS native editor lifetime, alternatives and protected Ghost history; explicitly authorized Playwright/CDP',
    run,workspace,filename,profile,extensions,selectedScenarioPlan,executedCommand:{node:process.execPath,script:__filename,selectedGroups:process.env.TERRAPHIM_SCENARIO_GROUPS||null,all41:process.env.TERRAPHIM_MAC_ALL41==='1',overflowOnly:process.env.TERRAPHIM_MAC_OVERFLOW_ONLY==='1'},autosaveDuringLiveAddition: autosave, packageSha256: crypto.createHash('sha256').update(await fs.readFile(vsix)).digest('hex'), checks: [] };
  await fs.copyFile(__filename,path.join(output,'executed-harness.cjs'));
  receipt.modes={autosave,split:process.env.TERRAPHIM_HISTORY_SPLIT==='1',external:process.env.TERRAPHIM_HISTORY_EXTERNAL==='1'};
  let inspectorSocket;let debug;const flushSnapshots=[];
  let app;
  let launchError;
  let browser;
  let electronApp;
  let focusOwnedWindow;
  let page;
  let log;
  let error;
  try {
    const executable = process.env.TERRAPHIM_VSCODE_EXECUTABLE || '/Applications/Visual Studio Code.app/Contents/MacOS/Code';
    const cli = process.env.TERRAPHIM_VSCODE_CLI || '/Applications/Visual Studio Code.app/Contents/Resources/app/bin/code';
    const flags = ['--user-data-dir', profile, '--extensions-dir', extensions, '--disable-gpu',
      '--skip-welcome', '--skip-release-notes'];
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
    let inspectorPort;if(process.env.TERRAPHIM_MAC_FLUSH_TRACE==='1'){const reservation=net.createServer();reservation.listen(0,'127.0.0.1');await once(reservation,'listening');inspectorPort=reservation.address().port;await new Promise(resolve=>reservation.close(resolve));}
    log = await fs.open(path.join(output, 'host.log'), 'wx');
    if(process.env.TERRAPHIM_MAC_ELECTRON_FOCUS==='1'){
      electronApp=await _electron.launch({executablePath:executable,args:[...flags,workspace,filename],env,timeout:20000});app=electronApp.process();page=await electronApp.firstWindow({timeout:20000});
      focusOwnedWindow=async()=>{await electronApp.evaluate(({app})=>app.focus({steal:true}));const handle=await electronApp.browserWindow(page);const proof=await handle.evaluate(window=>{window.show();window.focus();return{title:window.getTitle(),isFocused:window.isFocused()};});assert.ok(proof.title.startsWith('Terraphim Clipboard Verification'),'publicElectronwindowmustbeownedtesttitle');receipt.publicElectronFocus=proof;await fs.writeFile(path.join(output,'public-electron-focus.json'),JSON.stringify(proof,null,2));};
    }else{
    app = spawn(executable, [...flags, `--remote-debugging-port=${port}`,...(inspectorPort?[`--inspect-extensions=${inspectorPort}`]:[]), workspace, filename],
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
    }
    page.setDefaultTimeout(15000);
    await page.bringToFront();
    if(inspectorPort){
      let targets;await waitFor(async()=>{try{targets=await(await fetch(`http://127.0.0.1:${inspectorPort}/json/list`)).json();return targets.length>0;}catch{return false;}},'owned extension debugger',30000);
      inspectorSocket=new WebSocket(targets[0].webSocketDebuggerUrl);await once(inspectorSocket,'open');let debugId=0;const waiting=new Map();
      debug=(method,params={})=>new Promise((resolve,reject)=>{const id=++debugId;waiting.set(id,{resolve,reject});inspectorSocket.send(JSON.stringify({id,method,params}));});
      inspectorSocket.addEventListener('message',event=>{const packet=JSON.parse(event.data);if(packet.id){const item=waiting.get(packet.id);if(item){waiting.delete(packet.id);packet.error?item.reject(new Error(JSON.stringify(packet.error))):item.resolve(packet.result);}}
       if(packet.method==='Debugger.paused')void(async()=>{try{const frame=packet.params.callFrames[0];const read=await debug('Debugger.evaluateOnCallFrame',{callFrameId:frame.callFrameId,expression:"({message:{revision:message.revision,sourceHash:message.sourceHash,historyGeneration:message.historyGeneration,nonce:message.nonce},document:{revision:doc.revision,sourceHash:hash(doc.source),historyGeneration:doc.historyGeneration},viewSession:view.session,pendingSource:Boolean(doc.pendingAuthority),draftSequences:[...doc.drafts.values()].map(d=>d.sequence)})",returnByValue:true});flushSnapshots.push({at:new Date().toISOString(),location:frame.location,read});await fs.writeFile(path.join(output,'flush-mismatch-inspector.json'),JSON.stringify(flushSnapshots,null,2));}finally{await debug('Debugger.resume');}})().catch(error=>{flushSnapshots.push({error:String(error)});});
      });await debug('Runtime.runIfWaitingForDebugger');await debug('Debugger.enable');
    }
    if(executable.includes('Devin.app')){
      const onboarding=page.locator('article.windsurf-onboarding');await onboarding.waitFor({state:'visible',timeout:10000}).catch(()=>{});
      if(await onboarding.isVisible()){
        await page.getByText('Customize',{exact:true}).waitFor({state:'visible',timeout:10000}).catch(()=>{});
        await fs.writeFile(path.join(output,'onboarding-before.txt'),await page.locator('body').innerText());await page.screenshot({path:path.join(output,'onboarding-before.png')});
        if(await page.getByText('Customize',{exact:true}).isVisible()){
          const boxes=onboarding.locator('input[type="checkbox"]');assert.equal(await boxes.count(),1,'observed terminal install checkbox');if(await boxes.isChecked())await onboarding.getByText(/Install.*terminal command/).click();assert.equal(await boxes.isChecked(),false,'global terminal install disabled');await page.getByRole('button',{name:'Next',exact:true}).click();await delay(400);
          await fs.writeFile(path.join(output,'onboarding-after-customize.txt'),await page.locator('body').innerText());await page.screenshot({path:path.join(output,'onboarding-after-customize.png')});
        }
      }
    }
    await fs.writeFile(path.join(output,'initial-native-ax.txt'),await page.locator('body').innerText());
    const trust=page.getByRole('button',{name:/Yes, I trust the authors/});if(await trust.isVisible())await trust.click();
    execFileSync(cli,[...flags,'--reuse-window',filename],{env,encoding:'utf8',timeout:60000});
    await page.bringToFront();await waitFor(()=>page.evaluate(()=>document.hasFocus()),'owned native window focused before command palette');
    await page.getByRole('tab', { name:'history.md', exact:true }).click();
    await page.keyboard.press('Meta+Shift+P');
    const initialPalette=page.getByRole('textbox',{name:'Type the name of a command to run.',exact:true});await initialPalette.focus();await waitFor(()=>initialPalette.evaluate(el=>document.activeElement===el),'owned command palette input focus');await initialPalette.fill('>Terraphim: Open Alternative Control');assert.equal(await initialPalette.inputValue(),'>Terraphim: Open Alternative Control');
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
    if(debug){const lines=(await fs.readFile(path.join(installedRoot,'out/alternativeControl.js'),'utf8')).split('\n');const lineNumber=lines.findIndex(line=>line.includes("pending.reject(new Error('Snapshot changed before save'))"));assert.ok(lineNumber>=0);const set=await debug('Debugger.setBreakpointByUrl',{urlRegex:'alternativeControl\\.js$',lineNumber});receipt.flushTrace={readOnly:true,lineNumber,breakpoint:set};}
    const installedAssets=path.join(installedRoot,'media/editor');
    const { exportKnowledgeGraph, checkpointProjection } = require(path.join(installedRoot,'media/knowledge-graph.cjs'));
    async function capture(name, expectedTerms, expectedBody, expectedSource) {
      let saved;
      try { await waitFor(async () => {
        saved = await fs.readFile(filename, 'utf8');
        const graph = JSON.parse(await exportKnowledgeGraph(saved, installedAssets));
        return (await source.innerText()).split('\n')[0] === expectedBody &&
          await page.locator('.tabs-container .tab.dirty').count() === 0 &&
          saved.split('\n')[0] === expectedBody && (expectedSource===undefined||saved===expectedSource) &&
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
    if(process.env.TERRAPHIM_MAC_GHOST_REOPEN_INPUT){
      const projection=await checkpointProjection(initial,installedAssets),block=initial.match(/```terraphim-alternatives\n([\s\S]*?)\n```\s*$/);assert.ok(block);const a=JSON.parse(block[1]);const live=a.ghosts.filter(g=>projection.body.slice(g.anchor.start,g.anchor.end)===g.anchor.text);assert.ok(live.length);const mode=frame.getByTitle('Toggle Write_On mode');if(await mode.getAttribute('aria-pressed')!=='true')await mode.click();await waitFor(async()=>(await source.textContent())===projection.body,'actual fresh native reopened Ghost document body');const rendered=await source.locator('.te-ghost').evaluateAll(items=>items.map(el=>({text:el.textContent,tokens:el.getAttribute('data-te-decoration'),color:getComputedStyle(el).color})));for(const ghost of live)assert.ok(rendered.some(x=>x.tokens.split(' ').includes('ghosts:'+ghost.id)&&x.text===ghost.anchor.text));assert.ok(!rendered.some(x=>x.text==='editor'),'setaside deleted Ghost not attached to unrelated text');await page.screenshot({path:path.join(output,'fresh-ghost-reopened.png')});await fs.writeFile(path.join(output,'fresh-ghost-reopened.native.txt'),await frame.locator('body').innerText());await page.keyboard.press('Meta+s');await waitFor(async()=>await page.locator('.tabs-container .tab.dirty').count()===0,'actual reopened Ghost source native Save');const actual=await fs.readFile(filename,'utf8');assert.equal(actual,initial,'fresh actual native reopen/save preserves source bytes');await fs.writeFile(path.join(output,'fresh-ghost-reopened.saved.md'),actual);await fs.writeFile(path.join(output,'fresh-ghost-rendered.json'),JSON.stringify({inputActualSavedSource:process.env.TERRAPHIM_MAC_GHOST_REOPEN_INPUT,liveGhosts:live,rendered},null,2));receipt.checks.push({id:'actual-saved-ghost-fresh-native-reopen',status:'PASS',sourceSha256:crypto.createHash('sha256').update(actual).digest('hex')});receipt.status='PASS';return;
    }
    if(process.env.TERRAPHIM_MAC_OVERFLOW_ONLY==='1'){
      await fs.copyFile(path.join(__dirname,'native41Scenarios.cjs'),path.join(output,'executed-scenarios.cjs'));const scenarios=require('./native41Scenarios.cjs');const rounds=Number(process.env.TERRAPHIM_OVERFLOW_ROUNDS||5);assert.ok(Number.isInteger(rounds)&&rounds>=1&&rounds<=5);for(let round=1;round<=rounds;round++){const own=path.join(output,'round-'+round);await fs.mkdir(own);const matrix=await scenarios({page,frame,source,workspace,output:own,filename,host:receipt.hostVersion,packageSha256:receipt.packageSha256,focusOwnedWindow,matrixTemplate:process.env.TERRAPHIM_MATRIX_TEMPLATE});receipt.checks.push({id:'overflow-typing-round-'+round,status:matrix.rows.find(x=>x.id==='R-6.2').status,matrix:'round-'+round+'/all41-matrix.json'});}if(process.env.TERRAPHIM_MAC_OVERFLOW_REOPEN==='1'){
        const persisted=await fs.readFile(filename,'utf8');const encoded=persisted.match(/```terraphim-alternatives\n([\s\S]*?)\n```\s*$/);assert.ok(encoded);const expectedOverflow=JSON.parse(encoded[1]).overflow;assert.ok(expectedOverflow.includes('Public note https://terraphim.ai'));await fs.writeFile(path.join(output,'overflow-before-close.saved.md'),persisted);
        await page.keyboard.press('Meta+w');await page.keyboard.press('Meta+Shift+e');await page.getByLabel('Files Explorer',{exact:true}).getByText('history.md',{exact:true}).dblclick();await page.keyboard.press('Meta+Shift+p');await page.getByRole('textbox',{name:'Type the name of a command to run.',exact:true}).fill('>Terraphim: Open Alternative Control');await page.getByText('Terraphim: Open Alternative Control',{exact:true}).click();await waitFor(async()=>{frame=page.frames().filter(f=>f.url().includes('fake.html')).at(-1);return frame&&await frame.getByRole('textbox',{name:'Markdown source',exact:true}).isVisible();},'native reopened Overflow custom view',60000);source=frame.getByRole('textbox',{name:'Markdown source',exact:true});await waitFor(async()=>(await source.textContent()).trimEnd()===persisted.split('```terraphim-alternatives')[0].trimEnd(),'native reopened actual body');const mode=frame.getByTitle('Toggle Write_On mode');if(await mode.getAttribute('aria-pressed')!=='true')await mode.click();const area=frame.getByRole('textbox',{name:'Overflow text',exact:true});if(!await area.isVisible())await frame.getByRole('button',{name:'Overflow panel',exact:true}).click();assert.equal(await area.inputValue(),expectedOverflow,'actual reopened Overflow matches full saved text');await page.screenshot({path:path.join(output,'overflow-reopened.png')});await fs.writeFile(path.join(output,'overflow-reopened.native.txt'),await frame.locator('body').innerText());await page.keyboard.press('Meta+s');await waitFor(async()=>await page.locator('.tabs-container .tab.dirty').count()===0,'native reopened save settled');assert.equal(await fs.readFile(filename,'utf8'),persisted);await fs.writeFile(path.join(output,'overflow-after-reopened-save.saved.md'),await fs.readFile(filename,'utf8'));receipt.checks.push({id:'overflow-actual-save-close-reopen',status:'PASS',sha256:crypto.createHash('sha256').update(persisted).digest('hex')});
      }
      receipt.status=receipt.checks.every(x=>x.status==='PASS')?'PASS':receipt.checks.some(x=>x.status==='FAIL')?'FAIL':'INCOMPLETE';return;
    }
    if(process.env.TERRAPHIM_MAC_PLAIN_INPUT==='1'){
      await page.bringToFront();await source.press('Meta+ArrowUp');const steps=[];for(const key of ['A','Enter','B','Enter']){if(key==='Enter')await page.keyboard.press('Enter');else await page.keyboard.type(key);steps.push({key,body:await source.textContent(),focus:await source.evaluate(el=>({documentFocused:document.hasFocus(),sourceFocused:document.activeElement===el}))});await fs.writeFile(path.join(output,'plain-input-steps.json'),JSON.stringify(steps,null,2));}
      await page.keyboard.press('Meta+s');await waitFor(async()=>{const text=await fs.readFile(filename,'utf8');return text.startsWith('A\nB\nRisk needs mitigation.');},'real native plain Enter output');const actual=await fs.readFile(filename,'utf8');await fs.writeFile(path.join(output,'plain-enter.saved.md'),actual);await page.screenshot({path:path.join(output,'plain-enter.png')});receipt.checks.push({id:'native-plain-enter-two-lines',status:'PASS',sha256:crypto.createHash('sha256').update(actual).digest('hex')});receipt.status='PASS';return;
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
    await page.keyboard.press('Escape');
    await source.click({position:{x:12,y:12}});
    await page.keyboard.press('Meta+z');
    await page.keyboard.press('Meta+s');
    const undone = await capture('02-whole-undo',[],'Risk needs mitigation.',initial);
    assert.equal(undone,initial,'whole Undo must restore exact original bytes');
    await page.keyboard.press('Meta+Shift+z');
    await page.keyboard.press('Meta+s');
    assert.equal(await capture('03-whole-redo',['concern','risk'],'Concern needs mitigation.',accepted),accepted);

    // A remains unsaved while B is typed and cancelled; a following Redo may
    // never revive B. All observations are real UI and filesystem readbacks.
    await page.keyboard.press('Meta+z');
    await chooseRisk();
    await frame.locator('.te-alt-input--new').click();
    await page.keyboard.type('Worry',{delay:80});
    await page.keyboard.press('Enter');
    const newAlternative = frame.locator('.te-alt-input--new');
    await newAlternative.click();
    await page.keyboard.type('Hazard',{delay:80});
    await page.keyboard.press('Escape');
    await page.keyboard.press('Meta+s');
    const canceled = await capture('04-escape-preserves-A',['risk','worry'],'Worry needs mitigation.');
    await source.click({position:{x:12,y:12}});
    await page.keyboard.press('Meta+Shift+z');
    await page.keyboard.press('Meta+s');
    assert.equal(await capture('05-redo-cannot-revive-B',['risk','worry'],'Worry needs mitigation.',canceled),canceled);
    await page.keyboard.press('Meta+w');
    await page.keyboard.press('Meta+Shift+E');
    await page.getByLabel('Files Explorer',{exact:true}).getByText('history.md',{exact:true}).dblclick();
    await page.keyboard.press('Meta+Shift+P');
    await page.getByRole('textbox',{name:'Type the name of a command to run.',exact:true}).fill('>Terraphim: Open Alternative Control');
    await page.getByText('Terraphim: Open Alternative Control',{exact:true}).click();
    await waitFor(async()=>{
      frame=page.frames().filter(f=>f.url().includes('fake.html')).at(-1);
      return frame && await frame.getByRole('textbox',{name:'Markdown source',exact:true}).isVisible();
    },'actual saved document reopen',60000);
    source=frame.getByRole('textbox',{name:'Markdown source',exact:true});
    await waitFor(async()=>(await source.innerText()).split('\n')[0]==='Worry needs mitigation.','reopened accepted document content',60000);
    assert.equal(await capture('06-saved-reopen',['risk','worry'],'Worry needs mitigation.',canceled),canceled);


    assert.notEqual(process.env.TERRAPHIM_HISTORY_EXTERNAL,'1','Authority receipt sequence uses its independent profile');
    assert.notEqual(process.env.TERRAPHIM_HISTORY_SPLIT,'1','Authority receipt sequence creates and tracks its own split views');
    // Swap an existing alternative through the real panel before the protected
    // receipt test. Saved output, not the requested input, is the assertion.
    await source.click({position:{x:12,y:12}});
    await page.keyboard.press('Meta+ArrowUp');
    await page.keyboard.down('Shift');
    for(let i=0;i<5;i++)await page.keyboard.press('ArrowRight');
    await page.keyboard.up('Shift');
    assert.equal(await frame.evaluate(()=>getSelection().toString()),'Worry');
    await page.keyboard.press('Control+Shift+A');
    await frame.locator('.te-alt-line[data-index="1"] input').click();
    await page.keyboard.press('ArrowUp');
    await waitFor(async()=>(await source.innerText()).split('\n')[0]==='Risk needs mitigation.','actual original swap visible');
    await page.keyboard.press('Escape');
    await page.keyboard.press('Meta+s');
    const swapped = await capture('07-native-swap-original',['risk','worry'],'Risk needs mitigation.');
    await focusView(frame);
    await page.keyboard.press('Meta+z');
    await page.keyboard.press('Meta+s');
    assert.equal(await capture('08-native-swap-undo',['risk','worry'],'Worry needs mitigation.',canceled),canceled);
    await page.keyboard.press('Meta+Shift+z');
    await page.keyboard.press('Meta+s');
    assert.equal(await capture('09-native-swap-redo',['risk','worry'],'Risk needs mitigation.',swapped),swapped);
    await page.keyboard.press('Meta+z');
    await page.keyboard.press('Meta+s');
    assert.equal(await capture('10-native-swap-restored',['risk','worry'],'Worry needs mitigation.',canceled),canceled);

    if(process.env.TERRAPHIM_MAC_ALL41==='1'){
      await fs.copyFile(path.join(__dirname,'native41Scenarios.cjs'),path.join(output,'executed-scenarios.cjs'));const matrix=await require('./native41Scenarios.cjs')({page,frame,source,workspace,output,filename,host:receipt.hostVersion,packageSha256:receipt.packageSha256,focusOwnedWindow,matrixTemplate:process.env.TERRAPHIM_MATRIX_TEMPLATE});
      receipt.matrixSummary=matrix.summary;receipt.status=matrix.summary.NOT_RUN||matrix.summary.PARTIAL||matrix.summary.FAIL?'SCENARIOS_COMPLETED_WITH_UNRESOLVED_ROWS':'PASS';
    } else {
    // A real protected promotion must retain its private native receipt after a
    // second view attaches and after the issuing view is disposed.
    const priorGhosts = canceled;
    const currentToggle = frame.getByTitle('Toggle Write_On mode');
    if (await currentToggle.getAttribute('aria-pressed') !== 'true') await currentToggle.click();
    await frame.getByRole('button',{name:'Lab',exact:true}).click();
    await frame.getByRole('radio',{name:/Even sharper/}).click();
    await frame.getByRole('button',{name:'Done',exact:true}).click();
    await focusView(frame);
    await page.keyboard.press('Meta+s');
    function persistedGhosts(saved) {
      const marker='```terraphim-alternatives\n';
      const at=saved.lastIndexOf(marker);
      if(at<0)return [];
      const end=saved.indexOf('\n```',at+marker.length);
      assert.ok(end>at,'Real saved annotation fence must close');
      return JSON.parse(saved.slice(at+marker.length,end)).ghosts;
    }
    let promoted;
    await waitFor(async()=>{
      promoted=await fs.readFile(filename,'utf8');
      return await page.locator('.tabs-container .tab.dirty').count()===0 && persistedGhosts(promoted).length>0;
    },'actual protected Ghost promotion persisted to disk',30000);
    const expectedGhosts=persistedGhosts(promoted);
    await fs.writeFile(path.join(output,'11-native-promoted-ghosts.md'),promoted);
    await fs.writeFile(path.join(output,'11-native-issued-ghosts.json'),JSON.stringify(expectedGhosts,null,2)+'\n');
    receipt.checks.push({id:'11-native-protected-ghosts-saved',status:'PASS',ghostIds:expectedGhosts.map(g=>g.id),sourceSha256:crypto.createHash('sha256').update(promoted).digest('hex')});
    const issuingFrame=frame;
    await source.click({position:{x:12,y:12}});
    await page.keyboard.press('Meta+\\');
    let survivingFrame;
    await waitFor(async()=>{
      for(const candidate of page.frames().filter(f=>f.url().includes('fake.html')&&f!==issuingFrame)) {
        if(await candidate.getByRole('textbox',{name:'Markdown source',exact:true}).isVisible())survivingFrame=candidate;
      }
      return !!survivingFrame;
    },'new native view attaches to protected receipt owner',60000);
    frame=survivingFrame;source=frame.getByRole('textbox',{name:'Markdown source',exact:true});
    await focusView(frame);
    await page.keyboard.press('Meta+z');
    await page.keyboard.press('Meta+s');
    assert.equal(await capture('12-ghost-other-view-undo',['risk','worry'],'Worry needs mitigation.',priorGhosts),priorGhosts);
    await page.keyboard.press('Meta+Shift+z');
    await page.keyboard.press('Meta+s');
    assert.equal(await capture('13-ghost-other-view-redo',['risk','worry'],'Worry needs mitigation.',promoted),promoted);
    await focusView(issuingFrame);
    await page.keyboard.press('Meta+w');
    frame=survivingFrame;source=frame.getByRole('textbox',{name:'Markdown source',exact:true});
    await focusView(frame);
    await page.keyboard.press('Meta+z');
    await page.keyboard.press('Meta+s');
    assert.equal(await capture('14-ghost-issuing-view-closed-undo',['risk','worry'],'Worry needs mitigation.',priorGhosts),priorGhosts);
    await page.keyboard.press('Meta+Shift+z');
    await page.keyboard.press('Meta+s');
    assert.equal(await capture('15-ghost-survivor-redo',['risk','worry'],'Worry needs mitigation.',promoted),promoted);
    assert.deepEqual(persistedGhosts(await fs.readFile(filename,'utf8')),expectedGhosts);
    if(process.env.TERRAPHIM_MAC_UI_EXPORT==='1'){
      const acknowledged=await fs.readFile(filename,'utf8'),body=await source.textContent();const block=acknowledged.match(/```terraphim-alternatives\n([\s\S]*?)\n```\s*$/);assert.ok(block);const annotation=JSON.parse(block[1]);let expectedClean=body;for(const ghost of [...annotation.ghosts].sort((a,b)=>b.anchor.start-a.anchor.start)){assert.equal(body.slice(ghost.anchor.start,ghost.anchor.end),ghost.anchor.text);expectedClean=expectedClean.slice(0,ghost.anchor.start)+expectedClean.slice(ghost.anchor.end);}
      async function actualExport(command,name){await source.press('Escape');if(command){await page.keyboard.press('Meta+Shift+p');await page.getByRole('textbox',{name:'Type the name of a command to run.',exact:true}).fill('>'+command);await page.getByText(command,{exact:true}).click();await page.getByRole('textbox',{name:'Type the name of a command to run.',exact:true}).waitFor({state:'hidden'});}else await frame.getByRole('button',{name:'Markdown view and export',exact:true}).click();const chooser=page.locator('.quick-input-widget input').filter({visible:true});await chooser.waitFor();const target=path.join(workspace,name);await chooser.fill(target);await chooser.press('Enter');await waitFor(async()=>{try{return(await fs.stat(target)).size>0;}catch{return false;}},'actual native export file '+name);const actual=await fs.readFile(target,'utf8');await fs.writeFile(path.join(output,name),actual);await page.screenshot({path:path.join(output,name+'.png')});await page.keyboard.press('Meta+w');await source.waitFor({state:'visible'});return actual;}
      const graphText=await actualExport('Terraphim: Export Document Knowledge Graph','actual-ui-knowledge-graph.json');const graph=JSON.parse(graphText);assert.deepEqual(Object.keys(graph.data).sort(),['risk','worry']);assert.equal(new Set(Object.values(graph.data).map(x=>x.id)).size,1);const clean=await actualExport(null,'actual-ui-clean.md');assert.equal(clean.trimEnd(),expectedClean.trimEnd());assert.ok(!clean.includes('terraphim-alternatives'));receipt.checks.push({id:'actual-native-graph-and-clean-export',status:'PASS',graphSha256:crypto.createHash('sha256').update(graphText).digest('hex'),cleanSha256:crypto.createHash('sha256').update(clean).digest('hex')});
    }
    if(process.env.TERRAPHIM_MAC_FAST_TITLE_SAVE==='1'){
      await fs.writeFile(path.join(output,'native-title-controls.json'),JSON.stringify(await page.getByRole('button').evaluateAll(items=>items.map(el=>({name:el.getAttribute('aria-label'),title:el.getAttribute('title'),text:el.textContent})).filter(x=>(x.name||x.title||'').includes('Alternative Control'))),null,2));
      for(let cycle=1;cycle<=3;cycle++){
        await focusView(frame);await page.getByRole('button',{name:/Undo \(Alternative Control\)/}).click();await page.keyboard.press('Meta+s');
        assert.equal(await capture('fast-title-'+cycle+'-undo',['risk','worry'],'Worry needs mitigation.',priorGhosts),priorGhosts);
        await focusView(frame);await page.getByRole('button',{name:/Redo \(Alternative Control\)/}).click();await page.keyboard.press('Meta+s');
        assert.equal(await capture('fast-title-'+cycle+'-redo',['risk','worry'],'Worry needs mitigation.',promoted),promoted);
      }
    }
    if(process.env.TERRAPHIM_MAC_DOUBLE_TITLE==='1'){
      await source.press('Meta+ArrowUp');await page.keyboard.down('Shift');for(let i=0;i<5;i++)await page.keyboard.press('ArrowRight');await page.keyboard.up('Shift');assert.equal(await frame.evaluate(()=>getSelection().toString()),'Worry');
      await page.keyboard.press('Control+Shift+a');await frame.locator('.te-alt-input--new').click();await page.keyboard.type('Hazard',{delay:45});await waitFor(async()=>(await source.innerText()).startsWith('Hazard needs mitigation.'),'actual pending live Hazard visible');
      await page.getByRole('button',{name:/Undo \(Alternative Control\)/}).click();await page.getByRole('button',{name:/Undo \(Alternative Control\)/}).click();await page.keyboard.press('Meta+s');
      assert.equal(await capture('double-title-live-undo',['risk','worry'],'Worry needs mitigation.',priorGhosts),priorGhosts);
    }
    if(process.env.TERRAPHIM_MAC_EXTERNAL==='1'){
      await source.press('Escape');const beforeExternal=await fs.readFile(filename,'utf8');const fence=beforeExternal.lastIndexOf('```terraphim-alternatives');assert.ok(fence>0);const idleInput=beforeExternal.slice(0,fence)+'EXTERNAL-IDLE-SENTINEL\n\n'+beforeExternal.slice(fence);await fs.writeFile(path.join(output,'external-idle.input.md'),idleInput);await fs.writeFile(filename,idleInput);
      await waitFor(async()=>(await source.textContent()).includes('EXTERNAL-IDLE-SENTINEL'),'idle native external source adoption',20000);await source.press('Meta+z');assert.ok((await source.textContent()).includes('EXTERNAL-IDLE-SENTINEL'),'external adoption starts fresh native history');await page.keyboard.press('Meta+s');await waitFor(async()=>await page.locator('.tabs-container .tab.dirty').count()===0,'external native save settled');const adopted=await fs.readFile(filename,'utf8');assert.ok(adopted.includes('EXTERNAL-IDLE-SENTINEL'));await fs.writeFile(path.join(output,'external-idle-adopted.saved.md'),adopted);await page.screenshot({path:path.join(output,'external-idle-adopted.png')});receipt.checks.push({id:'external-idle-fresh-history',status:'PASS',sha256:crypto.createHash('sha256').update(adopted).digest('hex')});
      if(process.env.TERRAPHIM_MAC_EXTERNAL_SAVE_REOPEN==='1'){
        await source.press('Meta+ArrowDown');await page.keyboard.type('FIRST-EDIT-SAVED-AFTER-EXTERNAL',{delay:15});await waitFor(async()=>(await source.textContent()).includes('FIRST-EDIT-SAVED-AFTER-EXTERNAL'),'actual first edit after canonical external admission');await page.keyboard.press('Meta+s');let firstEditSaved;await waitFor(async()=>{firstEditSaved=await fs.readFile(filename,'utf8');return firstEditSaved.includes('FIRST-EDIT-SAVED-AFTER-EXTERNAL')&&await page.locator('.tabs-container .tab.dirty').count()===0;},'actual first external-admission edit persisted');await fs.writeFile(path.join(output,'external-first-edit.saved.md'),firstEditSaved);await page.screenshot({path:path.join(output,'external-first-edit-saved.png')});
        await page.keyboard.press('Meta+w');await page.keyboard.press('Meta+Shift+e');await page.getByLabel('Files Explorer',{exact:true}).getByText('history.md',{exact:true}).dblclick();await page.keyboard.press('Meta+Shift+p');await page.getByRole('textbox',{name:'Type the name of a command to run.',exact:true}).fill('>Terraphim: Open Alternative Control');await page.getByText('Terraphim: Open Alternative Control',{exact:true}).click();await waitFor(async()=>{frame=page.frames().filter(f=>f.url().includes('fake.html')).at(-1);return frame&&await frame.getByRole('textbox',{name:'Markdown source',exact:true}).isVisible();},'actual externally adopted first edit reopened',60000);source=frame.getByRole('textbox',{name:'Markdown source',exact:true});await waitFor(async()=>(await source.textContent()).includes('FIRST-EDIT-SAVED-AFTER-EXTERNAL'),'actual reopened first edit');assert.equal(await fs.readFile(filename,'utf8'),firstEditSaved,'reopen preserves exact first-edit canonical saved bytes');await fs.writeFile(path.join(output,'external-first-edit-reopened.native.txt'),await source.textContent());await page.screenshot({path:path.join(output,'external-first-edit-reopened.png')});receipt.checks.push({id:'external-canonical-first-edit-save-reopen',status:'PASS',sha256:crypto.createHash('sha256').update(firstEditSaved).digest('hex')});
        if(process.env.TERRAPHIM_MAC_EXTERNAL_SAVE_REOPEN_ONLY==='1'){receipt.status='PASS';return;}
      }
      await page.bringToFront();await source.press('Meta+ArrowDown');await waitFor(async()=>await source.evaluate(el=>document.hasFocus()&&document.activeElement===el),'owned native source focused before external draft');await fs.writeFile(path.join(output,'external-input-focus.json'),JSON.stringify(await source.evaluate(el=>({documentFocused:document.hasFocus(),sourceFocused:document.activeElement===el,activeTag:document.activeElement.tagName,activeClass:document.activeElement.className})),null,2));await page.keyboard.type('LOCAL-DRAFT-',{delay:10});await waitFor(async()=>(await source.textContent()).includes('LOCAL-DRAFT-'),'actual local marker received before external race');await page.keyboard.type('x'.repeat(1500));const visibleBefore=await source.textContent();assert.ok(visibleBefore.includes('LOCAL-DRAFT-'));await fs.writeFile(path.join(output,'external-dirty-before.native.txt'),visibleBefore);const replacement=adopted.replace('EXTERNAL-IDLE-SENTINEL','EXTERNAL-REPLACEMENT');await fs.writeFile(path.join(output,'external-dirty.input.md'),replacement);await fs.writeFile(filename,replacement);
      await frame.getByRole('button',{name:'Open recovery draft',exact:true}).waitFor({state:'visible',timeout:20000});await page.screenshot({path:path.join(output,'external-dirty-before-recovery-click.png')});await frame.getByRole('button',{name:'Open recovery draft',exact:true}).click();await fs.writeFile(path.join(output,'external-recovery-click.json'),JSON.stringify({actualButton:'Open recovery draft',clickedAt:new Date().toISOString(),acceptedTypedCharacters:visibleBefore.length,viewRetainedLocalDraft:visibleBefore.includes('LOCAL-DRAFT-')},null,2)+'\n');
      const recovery=path.join(profile,'User/globalStorage/applied-knowledge-systems.terraphim-it/native-authority-recovery');let archives=[];await waitFor(async()=>{try{archives=await fs.readdir(recovery);return archives.some(x=>x.endsWith('.json'));}catch{return false;}},'actual dirty external recovery archive',30000);const recovered=[];for(const name of archives.filter(x=>x.endsWith('.json'))){const bytes=await fs.readFile(path.join(recovery,name));await fs.writeFile(path.join(output,'external-recovery-'+name),bytes);recovered.push(JSON.parse(bytes));}const evidence=JSON.stringify(recovered);assert.ok(evidence.includes('LOCAL-DRAFT-'),'actual dirty draft/accepted document retained in real archive');assert.equal(await fs.readFile(filename,'utf8'),replacement,'external disk bytes are not overwritten');assert.equal(await source.textContent(),visibleBefore,'conflict does not overwrite existing native view draft');await page.screenshot({path:path.join(output,'external-dirty-preserved.png')});await fs.writeFile(path.join(output,'external-dirty-after.native.txt'),await source.textContent());receipt.checks.push({id:'external-dirty-preserves-native-buffer-and-disk',status:'PASS',archiveCount:recovered.length,opaqueDraftEntries:recovered.reduce((n,x)=>n+(x.drafts?.length||0),0),diskSha256:crypto.createHash('sha256').update(replacement).digest('hex')});
    }
    receipt.status='PASS';
    }
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
    inspectorSocket?.close();
    if(electronApp)await electronApp.close().catch(()=>{});
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
