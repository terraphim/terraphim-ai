// Actual browser DOM + production shared UI + real Rust/WASM in ResourceAuthority.
// The loopback transports native operations; it does not fabricate domain outcomes.
const {test,before,after}=require('node:test');
const assert=require('node:assert/strict');
const fs=require('node:fs/promises');const path=require('node:path');const os=require('node:os');
const http=require('node:http');const {createHash}=require('node:crypto');const {createRequire}=require('node:module');
const extension=process.env.TERRAPHIM_AUTHORITY_EXTENSION_DIR;
const assets=process.env.TERRAPHIM_AUTHORITY_ASSETS;
if(!extension||!assets)throw new Error('Set TERRAPHIM_AUTHORITY_EXTENSION_DIR and TERRAPHIM_AUTHORITY_ASSETS to actual built artifacts');
const extensionRequire=createRequire(path.join(extension,'package.json'));
const {ResourceAuthority}=extensionRequire('./out/resourceAuthority.js');
const {chromium}=extensionRequire('playwright-core');
const hash=value=>createHash('sha256').update(value).digest('hex');
let fixture,server,browser,origin;const authorities=[];let dispatch=0;
const role=JSON.stringify({name:'Controller fixture',data:{risk:{id:1,nterm:'risk',display_value:'Risk',pinned:false}}});
before(async()=>{
 fixture=await fs.mkdtemp(path.join(os.tmpdir(),'terraphim-real-controller-'));
 await fs.mkdir(path.join(fixture,'media','editor','roles'),{recursive:true});
 await fs.copyFile(path.join(extension,'media','resource-authority-worker.cjs'),path.join(fixture,'media','resource-authority-worker.cjs'));
 const manifest={};for(const name of ['terraphim_editor.js','terraphim_editor_bg.wasm']){const b=await fs.readFile(path.join(assets,name));manifest[name]=hash(b);await fs.writeFile(path.join(fixture,'media','editor',name),b);}
 const rb=await fs.readFile(path.join(assets,'roles','default.json'));manifest['roles/default.json']=hash(rb);await fs.writeFile(path.join(fixture,'media','editor','roles','default.json'),rb);
 await fs.writeFile(path.join(fixture,'media','editor-build.json'),JSON.stringify({protocol:1,assets:manifest}));
 server=http.createServer(async(req,res)=>{try{const url=new URL(req.url,'http://localhost');
  const file=url.pathname==='/history-checkpoint.js'?path.join(extension,'media','history-checkpoint.js'):path.resolve(assets,'.'+decodeURIComponent(url.pathname));
  if(url.pathname!=='/history-checkpoint.js'&&!file.startsWith(path.resolve(assets)+path.sep))throw new Error('outside fixture');
  const bytes=await fs.readFile(file);res.writeHead(200,{'content-type':file.endsWith('.wasm')?'application/wasm':file.endsWith('.js')?'text/javascript':'text/plain'});res.end(bytes);
 }catch{res.writeHead(404);res.end();}});
 await new Promise((resolve,reject)=>{server.once('error',reject);server.listen(0,'127.0.0.1',resolve);});origin=`http://127.0.0.1:${server.address().port}`;
 const candidates=[process.env.TERRAPHIM_BROWSER_EXECUTABLE,'/Applications/Google Chrome.app/Contents/MacOS/Google Chrome','/usr/bin/chromium'];let executablePath;
 for(const p of candidates.filter(Boolean)){try{await fs.access(p);executablePath=p;break;}catch{}}
 browser=await chromium.launch({headless:true,executablePath,timeout:15000});
});
after(async()=>{await browser?.close();for(const a of authorities)await a.dispose();if(server)await new Promise(r=>server.close(r));if(fixture)await fs.rm(fixture,{recursive:true,force:true});});
function base(s){return {resource:s.resource,authorityEpoch:s.authorityEpoch,revision:s.revision,sourceHash:s.sourceHash,historyGeneration:s.historyGeneration};}
async function setup(source='Risk needs a decision.\n',existing,authorityEnabled=true){
 let a=existing||await ResourceAuthority.create({extensionPath:fixture,resource:'file:///fixture-'+authorities.length+'.md',authorityEpoch:'opaque-'+authorities.length,revision:1,source,historyGeneration:0,roleJson:role});
 if(!existing)authorities.push(a);const view='view-'+(++dispatch);await a.attachView(view);let requestId=0;let lastPrepared;
 const page=await browser.newPage();page.setDefaultTimeout(10000);
 await page.exposeFunction('nativeOperation',async(op,value)=>{
  if(op==='snapshot')return a.snapshot();
  if(op==='recover'){a=await ResourceAuthority.create({extensionPath:fixture,resource:'file:///recovered.md',authorityEpoch:'fresh-'+(++dispatch),revision:1,source:value,historyGeneration:0,roleJson:role});authorities.push(a);await a.attachView(view);requestId=0;return a.snapshot();}
  if(op==='prepare'){const s=await a.snapshot(),request={...base(s),protocol:1,viewSession:view,requestId:++requestId,dispatchId:++dispatch};
   const {route,...command}=value;lastPrepared=route==='action'?await a.prepareAction(request,command):route==='history'?await a.prepareHistory(request,command):await a.prepareTrim(request,command);return lastPrepared;}
  if(op==='finish'||op==='abort'){const p=lastPrepared,s=await a.snapshot(),outcome=op==='finish'?await a.finish(p.request,p.token,p.afterHash):await a.abort(p.request,p.token);
   if(outcome.kind!==(op==='finish'?'committed':'guarded-aborted'))throw new Error(outcome.reason||outcome.kind);
   await a.ackOutcome(p.request,outcome,op==='finish'?{...base(s),sourceHash:p.afterHash,revision:s.revision+1,historyGeneration:s.historyGeneration+1}:base(s));return a.snapshot();}
  if(op==='history'){const s=await a.snapshot();return a.updateHistory(base(s),s.historyGeneration+1);}
  if(op==='query')return a.query(value.name,value.args);
  if(op==='control'){const result=await a.control(value);return {result,snapshot:await a.snapshot()};}
  throw new Error('Unsupported actual transport operation');
 });
 await page.goto(origin+'/terraphim-editor.min.js');await page.setContent('<!doctype html><div id="fixture"></div>');
 await page.addScriptTag({url:origin+'/terraphim-editor.min.js'});await page.addScriptTag({url:origin+'/history-checkpoint.js'});
 await page.evaluate(async({source,role,authorityEnabled})=>{
  window.actual=await TeraphimEditor.create(document.querySelector('#fixture'),{loadShoelace:false,value:source,dirtyTitle:false,config:{hostManaged:true,fileSystemAccess:false}});window.editor=actual.markdownEditor;actual.openDocument(source);
  editor.loadThesaurus(role);window.nativeSnapshot=await nativeOperation('snapshot');window.captured=null;window.drafts=[];
  if(!authorityEnabled)return;
  let queue=Promise.resolve();function serial(fn){const next=queue.then(fn);queue=next.catch(()=>{});return next;}
  editor.setAuthority({get roleGeneration(){return nativeSnapshot.roleGeneration;},get providerGeneration(){return nativeSnapshot.providerGeneration;},reportDraft(value){drafts.push(structuredClone(value));},
   transact(command,paint){return serial(async()=>{let prepared;try{prepared=await nativeOperation('prepare',command);}catch(error){if(/context exited/.test(error.message))error.authorityUncertain=true;throw error;}if(prepared.kind==='no-change'){window.nativeFixtureNoChangePending=true;if(window.nativeFixtureNoChangeDelay)await new Promise(resolve=>setTimeout(resolve,window.nativeFixtureNoChangeDelay));return null;}
    try{const started={...prepared.result,source:prepared.presentationSource,presentationSource:prepared.presentationSource};captured=await paint(started);if(window.nativeFixtureFailNextPaint){window.nativeFixtureFailNextPaint=false;throw new Error('Actual one-shot post-paint checkpoint failure');}if(window.nativeFixturePaintFailure)throw new Error('Actual post-paint checkpoint failure');if(window.nativeFixtureDelay)await new Promise(resolve=>setTimeout(resolve,window.nativeFixtureDelay));nativeSnapshot=await nativeOperation('finish');return started;}
    catch(error){window.nativeFixtureAbortPending=true;try{if(window.nativeFixtureAbortDelay)await new Promise(resolve=>setTimeout(resolve,window.nativeFixtureAbortDelay));nativeSnapshot=await nativeOperation('abort');error.authorityRolledBack=true;}catch{error.authorityUncertain=true;}finally{window.nativeFixtureAbortPending=false;}throw error;}});},
   historyChanged(){return serial(async()=>{captured=TerraphimHistoryCheckpoint.capture(editor.surface,editor.documentApi().save_document(),editor.altPanel);nativeSnapshot=await nativeOperation('history');});},
   query(name,args=[]){return serial(()=>nativeOperation('query',{name,args}));},control(value){return serial(async()=>{const r=await nativeOperation('control',value);nativeSnapshot=r.snapshot;return r.result;});}
  });
 },{source,role,authorityEnabled});return {page,a,view};
}
async function state(page){return page.evaluate(()=>({body:editor.surface.getText(),source:editor.documentApi().save_document(),history:editor.surface.historyIndex,canUndo:editor.surface.canUndo(),canRedo:editor.surface.canRedo(),drafts:editor.pendingAuthorityDraft(),status:editor.authorityStatus?.textContent??''}));}
async function settle(page){await page.evaluate(()=>editor.authoritySettled());}
test('actual controller accepts text, shared Undo/Redo and complete native source', {timeout:30000},async()=>{
 const {page,a}=await setup();try{
  await page.evaluate(()=>editor.surface.replaceRange(0,4,'Concern',{source:'test'}));await settle(page);
  assert.equal((await a.snapshot()).projection.body,'Concern needs a decision.\n');
  assert.equal(await page.evaluate(()=>editor.surface.undo()),true);assert.equal((await a.snapshot()).projection.body,'Risk needs a decision.\n');
  assert.equal(await page.evaluate(()=>editor.surface.redo()),true);assert.equal((await state(page)).body,(await a.snapshot()).projection.body);
 }finally{await page.close();}
});
test('actual typed alternative groups as one accepted addition and Escape preserves earlier addition', {timeout:60000},async()=>{
 const {page,a}=await setup();try{
  await page.evaluate(()=>editor.altPanel.open({start:0,end:4,kind:'word'}));
  await page.locator('.te-alt-input--new').pressSequentially('Concern');await settle(page);
  await page.evaluate(()=>editor.altPanel.finishLive());const savedA=(await a.snapshot()).source;
  assert.ok(savedA.includes('Concern'),JSON.stringify(await page.evaluate(()=>({state:editor.altPanel.pendingDrafts(),status:editor.altPanel.statusEl.textContent,live:!!editor.altPanel.live,source:editor.documentApi().save_document(),draft:editor.pendingAuthorityDraft()}))));await page.evaluate(()=>editor.altPanel.focusLine(editor.altPanel.lines().length));
  await page.locator('.te-alt-input--new').pressSequentially('Worry');await settle(page);
  await page.evaluate(()=>editor.altPanel.authorityCancelLive());await settle(page);
  assert.equal((await a.snapshot()).source,savedA);assert.equal((await state(page)).canRedo,false);
  await page.evaluate(()=>editor.surface.undo());assert.ok(!(await a.snapshot()).source.includes('Concern'));
  await page.evaluate(()=>editor.surface.redo());assert.equal((await a.snapshot()).source,savedA);
 }finally{await page.close();}
});
test('real protected Ghost receipt survives mixed edits and detached issuer through the same resource worker', {timeout:60000},async()=>{
 const body='Risk remains. Perhaps we might basically need to possibly review the very complex arrangement because the supplier could in some ways delay the launch.\n';
 const {page,a,view}=await setup(body);let other;
 try{
  await page.evaluate(()=>editor.authority.query('trim_plan_json'));
  const changed=await page.evaluate(()=>editor.commitTrimGhosts('sharper',[]));assert.ok(changed);
  const ghost=(await a.snapshot()).source;assert.ok((await a.snapshot()).projection.annotations.ghosts.length);
  await page.evaluate(()=>editor.surface.replaceRange(editor.surface.getText().length,editor.surface.getText().length,'Next.\n',{source:'test'}));
  await page.evaluate(()=>editor.surface.undo());assert.equal((await a.snapshot()).source,ghost);
  const cp=await page.evaluate(()=>TerraphimHistoryCheckpoint.capture(editor.surface,editor.documentApi().save_document(),editor.altPanel));
  await a.detachView(view);other=(await setup(body,a)).page;
  await other.evaluate(({source,cp})=>{actual.openDocument(source);TerraphimHistoryCheckpoint.restore(editor.surface,cp,source,editor);},{source:ghost,cp});
  assert.equal(await other.evaluate(()=>editor.surface.undo()),true);assert.equal((await a.snapshot()).projection.annotations.ghosts.length,0);
  assert.equal(await other.evaluate(()=>editor.surface.redo()),true);assert.equal((await a.snapshot()).source,ghost);
 }finally{await page.close();await other?.close();}
});

