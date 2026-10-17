/* Actual packaged VS Code UI regression; no injected events or editor models. */
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
  const output = path.resolve(process.env.TERRAPHIM_HISTORY_OUTPUT || path.join(root, 'native-results', path.basename(run)));
  const profile = path.join(run, 'profile');
  const extensions = path.join(run, 'extensions');
  const workspace = path.join(run, 'workspace');
  const filename = path.join(workspace, 'history.md');
  const initial = 'Risk needs mitigation.\n';
  const vsix = path.resolve(process.env.TERRAPHIM_HISTORY_VSIX || path.join(root, 'terraphim-it.vsix'));
  await fs.mkdir(workspace);
  await fs.mkdir(output, { recursive: true });
  await fs.writeFile(filename, initial, { flag: 'wx' });
  await fs.writeFile(path.join(workspace, 'native.md'), 'Native Markdown control.\n', { flag: 'wx' });
  await fs.writeFile(path.join(output, 'input.md'), initial);
  const receipt = { status: 'RUNNING', scope: 'Real isolated Linux VS Code UI whole-alternative history regression',
    packageSha256: crypto.createHash('sha256').update(await fs.readFile(vsix)).digest('hex'), checks: [] };
  let app;
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
    const source = frame.getByRole('textbox', {name:'Markdown source', exact:true});
    const toggle = frame.getByTitle('Toggle Write_On mode');
    if (await toggle.getAttribute('aria-pressed') !== 'true') await toggle.click();

    const { exportKnowledgeGraph } = require('../media/knowledge-graph.cjs');
    async function capture(name, expectedTerms, expectedBody) {
      let saved;
      try { await waitFor(async () => {
        saved = await fs.readFile(filename, 'utf8');
        const graph = JSON.parse(await exportKnowledgeGraph(saved, path.join(root, 'media/editor')));
        return saved.split('\n')[0] === expectedBody &&
          JSON.stringify(Object.keys(graph.data).sort()) === JSON.stringify(expectedTerms);
      }, name + ' actual saved Markdown and graph');
      } catch(error) {
        const observed = await fs.readFile(filename,'utf8');
        await fs.writeFile(path.join(output,name+'.observed.md'),observed);
        await fs.writeFile(path.join(output,name+'.observed-graph.json'),await exportKnowledgeGraph(observed,path.join(root,'media/editor')));
        throw error;
      }
      await fs.writeFile(path.join(output, name + '.md'), saved);
      const graph = JSON.parse(await exportKnowledgeGraph(saved, path.join(root, 'media/editor')));
      await fs.writeFile(path.join(output, name + '.knowledge-graph.json'), JSON.stringify(graph,null,2)+'\n');
      if (expectedTerms.length) {
        assert.equal(new Set(Object.values(graph.data).map(term=>term.id)).size,1);
        assert.equal(new Set(Object.values(graph.data).map(term=>term.nterm)).size,1);
      }
      await page.screenshot({path:path.join(output,name+'.png')});
      receipt.checks.push({id:name,status:'PASS',sourceSha256:crypto.createHash('sha256').update(saved).digest('hex'),terms:expectedTerms});
      return saved;
    }
    async function chooseRisk() {
      await page.keyboard.press('Escape');
      await source.click({position:{x:100,y:60}});
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
    await page.keyboard.type('Concern',{delay:100});
    await page.keyboard.press('Enter');
    await page.keyboard.press('Control+s');
    const accepted = await capture('01-added-concern',['concern','risk'],'Concern needs mitigation.');
    await page.keyboard.press('Escape');
    await source.click({position:{x:100,y:60}});
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
    await page.keyboard.type('Concern',{delay:80});
    await page.keyboard.press('Enter');
    const newAlternative = frame.locator('.te-alt-input--new');
    await newAlternative.click();
    await page.keyboard.type('Hazard',{delay:80});
    await page.keyboard.press('Escape');
    await page.keyboard.press('Control+s');
    const canceled = await capture('04-escape-preserves-A',['concern','risk'],'Concern needs mitigation.');
    await source.click({position:{x:100,y:60}});
    await page.keyboard.press('Control+Shift+z');
    await page.keyboard.press('Control+s');
    assert.equal(await capture('05-redo-cannot-revive-B',['concern','risk'],'Concern needs mitigation.'),canceled);
    receipt.status='PASS';
  } catch (caught) {
    error = caught;
    receipt.status = 'FAIL';
    receipt.error = String(caught.stack || caught);
    if (page) await page.screenshot({path:path.join(output,'failure.png')}).catch(() => {});
  } finally {
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
