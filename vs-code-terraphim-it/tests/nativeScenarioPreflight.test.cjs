'use strict';
const {test}=require('node:test'),assert=require('node:assert/strict'),fs=require('node:fs'),path=require('node:path'),os=require('node:os'),{spawnSync}=require('node:child_process');
const {nativeScenarioPreflight}=require('./nativeScenarioPreflight.cjs');
test('actual preflight refuses missing/extra Overflow groups and incomplete fixtures',()=>{
 for(const groups of [undefined,'','word-indicators','overflow-typing-isolated,word-indicators'])assert.throws(()=>nativeScenarioPreflight({TERRAPHIM_MAC_OVERFLOW_ONLY:'1',TERRAPHIM_SCENARIO_GROUPS:groups}),/preflight refused/);
 assert.throws(()=>nativeScenarioPreflight({TERRAPHIM_MAC_ALL41:'1'}),/explicit/);
 assert.throws(()=>nativeScenarioPreflight({TERRAPHIM_SCENARIO_GROUPS:'word-indicators'}),/complete all41 fixture/);
 assert.throws(()=>nativeScenarioPreflight({TERRAPHIM_MAC_ALL41:'1',TERRAPHIM_MATRIX_TEMPLATE:__filename,TERRAPHIM_SCENARIO_GROUPS:'invented'}),/unknown/);
});
test('exact controlled Overflow group and explicit complete-fixture groups accepted',()=>{
 assert.deepEqual(nativeScenarioPreflight({TERRAPHIM_MAC_OVERFLOW_ONLY:'1',TERRAPHIM_MATRIX_TEMPLATE:__filename,TERRAPHIM_SCENARIO_GROUPS:'overflow-typing-isolated'}),{mode:'overflow-only',groups:['overflow-typing-isolated']});
 assert.deepEqual(nativeScenarioPreflight({TERRAPHIM_MAC_ALL41:'1',TERRAPHIM_MATRIX_TEMPLATE:__filename,TERRAPHIM_SCENARIO_GROUPS:'context-hover-style,external-context-reanchor'}),{mode:'finite-all41',groups:['context-hover-style','external-context-reanchor']});
 assert.deepEqual(nativeScenarioPreflight({}),{mode:'native-core',groups:[]});
});
test('actual Mac harness refuses omitted Overflow group before creating output or launching application',{skip:process.platform!=='darwin'},()=>{
 const parent=fs.mkdtempSync(path.join(os.tmpdir(),'native-preflight-negative-')),out=path.join(parent,'must-not-exist');const env={...process.env,TERRAPHIM_MAC_OVERFLOW_ONLY:'1',TERRAPHIM_HISTORY_OUTPUT:out};delete env.TERRAPHIM_SCENARIO_GROUPS;delete env.TERRAPHIM_MAC_ALL41;
 const result=spawnSync(process.execPath,[path.join(__dirname,'nativeAuthorityMac.cjs')],{env,encoding:'utf8',timeout:10000});assert.equal(result.status,1);assert.match(result.stderr,/Native preflight refused: Overflow-only mode/);assert.equal(fs.existsSync(out),false);
});
