// Real browser DOM, production bundle and shipped Rust/WASM. No replay doubles.
const {test:nodeTest, before, after} = require('node:test');
const test=(name,fn)=>nodeTest(name,{timeout:30000},fn);
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const http = require('node:http');
const {chromium} = require('playwright-core');
const root = path.resolve(__dirname, '../media');
let server, browser, origin;
before(async () => {
  server = http.createServer((req, res) => {
    const file = path.resolve(root, '.' + decodeURIComponent(new URL(req.url, 'http://localhost').pathname));
    if (!file.startsWith(root + path.sep) || !fs.existsSync(file) || !fs.statSync(file).isFile()) {
      res.writeHead(404); res.end(); return;
    }
    const type = file.endsWith('.wasm') ? 'application/wasm' : file.endsWith('.js') ? 'text/javascript' : 'text/plain';
    res.writeHead(200, {'content-type': type}); fs.createReadStream(file).pipe(res);
  });
  await new Promise((resolve,reject) => { server.once('error',reject);server.listen(0,'127.0.0.1',resolve); });
  origin = `http://127.0.0.1:${server.address().port}`;
  const executablePath = [process.env.TERRAPHIM_BROWSER_EXECUTABLE, process.env.CHROME_BIN,
    '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome', '/usr/bin/chromium', '/usr/bin/chromium-browser', '/usr/bin/google-chrome',
    'C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe']
    .filter(Boolean).find(file => fs.existsSync(file));
  if (!executablePath) throw new Error('Set TERRAPHIM_BROWSER_EXECUTABLE to an installed Chromium browser');
  browser = await chromium.launch({headless: true, executablePath, timeout:15000});
});
after(async () => { await browser?.close(); await new Promise(resolve => server?.close(resolve)); });
async function editorPage(source = 'Risk needs a decision.\n') {
  const page = await browser.newPage();
  page.setDefaultTimeout(15000);
  await page.goto(origin + '/history-checkpoint.js');
  await page.setContent('<!doctype html><div id="fixture"></div>');
  await page.addScriptTag({url: origin + '/editor/terraphim-editor.min.js'});
  await page.addScriptTag({url: origin + '/history-checkpoint.js'});
  await page.evaluate(async value => {
    window.actual = await TeraphimEditor.create(document.querySelector('#fixture'), {loadShoelace:false, value});
    window.editor = actual.markdownEditor;
  }, source);
  return page;
}
test('real production editor and shipped WASM initialize before checkpoint assertions', async () => {
  const page = await editorPage();
  try {
    assert.deepEqual(await page.evaluate(() => ({text:editor.surface.getText(), model:typeof actual.bindings.open_document,
      history:editor.surface.history.length})), {text:'Risk needs a decision.\n', model:'function', history:1});
  } finally { await page.close(); }
});
async function startAlternative(page, text) {
  await page.evaluate(() => editor.altPanel.open({start:0,end:4,kind:'word'}));
  await page.locator('.te-alt-input--new').pressSequentially(text);
}
async function checkpoint(page) {
  return page.evaluate(() => {
    const api=editor.documentApi(); editor.alignDocumentModel(api);
    const source=api.save_document();
    return TerraphimHistoryCheckpoint.capture(editor.surface,source,editor.altPanel);
  });
}
async function state(page) {
  return page.evaluate(() => {
    const api=editor.documentApi(); editor.alignDocumentModel(api);
    return {body:editor.surface.getText(), source:api.save_document(), annotations:api.document_annotations(),
      canUndo:editor.surface.canUndo(), canRedo:editor.surface.canRedo()};
  });
}
test('live Infinity capture is finite and detached; capture never breaks original base reference or cancellation',async()=>{
 const page=await editorPage();
 try {
  await startAlternative(page,'C');
  await page.locator('.te-alt-input:not(.te-alt-input--new):not([readonly])').pressSequentially('oncern');
  const before=await page.evaluate(()=>({baseIndex:editor.surface.history.indexOf(editor.altPanel.live.base),
    liveLimit:editor.surface.historyLimit, length:editor.surface.history.length}));
  assert.equal(before.liveLimit,Infinity);
  const saved=await checkpoint(page);
  assert.equal(saved.historyLimit,500);
  assert.ok(saved.history.length<before.length,'transport folds actual sequential edits');
  assert.equal(await page.evaluate(()=>editor.surface.history.indexOf(editor.altPanel.live.base)),before.baseIndex);
  assert.equal(await page.evaluate(()=>editor.surface.history.length),before.length);
  await page.locator('.te-alt-input:not(.te-alt-input--new):not([readonly])').press('Escape');
  const cancelled=await state(page);
  assert.equal(cancelled.body,'Risk needs a decision.\n');
  assert.equal(cancelled.canRedo,false,'cancelled addition must not return through Redo');
  assert.ok(!cancelled.source.includes('Concern'));
 } finally {await page.close();}
});
test('actual history transfers between real WASM instances and replays complete authored annotation states',async()=>{
 const a=await editorPage(); let b;
 try {
  await startAlternative(a,'C');
  await a.locator('.te-alt-input:not(.te-alt-input--new):not([readonly])').pressSequentially('oncern');
  const saved=await checkpoint(a); const accepted=await state(a);
  b=await editorPage();
  await b.evaluate(source=>actual.openDocument(source),saved.presentationSource);
  await b.evaluate(saved=>TerraphimHistoryCheckpoint.restore(editor.surface,saved,saved.presentationSource,editor),saved);
  assert.equal((await state(b)).source,accepted.source);
  assert.equal(await b.evaluate(()=>editor.surface.undo()),true);
  const undone=await state(b); assert.equal(undone.body,'Risk needs a decision.\n'); assert.ok(!undone.source.includes('Concern'));
  assert.equal(await b.evaluate(()=>editor.surface.redo()),true);
  assert.equal((await state(b)).source,accepted.source);
 } finally {await a.close(); await b?.close();}
});
test('unsaved A survives B cancellation and transferred Undo/Redo cannot resurrect B',async()=>{
 const a=await editorPage(); let b;
 try {
  await startAlternative(a,'C');
  await a.locator('.te-alt-input:not(.te-alt-input--new):not([readonly])').pressSequentially('oncern');
  await a.locator('.te-alt-input:not(.te-alt-input--new):not([readonly])').press('Enter');
  const acceptedA=await state(a);
  await a.locator('.te-alt-input--new').pressSequentially('W');
  await a.locator('.te-alt-input:not(.te-alt-input--new):not([readonly])').last().pressSequentially('orry');
  await checkpoint(a); // A save/checkpoint boundary must not split or commit B.
  await a.locator('.te-alt-input:not(.te-alt-input--new):not([readonly])').last().press('Escape');
  const cancelled=await state(a); assert.equal(cancelled.source,acceptedA.source);
  assert.equal(cancelled.canRedo,false);
  const saved=await checkpoint(a);
  b=await editorPage(); await b.evaluate(source=>actual.openDocument(source),saved.presentationSource);
  await b.evaluate(saved=>TerraphimHistoryCheckpoint.restore(editor.surface,saved,saved.presentationSource,editor),saved);
  assert.equal(await b.evaluate(()=>editor.surface.undo()),true);
  assert.equal((await state(b)).body,'Risk needs a decision.\n');
  assert.equal(await b.evaluate(()=>editor.surface.redo()),true);
  assert.equal((await state(b)).source,acceptedA.source);
  assert.ok(!(await state(b)).source.includes('Worry'));
 } finally {await a.close();await b?.close();}
});
test('transferred history remains a valid base for the next live action and full cancellation',async()=>{
 const a=await editorPage(); let b;
 try {
  await startAlternative(a,'C'); await a.locator('.te-alt-input:not(.te-alt-input--new):not([readonly])').pressSequentially('oncern');
  const saved=await checkpoint(a); b=await editorPage();
  await b.evaluate(source=>actual.openDocument(source),saved.presentationSource);
  await b.evaluate(saved=>TerraphimHistoryCheckpoint.restore(editor.surface,saved,saved.presentationSource,editor),saved);
  await b.evaluate(()=>editor.altPanel.open({spanId:editor.documentApi().document_annotations().spans[0].id}));
  await b.locator('.te-alt-input--new').pressSequentially('A');
  await b.locator('.te-alt-input:not(.te-alt-input--new):not([readonly])').last().pressSequentially('larm');
  assert.ok(await b.evaluate(()=>editor.surface.history.includes(editor.altPanel.live.base)));
  await checkpoint(b);
  await b.locator('.te-alt-input:not(.te-alt-input--new):not([readonly])').last().press('Escape');
  assert.equal((await state(b)).source,saved.presentationSource);
  assert.equal((await state(b)).canRedo,false);
 } finally {await a.close();await b?.close();}
});
test('actual checkpoint enforces the exact UTF-8 16MiB encoded boundary',async()=>{
 const page=await editorPage('🧠 Café Ελληνικά Риск\r\n');
 try {
  const saved=await checkpoint(page);
  const {validateCheckpoint}=require('../media/history-checkpoint.js');
  const body=await page.evaluate(()=>editor.surface.getText());
  const padded=structuredClone(saved);
  padded.history.unshift({...padded.history[0],text:'',start:0,end:0});
  padded.historyIndex++;
  padded.history[0].text='x'.repeat(16*1024*1024-Buffer.byteLength(JSON.stringify(padded)));
  assert.equal(Buffer.byteLength(JSON.stringify(padded)),16*1024*1024);
  assert.equal(validateCheckpoint(padded,saved.presentationSource,body).history[0].text.length,padded.history[0].text.length);
  const oversized=structuredClone(padded); oversized.history[0].text+='x';
  assert.throws(()=>validateCheckpoint(oversized,saved.presentationSource,body),/limit/);
 } finally {await page.close();}
});
test('full admitted near-16MiB source survives chunk transport and actual history transfer',async()=>{
 const source='x'.repeat(16*1024*1024-1024);
 assert.ok(Buffer.byteLength(source)<16*1024*1024);
 const a=await editorPage(source);let b;
 try {
  await a.evaluate(()=>editor.surface.replaceRange(editor.surface.getText().length,editor.surface.getText().length,'!',{source:'api'}));
  const saved=await checkpoint(a);
  assert.equal(saved.presentationSource.length,source.length+1);
  const {chunks,LIMIT,CHUNK_UNITS}=require('../media/history-checkpoint.js');
  const transport=chunks(saved);
  assert.equal(transport.bytes,Buffer.byteLength(JSON.stringify(saved)));
  assert.ok(transport.bytes>LIMIT,'aggregate history must not shrink the admitted document bound');
  assert.ok(transport.parts.length>1);
  assert.ok(transport.parts.every(part=>part.length<=CHUNK_UNITS && Buffer.byteLength(JSON.stringify({part}))<LIMIT));
  const reconstructed=JSON.parse(transport.parts.join(''));
  assert.deepEqual(reconstructed,saved);
  b=await editorPage(); await b.evaluate(source=>actual.openDocument(source),reconstructed.presentationSource);
  await b.evaluate(saved=>TerraphimHistoryCheckpoint.restore(editor.surface,saved,saved.presentationSource,editor),reconstructed);
  assert.equal(await b.evaluate(()=>editor.surface.undo()),true);
  assert.equal(await b.evaluate(()=>editor.surface.getText().length),source.length);
  assert.equal(await b.evaluate(()=>editor.surface.redo()),true);
  assert.equal(await b.evaluate(()=>editor.surface.getText().length),source.length+1);
  assert.equal(await b.evaluate(()=>editor.surface.getText().endsWith('!')),true);
 } finally {await a.close();await b?.close();}
});
test('annotation-only source CAS refuses a foreign checkpoint despite identical visible body',async()=>{
 const a=await editorPage();let b;
 try {
  await startAlternative(a,'C'); await a.locator('.te-alt-input:not(.te-alt-input--new):not([readonly])').pressSequentially('oncern');
  await a.evaluate(()=>editor.altPanel.finishLive()); await a.evaluate(()=>editor.altPanel.setActiveIndex(0));
  const saved=await checkpoint(a);
  b=await editorPage(); await startAlternative(b,'W'); await b.locator('.te-alt-input:not(.te-alt-input--new):not([readonly])').pressSequentially('orry');
  await b.evaluate(()=>editor.altPanel.finishLive()); await b.evaluate(()=>editor.altPanel.setActiveIndex(0));
  const before=await state(b); assert.equal(before.body,(await state(a)).body); assert.notEqual(before.source,saved.presentationSource);
  await assert.rejects(b.evaluate(saved=>{const api=editor.documentApi();editor.alignDocumentModel(api);
    TerraphimHistoryCheckpoint.restore(editor.surface,saved,api.save_document(),editor);},saved),/source mismatch/);
  assert.deepEqual(await state(b),before,'refused foreign checkpoint does not mutate actual accepted source/history');
 } finally {await a.close();await b?.close();}
});
test('real-checkpoint non-finite positions and references are rejected without editor mutation',async()=>{
 const page=await editorPage('🧠 Café Ελληνικά Риск\r\n');
 try {
  const saved=await checkpoint(page);const body=await page.evaluate(()=>editor.surface.getText());
  const {validateCheckpoint}=require('../media/history-checkpoint.js');
  const invalid=structuredClone(saved); invalid.history[0].start=Infinity;
  assert.throws(()=>validateCheckpoint(invalid,saved.presentationSource,body),/non-finite|finite JSON/);
  const invalidReference=structuredClone(saved); invalidReference.history[0].edits=[{ref:Infinity}];
  assert.throws(()=>validateCheckpoint(invalidReference,saved.presentationSource,body),/non-finite|finite JSON/);
  assert.deepEqual(await checkpoint(page),saved,'refused validation leaves actual editor history unchanged');
 } finally {await page.close();}
});
test('real ghost receipt and Overflow history survive transfer with exact annotated Undo/Redo',async()=>{
 const original='Risk remains. Perhaps we might basically need to possibly review the very complex arrangement because the supplier could in some ways delay the launch.\n';
 const a=await editorPage(original);let b;
 try {
  await a.evaluate(()=>{
    try {
      editor.documentApi().kg_load_thesaurus(JSON.stringify({name:'History fixture',data:{risk:{id:1,nterm:'risk',display_value:'Risk',pinned:false}}}));
      editor.documentApi().trim_plan_json();
      editor.commitTrimGhosts('sharper',[]);
    } catch(error) {throw new Error(JSON.stringify(error));}
  });
  const ghosted=await state(a);assert.ok(ghosted.annotations.ghosts.length>0,'actual Rust trim creates ghost receipt');
  await a.evaluate(()=>editor.stashRange(0,4));
  const final=await state(a);assert.ok(final.source.includes('Risk'));
  const saved=await checkpoint(a);
  b=await editorPage();await b.evaluate(source=>actual.openDocument(source),saved.presentationSource);
  await b.evaluate(saved=>TerraphimHistoryCheckpoint.restore(editor.surface,saved,saved.presentationSource,editor),saved);
  assert.equal(await b.evaluate(()=>editor.surface.undo()),true);
  assert.equal((await state(b)).source,ghosted.source);
  const ghostUndo=await b.evaluate(()=>({ok:editor.surface.undo(),notice:editor.notice,warning:editor.warning,conflict:editor.ghostHistoryConflict}));
  assert.equal(ghostUndo.ok,true,JSON.stringify(ghostUndo));
  assert.equal((await state(b)).body,original);assert.equal((await state(b)).annotations.ghosts.length,0);
  assert.equal(await b.evaluate(()=>editor.surface.redo()),true);assert.equal((await state(b)).source,ghosted.source);
  assert.equal(await b.evaluate(()=>editor.surface.redo()),true);assert.equal((await state(b)).source,final.source);
 } finally {await a.close();await b?.close();}
});
test('real Unicode UTF-16 anchors and CRLF presentation survive whole-action transfer',async()=>{
 const prefix='🧠 Café e\u0301 Ελληνικά Риск. ';
 const a=await editorPage(prefix+'Risk needs a decision.\r\n');let b;
 try {
  assert.notEqual(prefix.length,[...prefix].length,'non-BMP fixture distinguishes UTF-16 anchors');
  await a.evaluate(start=>editor.altPanel.open({start,end:start+4,kind:'word'}),prefix.length);
  await a.locator('.te-alt-input--new').pressSequentially('C');
  const first=await checkpoint(a);
  const canonical=await a.evaluate(source=>actual.bindings.host_presentation_source(source.replace(/\n/g,'\r\n')),first.presentationSource);
  assert.equal(canonical,first.presentationSource,'existing Rust presentation conversion canonicalizes CRLF');
  const base=await a.evaluate(()=>editor.surface.history.indexOf(editor.altPanel.live.base));
  await a.locator('.te-alt-input:not(.te-alt-input--new):not([readonly])').pressSequentially('oncern');
  assert.equal(await a.evaluate(()=>editor.surface.history.indexOf(editor.altPanel.live.base)),base);
  await a.locator('.te-alt-input:not(.te-alt-input--new):not([readonly])').press('Enter');
  const accepted=await state(a);assert.equal(accepted.annotations.spans[0].anchor.start,prefix.length);
  const saved=await checkpoint(a);
  b=await editorPage();await b.evaluate(source=>actual.openDocument(source),saved.presentationSource);
  await b.evaluate(saved=>TerraphimHistoryCheckpoint.restore(editor.surface,saved,saved.presentationSource,editor),saved);
  assert.equal((await state(b)).source,accepted.source);
  assert.equal(await b.evaluate(()=>editor.surface.undo()),true);
  assert.equal((await state(b)).body,prefix+'Risk needs a decision.\n');
  assert.equal(await b.evaluate(()=>editor.surface.redo()),true);
  assert.equal((await state(b)).source,accepted.source);
  assert.equal((await state(b)).annotations.spans[0].anchor.start,prefix.length);
 } finally {await a.close();await b?.close();}
});
test('real alternative swap and annotated move steps transfer in accepted order',async()=>{
 const a=await editorPage();let b;
 try {
  await startAlternative(a,'C');await a.locator('.te-alt-input:not(.te-alt-input--new):not([readonly])').pressSequentially('oncern');
  await a.locator('.te-alt-input:not(.te-alt-input--new):not([readonly])').press('Enter');
  const authored=await state(a);
  await a.evaluate(()=>{editor.altPanel.close();editor.swapAlternative(editor.documentApi().document_annotations().spans[0].id,0);});
  const swapped=await state(a);
  await a.evaluate(()=>editor.moveRange(0,4,editor.surface.getText().length));
  const moved=await state(a);const saved=await checkpoint(a);
  assert.ok(saved.history.some(entry=>entry.edits?.some(step=>step.swap)));
  assert.ok(saved.history.some(entry=>entry.edits?.some(step=>step.move)));
  b=await editorPage();await b.evaluate(source=>actual.openDocument(source),saved.presentationSource);
  await b.evaluate(saved=>TerraphimHistoryCheckpoint.restore(editor.surface,saved,saved.presentationSource,editor),saved);
  assert.equal(await b.evaluate(()=>editor.surface.undo()),true);assert.equal((await state(b)).source,swapped.source);
  assert.equal(await b.evaluate(()=>editor.surface.undo()),true);assert.equal((await state(b)).source,authored.source);
  assert.equal(await b.evaluate(()=>editor.surface.undo()),true);assert.equal((await state(b)).body,'Risk needs a decision.\n');
  for(let i=0;i<3;i++)assert.equal(await b.evaluate(()=>editor.surface.redo()),true);
  assert.equal((await state(b)).source,moved.source);
 }finally{await a.close();await b?.close();}
});
