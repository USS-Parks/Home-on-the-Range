'use strict';

// Connection-only acceptance of the stock installed application. No model
// prompt, provider credential, source patch or call into private tool handlers.
const fs = require('node:fs');
const path = require('node:path');
const crypto = require('node:crypto');
const assert = require('node:assert/strict');
const { spawn } = require('node:child_process');

const ROOT = path.resolve(__dirname, '../..');
const PROFILE_ROOT = path.join(ROOT, 'work/hotr-client-profiles');
const TOOLS = ['hotr_create', 'hotr_get', 'hotr_health', 'hotr_revise', 'hotr_search'];
const ALLOW_HOOK = `if (!${JSON.stringify(TOOLS.map(name => `hotr__${name}`))}.includes(toolName)) throw new Error('HOTR acceptance permits only its five context tools');`;
const sha = value => crypto.createHash('sha256').update(value).digest('hex');
const newFile = (file, value) => {
  const fd = fs.openSync(file, 'wx');
  try { fs.writeFileSync(fd, value); fs.fsyncSync(fd); } finally { fs.closeSync(fd); }
};
function checkedExisting(file, parent) {
  // Rust's canonical Windows paths carry the extended-length prefix. Node's
  // legacy realpath walker mishandles that form; use the native Win32 resolver.
  file = file.replace(/^\\\\\?\\/, '');
  const resolved = fs.realpathSync.native(file).replace(/^\\\\\?\\/, '');
  assert.equal(path.relative(parent, resolved).startsWith('..'), false, 'path escapes approved root');
  assert.equal(path.isAbsolute(path.relative(parent, resolved)), false);
  for (let part = file; path.dirname(part) !== part; part = path.dirname(part)) {
    assert.equal(fs.lstatSync(part).isSymbolicLink(), false, 'reparse point refused');
  }
  return resolved;
}
function timeout(promise, ms = 30000) {
  let timer;
  return Promise.race([promise, new Promise((_, reject) => {
    timer = setTimeout(() => reject(new Error(`Timed out after ${ms} ms`)), ms);
  })]).finally(() => clearTimeout(timer));
}
function deferred() {
  let resolve, reject;
  const promise = new Promise((a, b) => { resolve = a; reject = b; });
  promise.catch(() => {});
  return { promise, resolve, reject };
}