test('raw typing and Escape during delayed real acceptance preserve accepted A and cancel only B', {timeout:60000},async()=>{
 const {page,a}=await setup();try{
  await page.evaluate(()=>{editor.altPanel.open({start:0,end:4,kind:'word'});window.nativeFixtureDelay=80;});
  await page.locator('.te-alt-input--new').pressSequentially('Concern');await settle(page);await page.evaluate(()=>editor.altPanel.finishLive());
  const saved=(await a.snapshot()).source;await page.evaluate(()=>editor.altPanel.focusLine(editor.altPanel.lines().length));
  await page.locator('.te-alt-input--new').pressSequentially('Worry');await page.keyboard.press('Escape');await settle(page);
  assert.equal((await a.snapshot()).source,saved);assert.equal((await state(page)).canRedo,false);
  assert.equal((await state(page)).drafts.intents.length,0);
  await page.evaluate(()=>editor.altPanel.open({spanId:editor.annotations().spans[0].id}));await page.evaluate(()=>editor.altPanel.focusLine(editor.altPanel.lines().length));
  await page.locator('.te-alt-input--new').pressSequentially('Hazard');await settle(page);assert.ok((await a.snapshot()).source.includes('Hazard'));
 }finally{await page.close();}
});
test('body inputs later than a real pending native acceptance are retained and accepted in order', {timeout:30000},async()=>{
 const {page,a}=await setup();try{
  await page.evaluate(()=>{window.nativeFixtureDelay=80;editor.surface.focus();editor.surface.setSelectionOffsets(editor.surface.getText().length);});
  await page.keyboard.type('abc');await settle(page);
  assert.equal((await a.snapshot()).projection.body,'Risk needs a decision.\nabc');assert.equal((await state(page)).drafts.intents.length,0);
 }finally{await page.close();}
});
test('rapid Unicode body typing followed by consecutive Enter keeps every character and line break', {timeout:60000},async()=>{
 const {page,a}=await setup();try{
  await page.evaluate(()=>{window.nativeFixtureDelay=80;editor.surface.focus();editor.surface.setSelectionOffsets(0);});
  await page.keyboard.type('🙂 Public prefix.');await page.keyboard.press('Enter');await page.keyboard.press('Enter');
  await settle(page);
  assert.equal((await a.snapshot()).projection.body,'🙂 Public prefix.\n\nRisk needs a decision.\n');
  assert.equal((await state(page)).drafts.intents.length,0);
 }finally{await page.close();}
});
test('replacement paste uses a backward raw selection during delayed acceptance and preserves Unicode on reopen', {timeout:60000},async()=>{
 const {page,a}=await setup();let reopened;try{
  await page.evaluate(()=>{window.nativeFixtureDelay=80;editor.surface.focus();editor.surface.setSelectionOffsets(0);});
  await page.keyboard.type('🙂 draft ');
  await page.evaluate(()=>{const node=editor.surface.root.firstChild;document.getSelection().setBaseAndExtent(node,8,node,3);editor.surface.replaceSelection('Café\r\n🚀','paste');});
  await settle(page);
  const expected='🙂 Café\n🚀 Risk needs a decision.\n',saved=await a.snapshot();
  assert.equal(saved.projection.body,expected);assert.equal((await state(page)).drafts.intents.length,0);
  const next=await setup(saved.source);reopened=next.page;assert.equal((await next.a.snapshot()).projection.body,expected);
 }finally{await page.close();await reopened?.close();}
});
test('rapid Overflow input during real delayed native acceptance keeps the entire draft and saved source', {timeout:60000},async()=>{
 const {page,a}=await setup();try{
  const text='Public note https://terraphim.ai\n- Risk review\nRAPID-OVERFLOW-SENTINEL';
  await page.evaluate(async()=>{await editor.chrome.setMode('write-on',{persist:false});window.nativeFixtureDelay=80;if(!editor.overflow.open({focus:true}))throw new Error('Actual Overflow panel did not open');});
  await page.getByRole('textbox',{name:'Overflow text',exact:true}).pressSequentially(text,{delay:15});
  await settle(page);
  assert.equal(await page.getByRole('textbox',{name:'Overflow text',exact:true}).inputValue(),text);
  const saved=await a.snapshot();assert.equal(saved.projection.overflow,text);
  await page.evaluate(()=>editor.surface.focus());await settle(page);
  assert.equal((await a.snapshot()).source,saved.source);
 }finally{await page.close();}
});
test('retyping identical Overflow retires only the represented raw draft and remains savable', {timeout:60000},async()=>{
 const {page,a}=await setup();try{
  const text='Public note https://terraphim.ai\n- Risk review\nRAPID-OVERFLOW-SENTINEL';
  await page.evaluate(async()=>{await editor.chrome.setMode('write-on',{persist:false});editor.overflow.open({focus:true});});
  const input=page.getByRole('textbox',{name:'Overflow text',exact:true});await input.pressSequentially(text);await settle(page);const saved=(await a.snapshot()).source;
  await input.press(process.platform==='darwin'?'Meta+a':'Control+a');await input.pressSequentially(text);await settle(page);
  assert.equal((await a.snapshot()).source,saved);assert.equal((await state(page)).drafts.intents.length,0);assert.equal(await input.inputValue(),text);
 }finally{await page.close();}
});
test('a delayed real no-change Overflow response never clears a later raw draft', {timeout:60000},async()=>{
 const {page,a}=await setup();try{
  const text='Review supplier';await page.evaluate(async()=>{await editor.chrome.setMode('write-on',{persist:false});editor.overflow.open({focus:true});});
  const input=page.getByRole('textbox',{name:'Overflow text',exact:true});await input.pressSequentially(text);await settle(page);
  await page.evaluate(()=>{window.nativeFixtureNoChangeDelay=100;window.nativeFixtureNoChangePending=false;});
  await input.press(process.platform==='darwin'?'Meta+a':'Control+a');await input.pressSequentially(text);
  await page.evaluate(()=>{window.noChangeFlush=editor.overflow.flush();});await page.waitForFunction(()=>window.nativeFixtureNoChangePending);
  await input.pressSequentially(' later');await page.evaluate(()=>window.noChangeFlush);
  assert.equal((await state(page)).drafts.intents.find(x=>x.key==='overflow').input.text,text+' later');
  await settle(page);assert.equal((await a.snapshot()).projection.overflow,text+' later');assert.equal((await state(page)).drafts.intents.length,0);
 }finally{await page.close();}
});
test('Overflow copy and guarded stash replay through actual native actions', {timeout:30000},async()=>{
 const {page,a}=await setup();try{
  await page.evaluate(async()=>{editor.overflow.open({focus:true});editor.overflow.area.value='Review supplier';editor.overflow.schedule();await editor.overflow.flush();});await settle(page);
  assert.equal((await a.snapshot()).projection.overflow,'Review supplier');
  await page.evaluate(()=>editor.overflow.pullBackAt(0,{start:0,end:6,text:'Review'}));await settle(page);
  assert.equal((await a.snapshot()).projection.overflow,'Review supplier');assert.ok((await a.snapshot()).projection.body.includes('Review'));
  await page.evaluate(()=>editor.surface.undo());assert.equal((await a.snapshot()).projection.body,'Risk needs a decision.\n');
  await page.evaluate(()=>editor.stashRange(0,4));assert.equal((await a.snapshot()).projection.body,' needs a decision.\n');
  await page.evaluate(()=>editor.surface.undo());assert.equal((await a.snapshot()).projection.body,'Risk needs a decision.\n');assert.equal((await a.snapshot()).projection.overflow,'Review supplier');
 }finally{await page.close();}
});
test('actual batch, move and alternative edit/reorder/remove preserve exact replay metadata', {timeout:30000},async()=>{
 const {page,a}=await setup();try{
  await page.evaluate(()=>editor.surface.replaceRanges([{start:0,end:4,insert:'Concern'},{start:13,end:21,insert:'choice'}],{source:'test'}));
  assert.equal((await a.snapshot()).projection.body,'Concern needs a choice.\n');await page.evaluate(()=>editor.surface.undo());
  await page.evaluate(()=>editor.moveRange(0,4,21));const moved=(await a.snapshot()).source;assert.ok((await a.snapshot()).projection.body.includes('Risk'));
  await page.evaluate(()=>editor.surface.undo());assert.equal((await a.snapshot()).projection.body,'Risk needs a decision.\n');await page.evaluate(()=>editor.surface.redo());assert.equal((await a.snapshot()).source,moved);
  await page.evaluate(()=>editor.surface.undo());
  const id=await page.evaluate(async()=>{const c=await editor.alternativeOp('create_span','word',0,4,'Concern','human');return c.span;});
  await page.evaluate(id=>editor.alternativeOp('add',id,'Worry','human'),id);await page.evaluate(id=>editor.alternativeOp('edit',id,2,'Hazard'),id);
  await page.evaluate(id=>editor.alternativeOp('move',id,2,1),id);await page.evaluate(id=>editor.alternativeOp('remove',id,1),id);
  await page.evaluate(()=>editor.surface.undo());assert.ok((await a.snapshot()).source.includes('Hazard'));
  await page.evaluate(()=>editor.surface.redo());assert.ok(!(await a.snapshot()).source.includes('Hazard'));
 }finally{await page.close();}
});
test('actual role publication preserves issued Ghost authority and provider changes reach native owner', {timeout:30000},async()=>{
 const body='Risk remains. Perhaps we might basically need to possibly review the very complex arrangement because the supplier could in some ways delay the launch.\n';
 const {page,a}=await setup(body);try{
  await page.evaluate(()=>editor.authority.query('trim_plan_json'));await page.evaluate(()=>editor.commitTrimGhosts('sharper',[]));const saved=(await a.snapshot()).source;
  await page.evaluate(()=>editor.loadThesaurus(JSON.stringify({name:'New role',data:{supplier:{id:2,nterm:'supplier'}}})));
  await page.evaluate(()=>editor.surface.undo());assert.equal((await a.snapshot()).projection.annotations.ghosts.length,0);
  await page.evaluate(()=>editor.surface.redo());assert.equal((await a.snapshot()).source,saved);
  await page.evaluate(()=>editor.registerAlternativesProvider({id:'actual-fixture',model:'local-test',alternatives:async()=>[]}));
  assert.ok((await a.query('alternatives_provider_ids')).includes('actual-fixture'));
  await page.evaluate(()=>editor.unregisterAlternativesProvider('actual-fixture'));assert.ok(!(await a.query('alternatives_provider_ids')).includes('actual-fixture'));
 }finally{await page.close();}
});

