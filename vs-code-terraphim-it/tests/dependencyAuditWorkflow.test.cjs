// Parse the actual CI documents; no synthetic workflow or npm results.
const {test}=require('node:test');
const assert=require('node:assert/strict');
const fs=require('node:fs');
const path=require('node:path');
const yaml=require('js-yaml');
const root=path.resolve(__dirname,'../..');
const read=name=>yaml.load(fs.readFileSync(path.join(root,'.github/workflows',name),'utf8'));
test('locked dependency audit fails closed before editor imports and packaging',()=>{
 const job=read('vscode-extension.yml').jobs.package;
 const steps=job.steps;
 const install=steps.findIndex(s=>s.run==='npm ci --ignore-scripts --no-audit --no-fund');
 const audit=steps.findIndex(s=>s.name==='Audit locked extension dependencies');
 const fetch=steps.findIndex(s=>s.run==='npm run editor:fetch');
 const pack=steps.findIndex(s=>s.run==='npm run package:vsix');
 assert.ok(install>=0&&audit>install&&fetch>audit&&pack>fetch);
 assert.equal(steps[audit].if,undefined);
 assert.equal(steps[audit]['continue-on-error'],undefined);
 assert.equal(job['continue-on-error'],undefined);
 assert.equal(steps[audit].run.trim(),'mkdir -p native-results\nnpm audit --json --audit-level=high > native-results/dependency-audit.json');
 const upload=steps.find(s=>s.name==='Retain dependency audit even on refusal');
 assert.equal(upload.if,'always()');
 assert.equal(upload.uses,'actions/upload-artifact@ea165f8d65b6e75b540449e92b4886f43607fa02');
 assert.equal(upload.with.path,'vs-code-terraphim-it/native-results/dependency-audit.json');
 assert.equal(upload.with.name,'vscode-dependency-audit-${{ github.sha }}');
});
test('release publication depends on the same full audited qualification workflow',()=>{
 const jobs=read('vscode-release.yml').jobs;
 assert.equal(jobs.qualify.uses,'./.github/workflows/vscode-extension.yml');
 const needs=Array.isArray(jobs.release.needs)?jobs.release.needs:[jobs.release.needs];
 assert.ok(needs.includes('qualify'));
});