async function launch(executable, profile, workspace, bootstrap, env) {
  const child = spawn(executable, ['--inspect-brk=0', '--remote-debugging-address=127.0.0.1', '--remote-debugging-port=0', `--user-data-dir=${profile}`], {
    env, cwd: workspace, windowsHide: true, stdio: ['ignore', 'pipe', 'pipe']
  });
  const node = deferred(), browser = deferred(), isolated = deferred(), exited = deferred();
  let out = '', err = '', socket, next = 0, exitCode, isolation;
  const pending = new Map(), scripts = new Map();
  const fail = error => { node.reject(error); browser.reject(error); isolated.reject(error); child.kill(); };
  child.on('error', fail);
  child.once('exit', code => {
    exitCode = code;
    exited.resolve(code);
    const error = new Error(`Owned application exited: ${code}`);
    node.reject(error); browser.reject(error); isolated.reject(error);
  });
  child.stdout.on('data', data => {
    out += data;
    if (Buffer.byteLength(out) > 4 * 1024 * 1024) fail(new Error('Application stdout ceiling'));
  });
  child.stderr.on('data', data => {
    err += data;
    if (Buffer.byteLength(err) > 4 * 1024 * 1024) return fail(new Error('Application stderr ceiling'));
    const n = err.match(/Debugger listening on (ws:\/\/127\.0\.0\.1:\d+\/[^\s]+)/);
    const b = err.match(/DevTools listening on (ws:\/\/127\.0\.0\.1:\d+\/[^\s]+)/);
    if (n) node.resolve(n[1]);
    if (b) browser.resolve(b[1]);
  });
  const send = (method, params = {}) => timeout(new Promise((resolve, reject) => {
    const id = ++next;
    pending.set(id, { resolve, reject });
    socket.send(JSON.stringify({ id, method, params }));
  }));
  try {
    socket = new WebSocket(await timeout(node.promise));
    await timeout(new Promise((resolve, reject) => {
      socket.addEventListener('open', resolve, { once: true });
      socket.addEventListener('error', reject, { once: true });
    }));
    socket.addEventListener('close', () => {
      for (const request of pending.values()) request.reject(new Error('Inspector closed'));
      pending.clear();
    });
    socket.addEventListener('message', event => {
      const message = JSON.parse(event.data);
      if (message.id) {
        const request = pending.get(message.id);
        pending.delete(message.id);
        if (message.error) request?.reject(new Error(message.error.message));
        else request?.resolve(message.result);
      } else if (message.method === 'Debugger.scriptParsed') {
        scripts.set(message.params.scriptId, message.params.url);
      } else if (message.method === 'Debugger.paused') {
        void (async () => {
          const frame = message.params.callFrames[0];
          const url = frame.url || scripts.get(frame.location.scriptId) || '';
          // Never resume an unidentified pause before profile isolation.
          assert.match(url, /app\.asar[\\/]out[\\/]main[\\/]index\.js$/);
          // V8 skips the initial strict-mode directive and pauses on the first
          // executable line. Verify the complete paused script, not just its URL.
          const parsed = await send('Debugger.getScriptSource', { scriptId: frame.location.scriptId });
          assert.equal(sha(parsed.scriptSource), 'd9d08465d0216032058109243dce5d30bb069be23437ca5aecd3594aef4857be');
          assert.equal(frame.location.lineNumber, 1, 'entry already advanced');
          assert.equal(parsed.scriptSource.split('\n')[0].trim(), '"use strict";');
          assert.equal(isolation, undefined, 'unexpected subsequent pause');
          const result = await send('Debugger.evaluateOnCallFrame', {
            callFrameId: frame.callFrameId,
            expression: `require(${JSON.stringify(bootstrap)}); global.__hotrIsolation()`, returnByValue: true
          });
          assert.equal(result.exceptionDetails, undefined, 'bootstrap evaluation failed');
          isolation = result.result.value;
          assert.equal(isolation.userData, profile);
          assert.equal(isolation.packaged, true);
          await send('Debugger.setSkipAllPauses', { skip: true });
          await send('Debugger.resume');
          isolated.resolve(isolation);
        })().catch(fail);
      }
    });
    await send('Runtime.enable');
    await send('Debugger.enable');
    await send('Debugger.setBreakpointByUrl', { lineNumber: 0, urlRegex: 'app\\.asar[\\\\/]out[\\\\/]main[\\\\/]index\\.js$' });
    await send('Runtime.runIfWaitingForDebugger');
    await timeout(isolated.promise);
    return {
      isolation, child, browser: () => timeout(browser.promise),
      evaluate: async expression => {
        const response = await send('Runtime.evaluate', { expression, returnByValue: true });
        assert.equal(response.exceptionDetails, undefined);
        return response.result.value;
      },
      close: async () => {
        if (exitCode === undefined) {
          await send('Runtime.evaluate', { expression: 'global.__hotrQuit()' }).catch(() => {});
          socket.close();
          await timeout(exited.promise, 10000).catch(async () => { child.kill(); await timeout(exited.promise, 5000); });
        } else socket.close();
        return { exit_code: exitCode, stdout: out, stderr: err };
      }
    };
  } catch (error) {
    socket?.close();
    if (exitCode === undefined) child.kill();
    await timeout(exited.promise, 5000).catch(() => {});
    throw Object.assign(new Error(error.message), { application: { stdout: out, stderr: err } });
  }
}