test('Enter during real delayed acceptance separates the next addition without losing raw typing', {timeout:60000},async()=>{
 const {page,a}=await setup();try{
  await page.evaluate(()=>{editor.altPanel.open({start:0,end:4,kind:'word'});window.nativeFixtureDelay=80;});
  await page.locator('.te-alt-input--new').pressSequentially('Concern');await page.keyboard.press('Enter');await page.keyboard.type('Worry');await settle(page);
  const spans=(await a.snapshot()).projection.annotations.spans;assert.deepEqual(spans[0].alts.map(x=>x.text),['Risk','Concern','Worry']);
  await page.keyboard.press('Escape');await settle(page);assert.deepEqual((await a.snapshot()).projection.annotations.spans[0].alts.map(x=>x.text),['Risk','Concern']);
 }finally{await page.close();}
});

test('real guarded abort restores tentative Surface/history without claiming acceptance', {timeout:30000},async()=>{
 const {page,a}=await setup();try{
  const before=await a.snapshot();const prior=await state(page);await page.evaluate(()=>window.nativeFixturePaintFailure=true);
  await assert.rejects(page.evaluate(()=>editor.surface.replaceRange(0,4,'Hazard',{source:'test'})),/post-paint/);
  assert.equal((await a.snapshot()).source,before.source);assert.equal((await state(page)).source,prior.source);assert.equal((await state(page)).history,prior.history);
  await page.evaluate(()=>window.nativeFixturePaintFailure=false);await page.evaluate(()=>editor.surface.replaceRange(0,4,'Concern',{source:'test'}));assert.equal((await a.snapshot()).projection.body,'Concern needs a decision.\n');
 }finally{await page.close();}
});
test('actual worker crash blocks mutations until explicit fresh-authority recovery resets shared controller', {timeout:30000},async()=>{
 const {page,a}=await setup();try{
  await page.evaluate(()=>editor.surface.replaceRange(0,4,'Concern',{source:'test'}));const saved=(await a.snapshot()).source;
  await a.worker.terminate();await assert.rejects(page.evaluate(()=>editor.surface.replaceRange(0,7,'Hazard',{source:'test'})),/context exited/);
  await assert.rejects(settle(page),/context exited/);
  await page.evaluate(async source=>{nativeSnapshot=await nativeOperation('recover',source);actual.openDocument(nativeSnapshot.presentationSource);editor.surface.resetHistory();editor.resetAuthorityAfterRecovery();},saved);
  await settle(page);assert.equal((await state(page)).canUndo,false);
  await page.evaluate(()=>editor.surface.replaceRange(0,7,'Hazard',{source:'test'}));assert.equal((await state(page)).body,'Hazard needs a decision.\n');
 }finally{await page.close();}
});
test('real Trim preview and Done controls query native authority, then manual Ghost and revive remain native', {timeout:30000},async()=>{
 const body='Risk remains. Perhaps we might basically need to possibly review the very complex arrangement because the supplier could in some ways delay the launch.\n';
 const {page,a}=await setup(body);try{
  const status=await page.evaluate(()=>editor.lab.trim.select('sharper'));assert.ok(status?.kgProtected,JSON.stringify(await page.evaluate(()=>editor.lab.trim.protectionNote.textContent)));
  assert.equal(await page.evaluate(()=>editor.lab.trim.done()),true);assert.ok((await a.snapshot()).projection.annotations.ghosts.length);
  await page.evaluate(()=>editor.surface.undo());assert.equal((await a.snapshot()).projection.annotations.ghosts.length,0);
  await page.evaluate(()=>editor.ghosts.change('ghost',0,4));assert.equal((await a.snapshot()).projection.annotations.ghosts.length,1);
  await page.evaluate(()=>editor.ghosts.change('revive',0,4));assert.equal((await a.snapshot()).projection.annotations.ghosts.length,0);
 }finally{await page.close();}
});

