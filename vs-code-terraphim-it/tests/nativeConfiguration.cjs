/* Real VS Code settings and the packaged @tomic Store. No HTTP/API substitutes. */
const vscode = require('vscode');
const assert = require('node:assert/strict');
const fs = require('node:fs/promises');
const path = require('node:path');
const { createRequire } = require('node:module');

exports.checkConfiguration = async (extension, output) => {
  const config = () => vscode.workspace.getConfiguration('terraphimIt');
  const keys = ['agent', 'atomicServerUrl', 'serverUrl', 'selectedRole', 'enableLocalServer'];
  const previous = Object.fromEntries(keys.map(key => [key, config().inspect(key).globalValue]));
  const { Agent } = createRequire(path.join(extension.extensionPath, 'out/extension.js'))('@tomic/lib');
  const stores = require(path.join(extension.extensionPath, 'out/helpers/getStore.js'));
  const { serverConnection } = require(path.join(extension.extensionPath, 'out/helpers/serverConnection.js'));
  const report = { hostVersion: vscode.version, extensionVersion: extension.packageJSON.version,
    scope: 'Actual native settings, packaged Store identity and routing refusal; no authenticated service or successful remote retrieval claimed', checks: [] };
  const update = async (key, value) => {
    if (config().inspect(key).globalValue === value) return;
    const observed = new Promise((resolve, reject) => {
      const timer = setTimeout(() => {
        listener.dispose();
        reject(new Error(`Native configuration event timed out: ${key}`));
      }, 5000);
      const listener = vscode.workspace.onDidChangeConfiguration(event => {
        if (event.affectsConfiguration(`terraphimIt.${key}`)) {
          clearTimeout(timer);
          listener.dispose();
          resolve();
        }
      });
    });
    await config().update(key, value, vscode.ConfigurationTarget.Global);
    await observed;
  };
  try {
    // Deliberately inert, synthetic credentials. Never sent to a service.
    const agent = new Agent(Buffer.alloc(32).toString('base64'), 'https://example.invalid/configuration-agent');
    await update('agent', agent.buildSecret());
    await update('atomicServerUrl', 'https://example.invalid/atomic-before');
    assert.equal(await vscode.commands.executeCommand('extension.terraphimAIAutocomplete'), true);
    const initial = stores.getStore();
    assert.equal(initial.getAgent().subject, agent.subject);
    assert.equal(initial.getServerUrl(), 'https://example.invalid/atomic-before');
    assert.equal(stores.getStore(), initial);
    report.checks.push('first autocomplete activation reads the configured agent; unchanged settings reuse the actual Store');

    await update('atomicServerUrl', 'https://example.invalid/atomic-after');
    const changed = stores.getStore();
    assert.notEqual(changed, initial);
    assert.equal(changed.getServerUrl(), 'https://example.invalid/atomic-after');
    assert.equal(changed.getAgent().subject, agent.subject);
    await update('agent', '');
    const anonymous = stores.getStore();
    assert.notEqual(anonymous, changed);
    assert.equal(anonymous.getAgent(), undefined);
    await update('agent', 'invalid-agent');
    assert.equal(await vscode.commands.executeCommand('extension.terraphimAIAutocomplete'), false);
    assert.throws(() => stores.getStore());
    await update('agent', '');
    report.checks.push('endpoint and credential changes replace the actual Store; clearing credentials is anonymous; malformed credentials reject activation');

    await update('serverUrl', 'http://127.0.0.1:0');
    await update('enableLocalServer', false);
    assert.throws(() => serverConnection.getServerUrl(), /Local Terraphim server is disabled/);
    const disabled = await serverConnection.healthCheck();
    assert.equal(disabled.healthy, false);
    assert.match(disabled.message, /Local Terraphim server is disabled/);
    assert.deepEqual(await serverConnection.searchDocuments('Risk'), {status:'error', results:[]});
    assert.deepEqual(await serverConnection.getAutocompleteSuggestions('Risk'), {status:'error', suggestions:[]});
    assert.equal(await vscode.commands.executeCommand('extension.terraphimServerAutocomplete'), false);
    for (const url of ['http://localhost:8000', 'http://127.1:8000', 'http://[::1]:8000', 'http://[::ffff:127.0.0.1]:8000', 'http://0.0.0.0:8000']) {
      await update('serverUrl', url);
      assert.throws(() => serverConnection.getServerUrl(), /Local Terraphim server is disabled/);
    }
    await update('serverUrl', 'https://example.invalid/terraphim/');
    assert.equal(serverConnection.getServerUrl(), 'https://example.invalid/terraphim');
    report.checks.push('disabled local mode refuses loopback in every request path and selects the explicit remote URL without a public fallback');

    const revision = serverConnection.getConfigurationRevision();
    await update('selectedRole', 'Configuration acceptance role');
    assert.equal(serverConnection.getSelectedRole(), 'Configuration acceptance role');
    assert.ok(serverConnection.getConfigurationRevision() > revision);
    await update('serverUrl', 'http://127.0.0.1:0');
    await update('enableLocalServer', true);
    assert.equal(serverConnection.getServerUrl(), 'http://127.0.0.1:0');
    const unavailable = await serverConnection.healthCheck();
    assert.equal(unavailable.healthy, false);
    assert.doesNotMatch(unavailable.message, /Local Terraphim server is disabled/);
    report.checks.push('enabled local mode attempts the real unavailable endpoint; role changes advance the completion-cache revision');
    report.health = {disabled, unavailable};
    report.result = 'PASS';
  } catch (error) {
    report.result = 'FAIL'; report.error = String(error.stack || error); throw error;
  } finally {
    for (const key of keys) {
      if (config().inspect(key).globalValue !== previous[key]) await update(key, previous[key]);
    }
    stores.resetStore();
    report.settingsRestored = keys.every(key => config().inspect(key).globalValue === previous[key]);
    await fs.writeFile(path.join(output, 'configuration.json'), JSON.stringify(report, null, 2) + '\n');
    assert.equal(report.settingsRestored, true);
  }
};
