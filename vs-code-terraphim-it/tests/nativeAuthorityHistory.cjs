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
const { matchesSavedSource } = require('./nativeSavedSnapshot.cjs');

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
  const output = path.resolve(process.env.TERRAPHIM_HISTORY_OUTPUT || path.join(root, 'native-results', path.basename(run)));
  const profile = path.join(run, 'profile');
  const extensions = path.join(run, 'extensions');
  const workspace = path.join(run, 'workspace');
  const filename = path.join(workspace, 'history.md');
  const initial = 'Risk needs mitigation.\n\nThe editor is basically the only target here today.\n';
  const vsix = path.resolve(process.env.TERRAPHIM_HISTORY_VSIX || path.join(root, 'terraphim-it.vsix'));
  await fs.mkdir(workspace);
  const autosave = process.env.TERRAPHIM_HISTORY_AUTOSAVE === '1';
  await fs.mkdir(path.join(profile,'User'),{recursive:true});
  await fs.writeFile(path.join(profile,'User/settings.json'),JSON.stringify({'window.dialogStyle':'custom',...(autosave?{'files.autoSave':'afterDelay','files.autoSaveDelay':200}:{})}));
  await fs.mkdir(output, { recursive: true });
  await fs.writeFile(filename, initial, { flag: 'wx' });
  await fs.writeFile(path.join(workspace, 'native.md'), 'Native Markdown control.\n', { flag: 'wx' });
  await fs.writeFile(path.join(output, 'input.md'), initial);
  const receipt = { status: 'RUNNING', scope: 'Real isolated Linux VS Code UI authority lifetime, alternatives and protected Ghost history regression',
    autosaveDuringLiveAddition: autosave, packageSha256: crypto.createHash('sha256').update(await fs.readFile(vsix)).digest('hex'), checks: [] };
  await fs.copyFile(__filename,path.join(output,'executed-harness.cjs'));
  receipt.modes={autosave,split:process.env.TERRAPHIM_HISTORY_SPLIT==='1',external:process.env.TERRAPHIM_HISTORY_EXTERNAL==='1'};
  let app;
  let launchError;
  let browser;
  let page;
  let log;
  let error,step='initializing';
  async function progress(name){step=name;await fs.writeFile(path.join(output,'progress.json'),JSON.stringify({step,checks:receipt.checks,at:new Date().toISOString()},null,2));}
  const watchdog=setTimeout(async()=>{
    receipt.status='FAIL';receipt.fixtureWatchdog=true;receipt.error='Bounded fixture watchdog at '+step;
    await fs.writeFile(path.join(output,'receipt.json'),JSON.stringify(receipt,null,2));
    await fs.copyFile(filename,path.join(output,'watchdog-actual-saved.md')).catch(()=>undefined);
    if(page)await Promise.race([page.screenshot({path:path.join(output,'watchdog.png')}).catch(()=>undefined),delay(2000)]);
    if(app&&app.exitCode===null)app.kill('SIGKILL');process.exit(124);
  },120000);watchdog.unref();
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
    log = await fs.open(path.join(output, 'host.log'), 'wx');
    app = spawn(executable, [...flags, `--remote-debugging-port=${port}`, workspace, filename],
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
    async function capture(name, expectedTerms, expectedBody, expectedFullSource) {
      await progress('capture: '+name);let saved;
      try { await waitFor(async () => {
        saved = await fs.readFile(filename, 'utf8');
        const graph = JSON.parse(await exportKnowledgeGraph(saved, installedAssets));
        return (await source.innerText()).split('\n')[0] === expectedBody &&
          await page.locator('.tabs-container .tab.dirty').count() === 0 &&
          saved.split('\n')[0] === expectedBody &&
          matchesSavedSource(saved, expectedFullSource) &&
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
    await page.keyboard.press('Escape');
    await source.click({position:{x:12,y:12}});
    await page.keyboard.press('Control+z');
    await page.keyboard.press('Control+s');
    const undone = await capture('02-whole-undo',[],'Risk needs mitigation.');
    assert.equal(undone,initial,'whole Undo must restore exact original bytes');
    await page.keyboard.press('Control+Shift+z');
    await page.keyboard.press('Control+s');
    assert.equal(await capture('03-whole-redo',['concern','risk'],'Concern needs mitigation.'),accepted);

    // A remains unsaved while B is typed and cancelled; a following Redo may
    // never revive B. All observations are real UI and filesystem readbacks.
    await page.keyboard.press('Control+z');
    await chooseRisk();
    await frame.locator('.te-alt-input--new').click();
    await page.keyboard.type('Worry',{delay:80});
    await page.keyboard.press('Enter');
    const newAlternative = frame.locator('.te-alt-input--new');
    await newAlternative.click();
    await page.keyboard.type('Hazard',{delay:80});
    await page.keyboard.press('Escape');
    await page.keyboard.press('Control+s');
    const canceled = await capture('04-escape-preserves-A',['risk','worry'],'Worry needs mitigation.');
    await source.click({position:{x:12,y:12}});
    await page.keyboard.press('Control+Shift+z');
    await page.keyboard.press('Control+s');
    assert.equal(await capture('05-redo-cannot-revive-B',['risk','worry'],'Worry needs mitigation.'),canceled);
    await progress('reopen: close current custom view');
    await page.keyboard.press('Control+w');
    await progress('reopen: show Explorer');
    await page.keyboard.press('Control+Shift+E');
    await progress('reopen: double-click saved file');
    await page.getByLabel('Files Explorer',{exact:true}).getByText('history.md',{exact:true}).dblclick();
    await progress('reopen: open command palette');
    await page.keyboard.press('Control+Shift+P');
    await progress('reopen: fill command');
    await page.getByRole('textbox',{name:'Type the name of a command to run.',exact:true}).fill('>Terraphim: Open Alternative Control');
    await progress('reopen: invoke custom editor');
    await page.getByText('Terraphim: Open Alternative Control',{exact:true}).click();
    await progress('reopen: wait actual saved document');
    await waitFor(async()=>{
      frame=page.frames().filter(f=>f.url().includes('fake.html')).at(-1);
      return frame && await frame.getByRole('textbox',{name:'Markdown source',exact:true}).isVisible();
    },'actual saved document reopen',60000);
    source=frame.getByRole('textbox',{name:'Markdown source',exact:true});
    await waitFor(async()=>(await source.innerText()).split('\n')[0]==='Worry needs mitigation.','reopened accepted document content',60000);
    assert.equal(await capture('06-saved-reopen',['risk','worry'],'Worry needs mitigation.'),canceled);


    assert.notEqual(process.env.TERRAPHIM_HISTORY_EXTERNAL,'1','Authority receipt sequence uses its independent profile');
    assert.notEqual(process.env.TERRAPHIM_HISTORY_SPLIT,'1','Authority receipt sequence creates and tracks its own split views');
    // Swap an existing alternative through the real panel before the protected
    // receipt test. Saved output, not the requested input, is the assertion.
    await source.click({position:{x:12,y:12}});
    await page.keyboard.press('Control+Home');
    await page.keyboard.down('Shift');
    for(let i=0;i<5;i++)await page.keyboard.press('ArrowRight');
    await page.keyboard.up('Shift');
    assert.equal(await frame.evaluate(()=>getSelection().toString()),'Worry');
    await page.keyboard.press('Control+Shift+A');
    await frame.locator('.te-alt-line[data-index="1"] input').click();
    await page.keyboard.press('ArrowUp');
    await waitFor(async()=>(await source.innerText()).split('\n')[0]==='Risk needs mitigation.','actual original swap visible');
    await page.keyboard.press('Escape');
    await page.keyboard.press('Control+s');
    const swapped = await capture('07-native-swap-original',['risk','worry'],'Risk needs mitigation.');
    await focusView(frame);
    await page.keyboard.press('Control+z');
    await page.keyboard.press('Control+s');
    assert.equal(await capture('08-native-swap-undo',['risk','worry'],'Worry needs mitigation.'),canceled);
    await page.keyboard.press('Control+Shift+z');
    await page.keyboard.press('Control+s');
    assert.equal(await capture('09-native-swap-redo',['risk','worry'],'Risk needs mitigation.'),swapped);
    await page.keyboard.press('Control+z');
    await page.keyboard.press('Control+s');
    assert.equal(await capture('10-native-swap-restored',['risk','worry'],'Worry needs mitigation.'),canceled);

    // A real protected promotion must retain its private native receipt after a
    // second view attaches and after the issuing view is disposed.
    const priorGhosts = canceled;
    const currentToggle = frame.getByTitle('Toggle Write_On mode');
    if (await currentToggle.getAttribute('aria-pressed') !== 'true') await currentToggle.click();
    await frame.getByRole('button',{name:'Lab',exact:true}).click();
    await frame.getByRole('radio',{name:/Even sharper/}).click();
    await frame.getByRole('button',{name:'Done',exact:true}).click();
    await focusView(frame);
    await page.keyboard.press('Control+s');
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
    await page.keyboard.press('Control+\\');
    let survivingFrame;
    await waitFor(async()=>{
      for(const candidate of page.frames().filter(f=>f.url().includes('fake.html')&&f!==issuingFrame)) {
        if(await candidate.getByRole('textbox',{name:'Markdown source',exact:true}).isVisible())survivingFrame=candidate;
      }
      return !!survivingFrame;
    },'new native view attaches to protected receipt owner',60000);
    frame=survivingFrame;source=frame.getByRole('textbox',{name:'Markdown source',exact:true});
    await focusView(frame);
    await page.keyboard.press('Control+z');
    await page.keyboard.press('Control+s');
    assert.equal(await capture('12-ghost-other-view-undo',['risk','worry'],'Worry needs mitigation.',priorGhosts),priorGhosts);
    await page.keyboard.press('Control+Shift+z');
    await page.keyboard.press('Control+s');
    assert.equal(await capture('13-ghost-other-view-redo',['risk','worry'],'Worry needs mitigation.',promoted),promoted);
    await focusView(issuingFrame);
    await page.keyboard.press('Control+w');
    frame=survivingFrame;source=frame.getByRole('textbox',{name:'Markdown source',exact:true});
    await focusView(frame);
    await page.keyboard.press('Control+z');
    await page.keyboard.press('Control+s');
    assert.equal(await capture('14-ghost-issuing-view-closed-undo',['risk','worry'],'Worry needs mitigation.',priorGhosts),priorGhosts);
    await page.keyboard.press('Control+Shift+z');
    await page.keyboard.press('Control+s');
    assert.equal(await capture('15-ghost-survivor-redo',['risk','worry'],'Worry needs mitigation.',promoted),promoted);
    assert.deepEqual(persistedGhosts(await fs.readFile(filename,'utf8')),expectedGhosts);
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
    clearTimeout(watchdog);
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