test('actual Lab acceptance waits for native finish and failed checkpoint retains proposal', {timeout:30000},async()=>{
 const {page,a}=await setup('We recieved the report.\n');try{
  const mark=await page.evaluate(()=>{editor.lab.run('typos_and_punctuation');return editor.lab.marks().find(m=>m.data.proposal);});assert.ok(mark);
  const before=(await a.snapshot()).source;await page.evaluate(()=>window.nativeFixturePaintFailure=true);
  assert.equal(await page.evaluate(id=>editor.lab.accept(id),mark.id),false);assert.equal((await a.snapshot()).source,before);
  assert.ok(await page.evaluate(id=>editor.lab.marks().some(m=>m.id===id),mark.id));
  await page.evaluate(()=>window.nativeFixturePaintFailure=false);assert.equal(await page.evaluate(id=>editor.lab.accept(id),mark.id),true);
  assert.equal((await a.snapshot()).projection.body,'We received the report.\n');
 }finally{await page.close();}
});
module.exports={setup,state,settle};
test('actual Blocks commit and view change wait for delayed native acceptance', {timeout:30000},async()=>{
 const {page,a}=await setup('First block.\n\nSecond block.\n');try{
  await page.evaluate(async()=>{await editor.chrome.setMode('plain');await editor.blocks.setView('blocks',{persist:false});await editor.blocks.editBlock(0);window.nativeFixtureDelay=80;});
  await page.locator('.te-block-editor').fill('Changed first.');
  const result=await page.evaluate(async()=>{const committed=editor.blocks.commitEdit();const switched=editor.blocks.setView('text',{persist:false});const selection=await committed;await switched;return selection;});
  await settle(page);assert.equal((await a.snapshot()).projection.body,'Changed first.\n\nSecond block.\n');assert.ok(Number.isFinite(result.start));
  assert.equal((await state(page)).drafts.intents.length,0);
 }finally{await page.close();}
});
test('actual Blocks failed native checkpoint retains open textarea and accepts explicit retry', {timeout:30000},async()=>{
 const {page,a}=await setup('First block.\n\nSecond block.\n');try{
  await page.evaluate(async()=>{await editor.chrome.setMode('plain');await editor.blocks.setView('blocks',{persist:false});await editor.blocks.editBlock(0);});
  await page.locator('.te-block-editor').fill('Changed first.');const before=(await a.snapshot()).source;const prior=await state(page);
  await page.evaluate(()=>window.nativeFixturePaintFailure=true);await assert.rejects(page.evaluate(()=>editor.blocks.commitEdit()),/post-paint/);
  assert.equal((await a.snapshot()).source,before);assert.equal((await state(page)).history,prior.history);assert.equal(await page.locator('.te-block-editor').inputValue(),'Changed first.');
  await page.evaluate(()=>window.nativeFixturePaintFailure=false);await page.evaluate(()=>editor.blocks.commitEdit());await settle(page);
  assert.equal((await a.snapshot()).projection.body,'Changed first.\n\nSecond block.\n');
 }finally{await page.close();}
});
test('actual Blocks structural move remains one Undo step and retained draft retires only on acceptance', {timeout:30000},async()=>{
 const {page,a}=await setup('First block.\n\nSecond block.\n');try{
  await page.evaluate(async()=>{await editor.chrome.setMode('plain');await editor.blocks.setView('blocks',{persist:false});});
  const initial=(await a.snapshot()).source;assert.equal(await page.evaluate(()=>editor.blocks.moveBlock(0,'down')),true);const moved=(await a.snapshot()).source;assert.notEqual(moved,initial);
  await page.evaluate(()=>editor.blocks.undo());assert.equal((await a.snapshot()).source,initial);await page.evaluate(()=>editor.blocks.redo());assert.equal((await a.snapshot()).source,moved);
  await page.evaluate(()=>editor.blocks.editBlock(0));await page.locator('.te-block-editor').fill('Kept draft.');const id=await page.evaluate(()=>editor.blocks.cancelEdit().id);
  await page.evaluate(()=>window.nativeFixturePaintFailure=true);await assert.rejects(page.evaluate(id=>editor.blocks.applyDraft(id),id),/post-paint/);
  assert.equal(await page.evaluate(id=>editor.blocks.drafts.some(d=>d.id===id),id),true);
  await page.evaluate(()=>window.nativeFixturePaintFailure=false);await page.evaluate(id=>editor.blocks.applyDraft(id),id);await settle(page);
  assert.equal(await page.evaluate(id=>editor.blocks.drafts.some(d=>d.id===id),id),false);assert.ok((await a.snapshot()).projection.body.includes('Kept draft.'));
 }finally{await page.close();}
});