async function main(request, exercise) {
  assert.equal(request.mode, 'preflight', 'This probe cannot send model prompts');
  const profile = checkedExisting(request.profile, fs.realpathSync(PROFILE_ROOT));
  const workspace = checkedExisting(request.workspace, fs.realpathSync(PROFILE_ROOT));
  assert.equal(fs.readFileSync(path.join(profile, 'SYNTHETIC-ONLY'), 'utf8').startsWith('HOTR-12-LAMPREY'), true);
  const credential = checkedExisting(request.credential, path.join(ROOT, 'work/hotr-tests'));
  const hotr = checkedExisting(request.hotr, path.join(ROOT, 'work/hotr-build'));
  const executable = fs.realpathSync(request.executable);
  const resources = path.join(path.dirname(executable), 'resources');
  const asarFile = path.join(resources, 'app.asar');
  const asar = require(path.join(request.lamprey_source, 'node_modules/@electron/asar'));
  const mainBytes = asar.extractFile(asarFile, path.normalize('out/main/index.js'));
  const packageInfo = JSON.parse(asar.extractFile(asarFile, 'package.json'));
  assert.equal(packageInfo.version, '0.32.0');
  assert.equal(packageInfo.main, './out/main/index.js');
  assert.equal(sha(mainBytes), 'd9d08465d0216032058109243dce5d30bb069be23437ca5aecd3594aef4857be');
  for (const name of ['session-data', 'crash-dumps', 'logs', 'temp', 'roaming', 'local']) fs.mkdirSync(path.join(profile, name));
  const plugins = {};
  for (const entry of fs.readdirSync(path.join(resources, 'plugins'), { withFileTypes: true })) {
    if (entry.isDirectory()) {
      const manifest = JSON.parse(fs.readFileSync(path.join(resources, 'plugins', entry.name, 'plugin.json'), 'utf8'));
      plugins[manifest.id] = false;
    }
  }
  newFile(path.join(profile, 'plugins.json'), JSON.stringify(plugins));
  newFile(path.join(profile, 'active-workspace.txt'), workspace);
  newFile(path.join(profile, 'settings.json'), JSON.stringify({ autoCheckUpdates: false, aiGeneratedTitles: false, minimizeToTray: false,
    toolSurface: 'full', agenticCodingMode: false, loopsEnabled: false, orchestrationEnabled: false, mcpCallTimeoutMs: 10000 }));
  newFile(path.join(profile, 'mcp-servers.json'), JSON.stringify([
    { id: 'hotr', name: 'Home on the Range', transport: 'stdio', command: hotr, args: ['mcp', '--credential', credential], auth: 'none', enabled: true },
    { id: 'node-repl', name: 'Node REPL', transport: 'stdio', command: process.execPath, args: [], auth: 'none', enabled: false }
  ]));
  const env = {};
  for (const name of ['SystemRoot', 'SYSTEMROOT', 'WINDIR', 'ComSpec', 'COMSPEC', 'PATH', 'PATHEXT', 'ProgramFiles', 'ProgramFiles(x86)', 'PROGRAMDATA', 'NUMBER_OF_PROCESSORS', 'PROCESSOR_ARCHITECTURE']) {
    if (process.env[name] !== undefined) env[name] = process.env[name];
  }
  Object.assign(env, { USERPROFILE: workspace, HOME: workspace, APPDATA: path.join(profile, 'roaming'), LOCALAPPDATA: path.join(profile, 'local'),
    TEMP: path.join(profile, 'temp'), TMP: path.join(profile, 'temp'), HOTR_LAMPREY_PROFILE: profile });
  const launched = await launch(executable, profile, workspace, path.join(__dirname, 'lamprey_bootstrap.cjs'), env);
  let browser, report, failure;
  const deadline = setTimeout(() => launched.child.kill(), 90000);
  try {
    const { chromium } = require(path.join(request.lamprey_source, 'node_modules/playwright'));
    browser = await chromium.connectOverCDP(await launched.browser());
    let page;
    const until = Date.now() + 30000;
    while (Date.now() < until && !page) {
      for (const candidate of browser.contexts().flatMap(context => context.pages())) {
        if (await candidate.evaluate(() => Boolean(window.api?.mcp && window.api?.hooks)).catch(() => false)) page = candidate;
      }
      if (!page) await new Promise(resolve => setTimeout(resolve, 100));
    }
    assert.ok(page, 'Installed renderer preload unavailable');
    const data = await page.evaluate(async ({ hookCode, names }) => {
      const api = window.api;
      const unwrap = response => { if (!response.success) throw new Error(response.error); return response.data; };
      const hook = unwrap(await api.hooks.create({ event: 'preToolUse', label: 'HOTR synthetic tools only', command: hookCode, language: 'js', timeoutMs: 1000 }));
      const hookPositive = await api.hooks.test({ code: hookCode, event: 'preToolUse', context: { toolName: 'hotr__hotr_get', args: {} } });
      const hookNegative = await api.hooks.test({ code: hookCode, event: 'preToolUse', context: { toolName: 'read_file', args: {} } });
      const deadline = Date.now() + 20000;
      let tools;
      while (Date.now() < deadline) {
        tools = unwrap(await api.tools.resolve(names));
        if (tools.length === names.length) break;
        await new Promise(resolve => setTimeout(resolve, 100));
      }
      return { hook, hookPositive, hookNegative, hooks: await api.hooks.list(), tools,
        servers: await api.mcp.list(), status: await api.mcp.getStatus('hotr'), dataDir: await api.app.getDataDir(),
        persistence: await api.persistence.runIntegrityCheck() };
    }, { hookCode: ALLOW_HOOK, names: TOOLS.map(name => `hotr__${name}`) });
    assert.deepEqual(data.tools.map(tool => tool.id).sort(), TOOLS.map(name => `hotr__${name}`).sort());
    // Lamprey's provider normalizer rejects these even after MCP discovery
    // succeeds. Fail before any model prompt if that incompatibility returns.
    const portable = schema => {
      if (!schema || typeof schema !== 'object') return;
      if (Array.isArray(schema)) return schema.forEach(portable);
      for (const [name, value] of Object.entries(schema)) {
        assert.ok(!['$ref', 'oneOf', 'anyOf', 'allOf'].includes(name), `provider-incompatible schema: ${name}`);
        portable(value);
      }
    };
    data.tools.forEach(tool => portable(tool.inputSchema));
    assert.equal(data.hook.enabled, true);
    assert.equal(data.hookPositive.success, true);
    assert.equal(data.hookPositive.data.thrown, undefined);
    assert.match(data.hookNegative.data.thrown, /only its five context tools/);
    assert.equal(data.status.success, true);
    assert.equal(data.status.data.status, 'connected');
    assert.equal(data.persistence.data.ok, true);
    assert.equal(data.dataDir.data.userData, profile);
    assert.deepEqual(data.servers.data.filter(server => server.enabled).map(server => server.id), ['hotr']);
    const isolation = await launched.evaluate('global.__hotrIsolation()');
    assert.equal(isolation.windows.every(window => !window.visible && !window.focused), true);
    assert.equal(isolation.userData, profile);
    report = { result: 'PREFLIGHT_PASS', model_prompts: 0, version: packageInfo.version,
      executable_sha256: sha(fs.readFileSync(executable)), installed_main_sha256: sha(mainBytes),
      isolation, data, evidence_boundary: 'Installed application startup, renderer IPC, five MCP descriptors and native hook test. No model dispatch or shared-record acceptance yet.' };
    if (exercise) {
      clearTimeout(deadline);
      report.exercise = await exercise(page, profile, report);
    }
  } catch (error) {
    failure = error;
  } finally {
    clearTimeout(deadline);
    const closed = await launched.close();
    if (browser) await browser.close().catch(() => {});
    if (report) report.application = closed;
    if (failure) failure.application = closed;
  }
  if (failure) throw failure;
  return report;
}

module.exports = { main, TOOLS, ALLOW_HOOK };
if (require.main === module) {
  let input = '';
  process.stdin.setEncoding('utf8');
  process.stdin.on('data', data => { input += data; if (input.length > 65536) process.exit(2); });
  process.stdin.on('end', () => {
    void main(JSON.parse(input)).then(result => process.stdout.write(JSON.stringify(result) + '\n')).catch(error => {
      process.stdout.write(JSON.stringify({ result: 'FAIL', error: error.stack, application: error.application }) + '\n');
      process.exitCode = 1;
    });
  });
}
