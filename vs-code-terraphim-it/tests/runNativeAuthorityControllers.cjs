// Admission for the mechanically pinned upstream suite. The existing CI extraction
// receipt binds every runtime input to the downloaded VSIX before a browser starts.
const fs=require('node:fs');
const path=require('node:path');
const assert=require('node:assert/strict');
const {createHash}=require('node:crypto');
const {spawnSync}=require('node:child_process');
const root=path.resolve(__dirname,'..');
const runtime=path.resolve(process.env.TERRAPHIM_CONTROLLER_RUNTIME||root);
const results=path.resolve(process.env.TERRAPHIM_CONTROLLER_RESULTS||path.join(root,'native-results'));
const suite=path.join(__dirname,'nativeAuthorityControllers.snapshot.cjs');
const provenance=JSON.parse(fs.readFileSync(path.join(__dirname,'nativeAuthorityControllers.provenance.json'),'utf8'));
const hash=bytes=>createHash('sha256').update(bytes).digest('hex');
const bytes=fs.readFileSync(suite);
assert.equal(hash(bytes),provenance.sha256,'Pinned controller suite changed; update its source provenance explicitly');
assert.equal(bytes.toString().split('\ntest(').length-1,provenance.testCount,'Entire canonical suite must be admitted');
const fixture=path.resolve(__dirname,provenance.fixture.snapshotPath);
assert.ok(fixture.startsWith(__dirname+path.sep),'Fixture path must remain within test sources');
const fixtureBytes=fs.readFileSync(fixture);
assert.equal(hash(fixtureBytes),provenance.fixture.sha256,'Pinned actual public brief fixture changed');
assert.equal(fixtureBytes.length,provenance.fixture.bytes,'Actual fixture byte count changed');
const inputs=JSON.parse(fs.readFileSync(path.join(results,'history-component-inputs.json'),'utf8'));
const packagePath=path.resolve(process.env.TERRAPHIM_CONTROLLER_VSIX||path.join(root,'terraphim-it.vsix'));
assert.equal(hash(fs.readFileSync(packagePath)),inputs.packageSha256,'Runtime extraction receipt belongs to a different VSIX');
for(const [relative,digest] of Object.entries(inputs.runtimeInputs)){
 const file=path.resolve(runtime,relative);
 assert.ok(file.startsWith(runtime+path.sep)&&/^(media|out)\//.test(relative),'Invalid runtime input path');
 assert.equal(hash(fs.readFileSync(file)),digest,'Packaged runtime changed: '+relative);
}
for(const required of ['media/editor/terraphim-editor.min.js','media/editor/terraphim_editor.js','media/editor/terraphim_editor_bg.wasm','media/history-checkpoint.js','media/resource-authority-worker.cjs','out/resourceAuthority.js'])assert.ok(inputs.runtimeInputs[required],'Required packaged input absent: '+required);
const receipt={status:'PASS integrity admission only; browser result recorded separately',upstream:provenance,packageSha256:inputs.packageSha256,runtimeFiles:Object.keys(inputs.runtimeInputs).length};
fs.writeFileSync(path.join(results,'authority-controllers-provenance.json'),JSON.stringify(receipt,null,2)+'\n');
if(process.argv.includes('--check-only')){process.stdout.write(JSON.stringify(receipt)+'\n');process.exit(0);}
const env={...process.env,TERRAPHIM_AUTHORITY_EXTENSION_DIR:runtime,TERRAPHIM_AUTHORITY_ASSETS:path.join(runtime,'media','editor'),TERRAPHIM_TRIM_REGRESSION_FIXTURE:fixture,TERRAPHIM_TRIM_REGRESSION_OUTPUT:path.join(results,'actual-public-brief-controller'),NODE_PATH:[path.join(root,'node_modules'),process.env.NODE_PATH].filter(Boolean).join(path.delimiter)};
const result=spawnSync(process.execPath,['--test',suite],{env,stdio:'inherit',timeout:12*60*1000});
if(result.error)throw result.error;
process.exit(result.status??1);