test('standalone editor retains synchronous native mutations and whole-addition history', {timeout:30000},async()=>{
 const {page}=await setup('Risk needs a decision.\n',undefined,false);try{
  const native=await page.evaluate(()=>{const edit=editor.surface.replaceRange(0,4,'Concern');const undo=editor.surface.undo();return {promise:!!edit?.then,undo,body:editor.documentApi().document_body()};});
  assert.deepEqual(native,{promise:false,undo:true,body:'Risk needs a decision.\n'});
  await page.evaluate(()=>editor.altPanel.open({start:0,end:4,kind:'word'}));await page.locator('.te-alt-input--new').pressSequentially('Concern');
  await page.evaluate(()=>editor.altPanel.finishLive());assert.ok((await state(page)).source.includes('Concern'));
  assert.equal(await page.evaluate(()=>editor.surface.undo()),true);assert.ok(!(await state(page)).source.includes('Concern'));
 }finally{await page.close();}
});

test('native duplicate live edit stays a recoverable raw draft until Escape cancels B', {timeout:30000},async()=>{
 const {page,a}=await setup();try{
  await page.evaluate(()=>editor.altPanel.open({start:0,end:4,kind:'word'}));await page.locator('.te-alt-input--new').pressSequentially('Concern');await settle(page);await page.evaluate(()=>editor.altPanel.finishLive());const saved=(await a.snapshot()).source;
  await page.evaluate(()=>editor.altPanel.focusLine(editor.altPanel.lines().length));await page.locator('.te-alt-input--new').pressSequentially('Worry');await settle(page);
  await page.locator('.te-alt-line[data-index="2"] input').fill('Concern');await page.evaluate(()=>editor.altPanel.authorityLineQueue);
  await assert.rejects(settle(page),/Unaccepted raw input/);assert.ok((await a.snapshot()).source.includes('Worry'));
  await page.keyboard.press('Enter');assert.ok((await state(page)).drafts.intents.some(d=>d.input.text==='Concern'));
  await page.keyboard.press('Escape');await settle(page);assert.equal((await a.snapshot()).source,saved);assert.equal((await state(page)).canRedo,false);
 }finally{await page.close();}
});
test('duplicate new alternative records no phantom history and Escape discards only refused raw input', {timeout:30000},async()=>{
 const {page,a}=await setup();try{
  await page.evaluate(()=>editor.altPanel.open({start:0,end:4,kind:'word'}));await page.locator('.te-alt-input--new').pressSequentially('Concern');await settle(page);await page.evaluate(()=>editor.altPanel.finishLive());
  const saved=(await a.snapshot()).source,prior=(await state(page)).history;await page.evaluate(()=>editor.altPanel.focusLine(editor.altPanel.lines().length));
  await page.locator('.te-alt-input--new').fill('Concern');await page.evaluate(()=>editor.altPanel.authorityLineQueue);
  assert.equal((await a.snapshot()).source,saved);assert.equal((await state(page)).history,prior);
  await page.keyboard.press('Escape');await settle(page);assert.equal((await a.snapshot()).source,saved);assert.equal((await state(page)).drafts.intents.length,0);
 }finally{await page.close();}
});

