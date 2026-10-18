/* Retained-state regression starts the exact extracted VSIX in a disposable VS Code profile. */
const fs = require('node:fs/promises');
const os = require('node:os');
const path = require('node:path');
const { execFileSync } = require('node:child_process');
const { runTests } = require('@vscode/test-electron');

(async () => {
  const root = path.resolve(__dirname, '..');
  const run = await fs.mkdtemp(path.join(os.tmpdir(), 'tv-'));
  const extracted = path.join(run, 'package');
  const receipt = path.join(run, 'outputs');
  let failure;
  let packageSha256 = null;
  const snapshot = path.join(run, 'terraphim-it.vsix');
  try {
    const packageBytes = await fs.readFile(path.join(root, 'terraphim-it.vsix'));
    packageSha256 = require('node:crypto').createHash('sha256').update(packageBytes).digest('hex');
    await fs.writeFile(snapshot, packageBytes, {flag: 'wx'});
    // Verify both manifests and safe extraction paths before executing the package.
    execFileSync(process.env.PYTHON || 'python3', [path.join(root, 'scripts/verify_vsix.py'),
      snapshot, '--output', path.join(run, 'package-manifest.json')]);
    execFileSync(process.env.PYTHON || 'python3', ['-m', 'zipfile', '-e',
      snapshot, extracted]);
    await runTests({
    version: '1.126.0',
    vscodeExecutablePath: process.env.TERRAPHIM_VSCODE_EXECUTABLE,
    extensionDevelopmentPath: path.join(extracted, 'extension'),
    extensionTestsPath: path.join(__dirname, 'nativeRetainedRecovery.cjs'),
    extensionTestsEnv: {TERRAPHIM_RETAINED_OUTPUT: receipt},
    launchArgs: ['--user-data-dir', path.join(run, 'profile'), '--extensions-dir', path.join(run, 'extensions'),
      '--skip-welcome', '--skip-release-notes', '--disable-workspace-trust', '--no-sandbox'],
    });
  } catch (error) {
    failure = error;
  } finally {
    await fs.mkdir(receipt, {recursive: true});
    await fs.writeFile(path.join(receipt, 'launcher.json'), JSON.stringify({
      kind: 'retained-recovery',
      platform: process.platform,
      status: failure ? 'FAIL' : 'PASS',
      message: failure ? String(failure.message || failure) : 'Packaged retained-state recovery completed',
      package: 'terraphim-it.vsix',
      packageSha256,
    }, null, 2) + '\n');
    const output = path.join(root, 'native-results', path.basename(run));
    try {
      await fs.cp(receipt, output, {recursive: true, errorOnExist: true, force: false});
      console.log('Packaged retained-state recovery outputs:', output);
    } catch (copyError) {
      console.error('Evidence retained at:', receipt);
      if (!failure) { failure = copyError; }
    }
  }
  if (failure) { throw failure; }
})().catch(error => {console.error(error); process.exitCode = 1;});
