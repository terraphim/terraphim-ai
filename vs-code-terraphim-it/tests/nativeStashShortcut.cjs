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
  const output = path.resolve(process.env.TERRAPHIM_SHORTCUT_OUTPUT || path.join(root, 'native-results', path.basename(run)));
  const profile = path.join(run, 'profile');
  const extensions = path.join(run, 'extensions');
  const workspace = path.join(run, 'workspace');
  const filename = path.join(workspace, 'shortcut.md');
  const initial = 'Keep this line.\n\nMove this paragraph.\n';
  const selected = 'Move this paragraph.';
  const vsix = path.resolve(process.env.TERRAPHIM_SHORTCUT_VSIX || path.join(root, 'terraphim-it.vsix'));
  await fs.mkdir(workspace);
  await fs.mkdir(output, { recursive: true });
  await fs.writeFile(filename, initial, { flag: 'wx' });
  await fs.writeFile(path.join(workspace, 'native.md'), 'Native Markdown control.\n', { flag: 'wx' });
  await fs.writeFile(path.join(output, 'input.md'), initial);
  const receipt = { status: 'RUNNING', scope: 'Real isolated Linux VS Code UI shortcut regression',
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
    await page.getByRole('tab', { name:'shortcut.md', exact:true }).click();
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
    await source.click({ position:{x:100,y:80} });
    await page.keyboard.press('Control+Home');
    for (let n=0; n<initial.indexOf(selected); n++) await page.keyboard.press('ArrowRight');
    await page.keyboard.down('Shift');
    for (let n=0; n<selected.length; n++) await page.keyboard.press('ArrowRight');
    await page.keyboard.up('Shift');
    assert.equal(await frame.evaluate(() => getSelection().toString()), selected);
    await page.keyboard.press('Control+Shift+X');
    await waitFor(async () => (await frame.getByRole('textbox', {name:'Overflow text', exact:true}).inputValue()) === selected, 'stash output');
    const focused = await source.evaluate(element => document.hasFocus() && (element === document.activeElement || element.contains(document.activeElement)));
    const hostText = await page.locator('body').innerText();
    await fs.writeFile(path.join(output, 'after-stash-ui.json'), JSON.stringify({ focused, hostText }, null, 2)+'\n');
    await page.screenshot({ path:path.join(output, 'after-stash.png') });
    await page.keyboard.press('Control+s');
    await waitFor(async () => (await fs.readFile(filename,'utf8')).includes('"overflow": "'+selected+'"'), 'saved stash');
    await fs.copyFile(filename, path.join(output, 'stashed.md'));
    assert.equal(focused, true, 'consumed stash must retain source focus');
    assert.ok(!hostText.includes('Search Extensions in Marketplace'), 'consumed stash must not open Extensions');
    console.log('Verified next shortcut scenario');
    receipt.checks.push({ id:'consumed-stash-focus', status:'PASS' });
    await page.keyboard.press('Control+z');
    await waitFor(async () => (await source.innerText()).includes(selected), 'native Undo without refocus');
    await page.keyboard.press('Control+s');
    await waitFor(async () => await fs.readFile(filename,'utf8') === initial, 'saved native Undo');
    await fs.copyFile(filename, path.join(output,'undo.md'));
    await page.keyboard.press('Control+Shift+z');
    await waitFor(async () => !(await source.innerText()).includes(selected), 'native Redo');
    await page.keyboard.press('Control+s');
    await waitFor(async () => await fs.readFile(filename,'utf8') === await fs.readFile(path.join(output,'stashed.md'),'utf8'), 'saved native Redo');
    await fs.copyFile(filename, path.join(output,'redo.md'));
    console.log('Verified next shortcut scenario');
    receipt.checks.push({ id:'immediate-native-undo-redo', status:'PASS' });

    // The editor does not consume stash with an empty selection.
    await source.click({ position:{x:100,y:80} });
    await page.keyboard.press('Control+Home');
    await page.keyboard.press('Control+Shift+X');
    await waitFor(async () => (await page.locator('body').innerText()).includes('Search Extensions in Marketplace'), 'unhandled shortcut reaches native Extensions');
    await page.screenshot({path:path.join(output,'unhandled-empty-selection.png')});
    assert.equal(await fs.readFile(filename,'utf8'), await fs.readFile(path.join(output,'stashed.md'),'utf8'));
    console.log('Verified next shortcut scenario');
    receipt.checks.push({id:'empty-selection-native-extensions',status:'PASS'});

    await page.keyboard.press('Control+Shift+E');
    await toggle.click();
    assert.equal(await toggle.getAttribute('aria-pressed'),'false');
    await source.click({position:{x:100,y:80}});
    await page.keyboard.press('Control+Home');
    await page.keyboard.press('Control+Shift+End');
    await page.keyboard.press('Control+Shift+X');
    await waitFor(async () => (await page.locator('body').innerText()).includes('Search Extensions in Marketplace'), 'plain mode native Extensions');
    await page.screenshot({path:path.join(output,'unhandled-plain-mode.png')});
    assert.equal(await fs.readFile(filename,'utf8'), await fs.readFile(path.join(output,'stashed.md'),'utf8'));
    console.log('Verified next shortcut scenario');
    receipt.checks.push({id:'plain-mode-native-extensions',status:'PASS'});

    await page.keyboard.press('Control+Shift+E');
    await page.getByLabel('Files Explorer',{exact:true}).getByText('native.md',{exact:true}).dblclick();
    await page.getByRole('tab',{name:'native.md',exact:true}).waitFor();
    await page.keyboard.press('Control+Shift+X');
    await waitFor(async () => (await page.locator('body').innerText()).includes('Search Extensions in Marketplace'), 'ordinary native editor Extensions');
    await page.screenshot({path:path.join(output,'native-editor-extensions.png')});
    assert.equal(await fs.readFile(path.join(workspace,'native.md'),'utf8'),'Native Markdown control.\n');
    console.log('Verified next shortcut scenario');
    receipt.checks.push({id:'native-editor-extensions',status:'PASS'});
    receipt.status = 'PASS';
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