test('actual Write On switch retains Blocks textarea and plain mode until native commit accepts', {timeout:30000},async()=>{
 const {page,a}=await setup('First block.\n\nSecond block.\n');try{
  await page.evaluate(async()=>{await editor.chrome.setMode('plain');await editor.blocks.setView('blocks',{persist:false});await editor.blocks.editBlock(0);window.nativeFixtureDelay=120;});
  await page.locator('.te-block-editor').fill('Changed first.');
  await page.evaluate(()=>{window.modeSwitch=editor.chrome.setMode('write-on',{persist:false});});
  assert.deepEqual(await page.evaluate(()=>({mode:editor.chrome.mode,editing:!!editor.blocks.editing,connected:!!document.querySelector('.te-block-editor')})),{mode:'plain',editing:true,connected:true});
  await page.evaluate(()=>window.modeSwitch);await settle(page);assert.equal((await a.snapshot()).projection.body,'Changed first.\n\nSecond block.\n');assert.equal(await page.evaluate(()=>editor.chrome.mode),'write-on');
 }finally{await page.close();}
});
test('actual Write On switch refuses failed native commit without hiding Blocks textarea', {timeout:30000},async()=>{
 const {page,a}=await setup('First block.\n\nSecond block.\n');try{
  await page.evaluate(async()=>{await editor.chrome.setMode('plain');await editor.blocks.setView('blocks',{persist:false});await editor.blocks.editBlock(0);});
  await page.locator('.te-block-editor').fill('Changed first.');const before=(await a.snapshot()).source;
  await page.evaluate(()=>window.nativeFixturePaintFailure=true);await assert.rejects(page.evaluate(()=>editor.chrome.setMode('write-on',{persist:false})),/post-paint/);
  assert.equal((await a.snapshot()).source,before);assert.equal(await page.evaluate(()=>editor.chrome.mode),'plain');assert.equal(await page.locator('.te-block-editor').inputValue(),'Changed first.');
  await page.evaluate(()=>window.nativeFixturePaintFailure=false);await page.evaluate(()=>editor.chrome.setMode('write-on',{persist:false}));assert.equal(await page.evaluate(()=>editor.chrome.mode),'write-on');
 }finally{await page.close();}
});
test('real graph Worker prepared-role publication commits through the native authority coordinator', {timeout:30000},async()=>{
 const {page,a}=await setup();try{
  await page.evaluate(()=>editor.rolegraph.loadRole(JSON.stringify({name:'Graph coordinated',data:{concern:{id:7,nterm:'concern',display_value:'Concern',pinned:false}}})));
  const raw=await a.query('kg_role_snapshot_json');const roleState=typeof raw==='string'?JSON.parse(raw):raw;assert.equal(roleState.loaded,true);assert.ok(roleState.thesaurusText.includes('concern'));
  const local=await page.evaluate(()=>JSON.parse(editor.documentApi().kg_role_snapshot_json()));assert.equal(local.thesaurusText,roleState.thesaurusText);
 }finally{await page.close();}
});
test('authored alternative exports real typed KG synonyms and Undo removes actual graph registration', {timeout:30000},async()=>{
 const {page,a}=await setup();try{
  const id=await page.evaluate(async()=>{const c=await editor.alternativeOp('create_span','word',0,4,'Concern','human');return c.span;});
  const graph=JSON.parse(await a.query('export_knowledge_graph'));assert.ok(graph.data.risk);assert.ok(graph.data.concern);assert.equal(graph.data.risk.id,graph.data.concern.id);
  const rawSynonyms=await a.query('document_synonyms',[id,'Concern']);const synonyms=typeof rawSynonyms==='string'?JSON.parse(rawSynonyms):rawSynonyms;assert.ok(Array.from(synonyms).some(term=>String(term).toLowerCase()==='risk'));
  await page.evaluate(()=>editor.surface.undo());assert.equal(Object.keys(JSON.parse(await a.query('export_knowledge_graph')).data).length,0);
  await page.evaluate(()=>editor.surface.redo());assert.deepEqual(JSON.parse(await a.query('export_knowledge_graph')),graph);
 }finally{await page.close();}
});

test('actual public brief Walk Keep Cuts Undo then Done preserves protected history', {timeout:60000},async()=>{
 const body=await fs.readFile(process.env.TERRAPHIM_TRIM_REGRESSION_FIXTURE||path.resolve(__dirname,'../../crates/terraphim_lab/tests/fixtures/public-project-brief.md'),'utf8');
 const role=await fs.readFile(path.join(process.env.TERRAPHIM_AUTHORITY_ASSETS,'roles/default.json'),'utf8');
 const output=process.env.TERRAPHIM_TRIM_REGRESSION_OUTPUT||path.join(await fs.mkdtemp(path.join(os.tmpdir(),'terraphim-trim-controller-')),'outputs');await fs.mkdir(output,{recursive:false});
 const {page,a}=await setup(body);const receipt={scope:'Real component DOM/controllers + actual native worker; not installed-host UI save evidence',checks:[]};
 try{
  await page.evaluate(async role=>{await editor.authority.control({kind:'role-load',json:role});await editor.chrome.setMode('write-on',{persist:false});},role);
  const initial=await a.snapshot();
  for(const action of ['typos_and_punctuation','weakest_sentences','long_sentences','convoluted_sentences','off_tone','hedges_and_filler'])await page.evaluate(action=>editor.lab.run(action),action);
  for(const level of ['original','slight','tighten','sharper','half'])await page.evaluate(level=>editor.lab.trim.select(level),level);
  const first=await page.evaluate(()=>({plan:editor.lab.trim.plan,status:editor.lab.trim.status}));assert.equal(first.plan.totalWords,111);assert.equal(first.plan.protectionCount,1);assert.equal((await a.snapshot()).source,initial.source);receipt.checks.push('Six marks and five levels preserve111-word source/protectedCount1');
  await page.locator('.te-trim-walk-start').click();await page.locator('.te-trim-next').click();await page.locator('.te-trim-prev').click();await page.locator('.te-trim-keep').click();
  await page.waitForFunction(()=>editor.lab.trim.kept.has(0)&&editor.lab.trim.active.every(c=>c.id!==0));await page.locator('.te-trim-stop').click();
  const clicked=await page.evaluate(()=>{const el=Array.from(editor.surface.root.querySelectorAll('[data-te-decoration]')).find(el=>el.textContent.trim()==='maybe');if(!el)return false;el.click();return true;});assert.equal(clicked,true,'Actual faded maybe segment must exist');
  await page.waitForFunction(()=>editor.lab.trim.kept.has(1)&&editor.lab.trim.active.every(c=>c.id!==1));
  const expected=await a.query('trim_make_cuts',['half','[0,1]']);await page.locator('.te-trim-cut').click();await page.waitForFunction(expected=>editor.surface.getText()===expected,expected.text);await settle(page);
  const cut=await a.snapshot();assert.equal(cut.projection.body,expected.text);await fs.writeFile(path.join(output,'actual-make-cuts.md'),cut.source);receipt.checks.push('Actual Walk/Keep/faded-Keep MakeCuts matches native result');
  assert.equal(await page.evaluate(()=>editor.surface.undo()),true);await settle(page);assert.equal((await a.snapshot()).source,initial.source);await fs.writeFile(path.join(output,'actual-cuts-undo.md'),(await a.snapshot()).source);receipt.checks.push('Whole Cuts Undo restores exact original716 bytes');
  await page.evaluate(async()=>{editor.lab.open();await editor.lab.trim.select('half');});
  await fs.writeFile(path.join(output,'actual-before-done.json'),JSON.stringify(await page.evaluate(()=>({plan:editor.lab.trim.plan,status:editor.lab.trim.status})),null,2)+'\n');
  const done=await page.evaluate(()=>editor.lab.trim.done());receipt.done=done;receipt.live=await page.evaluate(()=>editor.lab.trim.live.textContent);await settle(page);
  const promoted=await a.snapshot();await fs.writeFile(path.join(output,'actual-done.md'),promoted.source);assert.equal(done,true,receipt.live);assert.equal(promoted.projection.body,body);assert.ok(promoted.projection.annotations.ghosts.length);receipt.checks.push('Done promotes independent actual persistent ghosts without changing body');
  assert.equal(await page.evaluate(()=>editor.surface.undo()),true);assert.equal((await a.snapshot()).source,initial.source);assert.equal(await page.evaluate(()=>editor.surface.redo()),true);assert.equal((await a.snapshot()).source,promoted.source);receipt.checks.push('Whole protected Done Undo/Redo retains exact source and native receipt');receipt.status='PASS';
 }catch(error){receipt.status='FAIL';receipt.error=error.stack;throw error;}
 finally{await fs.writeFile(path.join(output,'receipt.json'),JSON.stringify(receipt,null,2)+'\n');await page.close();}
});


test('native Ghost text mapping survives real typing Undo Redo and saved-source reopening', {timeout:60000},async()=>{
 const body='Risk needs mitigation.\n\nThe public project needs careful review.\n';
 for(const mode of ['burst','tail']){
  const {page,a}=await setup(body);let reopened;
  try{
   await page.evaluate(()=>{window.ghostInputTrace=[];for(const name of ['beforeinput','input'])editor.surface.root.addEventListener(name,event=>{const selection=document.getSelection();ghostInputTrace.push({type:name,trusted:event.isTrusted,inputType:event.inputType,data:event.data,text:editor.surface.root.textContent,selection:selection?{text:selection.toString(),anchor:selection.anchorOffset,focus:selection.focusOffset}:null,ranges:event.getTargetRanges?Array.from(event.getTargetRanges(),range=>({start:range.startOffset,end:range.endOffset})):[],sequence:editor.authorityDraftSequence,painting:editor.authorityPainting});},{capture:true,passive:true});});
   await page.evaluate(()=>editor.ghosts.ghost(0,4));await settle(page);
   const before=await a.snapshot();
   await page.evaluate(mode=>{editor.surface.focus();editor.surface.setSelectionOffsets(mode==='tail'?3:1,4);},mode);
   if(mode==='burst')await page.keyboard.type('isk-long');else await page.keyboard.press('Backspace');
   await settle(page);const after=await a.snapshot();
   if(process.env.TERRAPHIM_GHOST_HISTORY_OUTPUT){await fs.mkdir(process.env.TERRAPHIM_GHOST_HISTORY_OUTPUT,{recursive:true});await fs.writeFile(path.join(process.env.TERRAPHIM_GHOST_HISTORY_OUTPUT,mode+'.json'),JSON.stringify({scope:'Real headless controller with actual native owner; not native Mac UI',before,after,events:await page.evaluate(()=>ghostInputTrace)},null,2)+'\n');}
   assert.equal(after.projection.body,mode==='burst'?body.replace('Risk','Risk-long'):body.replace('Risk','Ris'));
   assert.equal(await page.evaluate(()=>editor.surface.undo()),true);await settle(page);assert.equal((await a.snapshot()).source,before.source);
   assert.equal(await page.evaluate(()=>editor.surface.redo()),true);await settle(page);assert.equal((await a.snapshot()).source,after.source);
   const next=await setup(after.source);reopened=next.page;
   assert.equal((await next.a.snapshot()).source,after.source);
   assert.equal(await reopened.evaluate(()=>editor.surface.getText()),after.projection.body);
  }finally{await reopened?.close();await page.close();}
 }
});


test('native Ghost receipts follow exact batch order through no-ops and a fresh view', {timeout:60000},async()=>{
 const {page,a,view}=await setup('Risk needs mitigation.\n');let other;
 try{
  await page.evaluate(()=>editor.ghosts.ghost(0,4));await settle(page);const before=await a.snapshot();
  await page.evaluate(()=>editor.surface.replaceRanges([{start:4,end:4,insert:'1'},{start:4,end:4,insert:'2'},{start:0,end:1,insert:'R'},{start:3,end:4,insert:''}],{source:'test-batch'}));await settle(page);
  const after=await a.snapshot();assert.equal(after.projection.body,'Ris12 needs mitigation.\n');
  const checkpoint=await page.evaluate(()=>TerraphimHistoryCheckpoint.capture(editor.surface,editor.documentApi().save_document(),editor.altPanel));
  await a.detachView(view);other=(await setup(after.source,a)).page;
  await other.evaluate(({checkpoint,source})=>TerraphimHistoryCheckpoint.restore(editor.surface,checkpoint,source,editor),{checkpoint,source:after.source});
  assert.equal(await other.evaluate(()=>editor.surface.undo()),true);await settle(other);assert.equal((await a.snapshot()).source,before.source);
  assert.equal(await other.evaluate(()=>editor.surface.redo()),true);await settle(other);assert.equal((await a.snapshot()).source,after.source);
 }finally{await other?.close();await page.close();}
});
// Append-only proposed regressions for the existing actual-controller fixture.
// Uses actual Rust ResourceAuthority/production controller and a delayed REAL finish,
// never an invented native response. setup/settle/state/assert/test are canonical fixture bindings.
test('decoration-only refresh preserves pending real native body text and DOM caret', {timeout:60000},async()=>{
 const body='Risk remains.\n';const {page,a}=await setup(body);
 try{
  await page.evaluate(()=>editor.ghosts.ghost(0,4));await settle(page);
  await page.evaluate(()=>{window.nativeFixtureDelay=250;editor.surface.focus();editor.surface.setSelectionOffsets(1,4);});
  await page.keyboard.type('isk-long');
  await page.waitForFunction(()=>{
   const draft=editor.authorityDrafts.get('body');
   return draft&&draft.input.text!==editor.surface.getText()&&editor.surface.serialise().text===draft.input.text;
  });
  const observation=await page.evaluate(()=>{
   const read=()=>({text:editor.surface.serialise().text,selection:editor.surface.readSelection(),draft:structuredClone(editor.authorityDrafts.get('body')),accepted:editor.surface.getText()});
   const before=read();
   if (!before.draft || before.text === before.accepted || before.text !== before.draft.input.text) throw new Error('Pending raw decoration precondition changed before actual apply');
   const renderCount=editor.decorations.renderCount, batchDepth=editor.decorations.batchDepth;
   // Real production registry application of an ordinary cosmetic layer. It
   // exercises the same setDecorations/render path as a scheduled Ghost refresh.
   editor.decorations.set('raw-paint-regression',[{id:'pending',start:0,end:1,className:'te-regression-decoration'}]);
   return {before,after:read(),renderCountBefore:renderCount,renderCountAfter:editor.decorations.renderCount,batchDepth};
  });
  console.log('actual-decoration-observation '+JSON.stringify(observation));
  assert.equal(observation.after.text,observation.before.text,'decoration paint discarded raw native input');
  assert.deepEqual(observation.after.selection,observation.before.selection,'decoration paint moved raw native caret');
  assert.deepEqual(observation.after.draft,observation.before.draft,'cosmetic refresh changed the native proposal');
  await settle(page);assert.equal((await a.snapshot()).projection.body,'Risk-long remains.\n');
 }finally{await page.close();}
});

test('real ArrowLeft during pending native body acceptance survives paint and next character', {timeout:60000},async()=>{
 const body='Risk remains.\n';const {page,a}=await setup(body);
 try{
  await page.evaluate(()=>editor.ghosts.ghost(0,4));await settle(page);
  await page.evaluate(()=>{window.nativeFixtureDelay=250;editor.surface.focus();editor.surface.setSelectionOffsets(1,4);});
  await page.keyboard.type('isk-long');
  await page.waitForFunction(()=>{
   const draft=editor.authorityDrafts.get('body');
   return draft&&draft.input.text==='Risk-long remains.\n'&&draft.input.text!==editor.surface.getText()&&editor.surface.serialise().text===draft.input.text;
  });
  await page.keyboard.press('ArrowLeft');
  const moved=await page.evaluate(()=>editor.surface.readSelection());
  assert.equal(moved.start,8);assert.equal(moved.end,8);
  await settle(page);
  const after=await page.evaluate(()=>({text:editor.surface.serialise().text,selection:editor.surface.readSelection()}));
  assert.equal(after.text,'Risk-long remains.\n');
  assert.deepEqual(after.selection,moved,'accepted paint restored the older input-event caret');
  await page.keyboard.type('X');await settle(page);
  assert.equal((await a.snapshot()).projection.body,'Risk-lonXg remains.\n');
 }finally{await page.close();}
});

test('guarded real native abort preserves newer actual caret and raw body draft', {timeout:60000},async()=>{
 const body='Risk remains.\n';const {page,a}=await setup(body);
 try {
  await page.evaluate(()=>editor.ghosts.ghost(0,4));await settle(page);
  const baseline=await a.snapshot();
  await page.evaluate(()=>{
   window.nativeFixtureFailNextPaint=true;
   window.nativeFixtureAbortDelay=1200;
   editor.surface.focus();editor.surface.setSelectionOffsets(1,4);
  });
  await page.keyboard.type('Q');
  await page.waitForFunction(()=>window.nativeFixtureAbortPending===true);
  const pending=await page.evaluate(()=>({
   raw:editor.surface.serialise().text,
   draft:structuredClone(editor.authorityDrafts.get('body')),
   selection:editor.surface.readSelection()
  }));
  assert.equal(pending.raw,'RQ remains.\n');
  assert.equal(pending.draft.input.text,pending.raw);
  assert.equal(pending.selection.start,2);assert.equal(pending.selection.end,2);
  // This is actual browser keyboard selection movement WHILE a REAL guarded
  // native abort is waiting. No selection setter, synthetic key or snapshot edit.
  await page.keyboard.press('ArrowLeft');
  const moved=await page.evaluate(()=>editor.surface.readSelection());
  assert.equal(moved.start,1);assert.equal(moved.end,1);
  // The public queue resolves its rejection handler; authoritySettled correctly
  // refuses the still-unaccepted draft and must not be used as a success oracle.
  await page.evaluate(()=>editor.authorityQueue);
  await page.waitForFunction(()=>window.nativeFixtureAbortPending===false);
  const after=await page.evaluate(()=>({
   raw:editor.surface.serialise().text,accepted:editor.surface.getText(),
   draft:structuredClone(editor.authorityDrafts.get('body')),
   selection:editor.surface.readSelection(),fault:!!editor.authorityFault,
   pendingDecorations:!!editor.surface.authorityDecorationsPending
  }));
  assert.equal((await a.snapshot()).source,baseline.source,
   'guarded abort changed actual accepted native source/annotations');
  assert.equal(after.accepted,body);
  assert.equal(after.raw,pending.raw,'rollback lost later raw text');
  assert.equal(after.draft.sequence,pending.draft.sequence);
  assert.equal(after.draft.input.text,pending.raw);
  assert.deepEqual(after.selection,moved,'rollback restored stale input-event caret');
  assert.equal(after.fault,false,'confirmed abort incorrectly retired native authority');
  // The one-shot fixture failure is already consumed: an actual next key accepts
  // the retained raw document at the preserved caret through the real worker.
  await page.keyboard.type('X');await settle(page);
  assert.equal((await a.snapshot()).projection.body,'RXQ remains.\n');
  assert.equal((await state(page)).drafts.intents.length,0);
 } finally { await page.close(); }
});

test('accepted native block Undo Redo maps kept draft anchors exactly once', {timeout:60000},async()=>{
 const {page,a}=await setup('A\n\nX\n\nC\n');
 try{
  await page.evaluate(async()=>{await editor.blocks.setView('blocks');await editor.blocks.editBlock(1);});
  await page.locator('.te-block-editor').fill('X draft');await page.keyboard.press('Escape');
  await page.waitForFunction(()=>!editor.blocks.editing&&editor.blocks.keptDrafts().length===1);
  await page.evaluate(async()=>{await editor.blocks.moveBlock(1,'down');await editor.blocks.moveBlock(2,'up');});
  assert.equal(await page.evaluate(()=>editor.blocks.keptDrafts()[0].anchor),3);
  await page.evaluate(()=>editor.blocks.undo());
  assert.equal((await a.snapshot()).projection.body,'A\n\nC\n\nX\n');
  assert.equal(await page.evaluate(()=>editor.blocks.keptDrafts()[0].anchor),6);
  await page.evaluate(()=>editor.blocks.redo());assert.equal(await page.evaluate(()=>editor.blocks.keptDrafts()[0].anchor),3);
  await page.evaluate(()=>editor.blocks.undo());
  assert.equal(await page.evaluate(()=>editor.blocks.applyDraft(editor.blocks.keptDrafts()[0].id)),'replace');
  await settle(page);assert.equal((await a.snapshot()).projection.body,'A\n\nC\n\nX draft\n');
  assert.equal((await state(page)).drafts.intents.length,0);
 }finally{await page.close();}
});
