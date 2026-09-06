/* HOTR-18 installed-Chrome gate.  Reads one JSON configuration from stdin and
 * writes only a metadata report plus screenshots containing synthetic data. */
'use strict';
const fs = require('fs');
const http = require('http');
const path = require('path');
const { spawnSync } = require('child_process');
const ephemeralCredentials = new Set();

const repo = path.resolve(__dirname, '..', '..');
const runs = path.join(repo, 'work', 'hotr-tests') + path.sep;
const playwrightModule = process.env.HOTR_PLAYWRIGHT_MODULE || 'C:\\Users\\17076\\.cache\\codex-runtimes\\codex-primary-runtime\\dependencies\\node\\node_modules\\playwright';
const chrome = process.env.HOTR_CHROME_EXE || 'C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe';

function fail(message) { throw new Error(message); }
function localPath(value) {
  if (typeof value !== 'string') fail('local path required');
  return path.resolve(value.startsWith("\\\\?\\") ? value.slice(4) : value);
}
function insideRun(run, candidate) {
  const resolved = localPath(candidate);
  return resolved === run || resolved.startsWith(run + path.sep);
}
function newFile(file, bytes) {
  const descriptor = fs.openSync(file, 'wx', 0o600);
  try { fs.writeFileSync(descriptor, bytes); fs.fsyncSync(descriptor); } finally { fs.closeSync(descriptor); }
}
function noReparse(pathname) {
  const absolute = localPath(pathname);
  let current = path.parse(absolute).root;
  for (const part of absolute.slice(current.length).split(path.sep).filter(Boolean)) {
    current = path.join(current, part);
    const entry = fs.lstatSync(current);
    if (entry.isSymbolicLink()) fail('reparse path rejected');
  }
}
function readConfig() {
  const raw = fs.readFileSync(0, 'utf8');
  const config = JSON.parse(raw);
  if (!config || typeof config !== 'object') fail('config object required');
  for (const name of ['run', 'vault', 'binary', 'port']) if (!(name in config)) fail(`missing ${name}`);
  const run = localPath(config.run);
  if (!run.startsWith(runs) || !fs.existsSync(path.join(run, '.fixture'))) fail('new marked synthetic run required');
  noReparse(run);
  if (fs.readFileSync(path.join(run, '.fixture'), 'utf8') !== 'HOTR-18 synthetic viewer fixture\n') fail('fixture marker rejected');
  if (localPath(config.vault) !== path.join(run, 'vault')) fail('vault outside fixture');
  noReparse(config.vault);
  const binary = localPath(config.binary);
  const expectedBinary = path.join(repo, 'work', 'hotr-build', 'target', 'release', 'hotr.exe');
  if (binary !== expectedBinary || !fs.existsSync(binary)) fail('unexpected binary');
  noReparse(binary);
  if (!Number.isInteger(config.port) || config.port < 1 || config.port > 65535) fail('bad port');
  return { ...config, vault: localPath(config.vault), binary, run };
}
function requestStatus(origin, token) {
  return new Promise((resolve, reject) => {
    const request = http.request(`${origin}/viewer/api/read`, {
      method: 'POST', headers: { Origin: origin, 'Sec-Fetch-Site': 'same-origin', 'X-HOTR-Viewer': '1', 'Content-Type': 'application/json', Authorization: `Bearer ${token}`, 'Content-Length': '20' }
    }, response => { response.resume(); response.once('end', () => resolve(response.statusCode)); });
    request.setTimeout(2_000, () => request.destroy(new Error('viewer read timeout')));
    request.once('error', reject); request.end('{"operation":"ping"}');
  });
}
function lockVault(config) {
  const result = spawnSync(config.binary, ['lock', config.vault], { encoding: 'utf8', windowsHide: true, timeout: 15_000 });
  if (result.error || result.status !== 0) fail('final fixture lock failed');
}
async function waitText(page, selector, needle) {
  await page.waitForFunction(({ selector, needle }) => {
    const node = document.querySelector(selector);
    return Boolean(node && node.textContent && node.textContent.includes(needle));
  }, { selector, needle }, { timeout: 10_000 });
}
function runSession(config, seconds) {
  const result = spawnSync(config.binary, ['viewer-session', config.vault, '--seconds', String(seconds)], { encoding: 'utf8', windowsHide: true, timeout: 15_000 });
  if (result.error || result.status !== 0) fail('viewer-session CLI failed');
  let reply; try { reply = JSON.parse(result.stdout); } catch { fail('viewer-session returned non-JSON'); }
  const data = reply && reply.data;
  if (!data || !/^[a-f0-9]{64}$/i.test(data.code) || data.session_seconds !== seconds) fail('viewer-session response invalid');
  ephemeralCredentials.add(data.code);
  return data.code;
}
function startForeignPage() {
  const server = http.createServer((request, response) => {
    response.writeHead(200, { 'content-type': 'text/html', 'cache-control': 'no-store' });
    response.end('<!doctype html><button id="go">go</button><pre id="result"></pre><script>go.onclick=async()=>{try{await fetch(window.target,{method:"POST",headers:{"Authorization":"Bearer "+window.token,"Content-Type":"application/json","X-HOTR-Viewer":"1"},body:"{\\"operation\\":\\"ping\\"}"});result.textContent="unexpected"}catch(e){result.textContent="blocked"}}</script>');
  });
  return new Promise(resolve => server.listen(0, '127.0.0.1', () => resolve(server)));
}
function scanEphemeralCredentials(run) {
  const patterns = [...ephemeralCredentials].flatMap(value => [Buffer.from(value), Buffer.from(value, 'utf16le')]);
  if (patterns.length < 4) fail('viewer credential scan coverage missing');
  const overlap = Math.max(...patterns.map(value => value.length)) - 1;
  let files = 0; let total = 0;
  function visit(directory) {
    for (const name of fs.readdirSync(directory)) {
      const file = path.join(directory, name);
      if (!insideRun(run, file)) fail('credential scan path escaped fixture');
      const stat = fs.lstatSync(file);
      if (stat.isSymbolicLink()) fail('credential scan reparse rejected');
      if (stat.isDirectory()) { visit(file); continue; }
      if (!stat.isFile()) continue;
      total += stat.size;
      if (total > 1024 * 1024 * 1024 || ++files > 20000) fail('credential scan exceeds fixture budget');
      const fd = fs.openSync(file, 'r'); let previous = Buffer.alloc(0);
      try {
        const chunk = Buffer.alloc(1024 * 1024); let size;
        while ((size = fs.readSync(fd, chunk)) > 0) {
          const window = Buffer.concat([previous, chunk.subarray(0, size)]);
          if (patterns.some(pattern => window.includes(pattern))) fail(`ephemeral viewer credential persisted in fixture file ${path.relative(run, file)}`);
          previous = Buffer.from(window.subarray(Math.max(0, window.length - overlap)));
        }
      } finally { fs.closeSync(fd); }
    }
  }
  visit(run);
  return { files, bytes: total, credentials: ephemeralCredentials.size, utf8_utf16le_absent: true };
}
async function main() {
  const config = readConfig();
  const { chromium } = require(playwrightModule);
  if (!fs.existsSync(chrome)) fail('installed Chrome executable unavailable');
  const profile = path.join(config.run, 'viewer-browser-profile');
  if (!insideRun(config.run, profile) || fs.existsSync(profile)) fail('fresh isolated browser profile required');
  fs.mkdirSync(profile, { recursive: true, mode: 0o700 });
  noReparse(profile);
  newFile(path.join(profile, 'SYNTHETIC-ONLY'), 'HOTR-18 isolated Chrome profile; no personal data\n');
  const context = await chromium.launchPersistentContext(profile, { executablePath: chrome, headless: true, ignoreHTTPSErrors: false, args: ['--no-first-run', '--no-default-browser-check'] });
  const origin = `http://127.0.0.1:${config.port}`;
  const evidence = { prompt: 'HOTR-18', result: 'PASS', browser: context.browser().version(), headless: true, installed_chrome_process: true, synthetic_profile: true, assertions: [] };
  const assert = (condition, name) => { if (!condition) fail(name); evidence.assertions.push(name); };
  const screenshot = async (page, name) => { const file = path.join(config.run, name); if (!insideRun(config.run, file) || fs.existsSync(file)) fail('screenshot target exists'); await page.screenshot({ path: file, fullPage: true }); };
  try {
    const page = await context.newPage();
    let actualToken = null;
    const pageHttpRequests = [];
    page.on('request', request => {
      if (/^https?:/i.test(request.url())) pageHttpRequests.push(request.url());
      if (request.url().endsWith('/viewer/api/read')) {
        const authorization = request.headers().authorization;
        if (authorization && authorization.startsWith('Bearer ')) { actualToken = authorization.slice(7); ephemeralCredentials.add(actualToken); }
      }
    });
    await page.goto(`${origin}/viewer/`, { waitUntil: 'networkidle' });
    assert(await page.locator('#login-form').isVisible(), 'login visible');
    assert((await page.title()) === 'Home on the Range — Owner viewer', 'static viewer loaded');
    const staticHeaders = await page.evaluate(async () => { const r = await fetch('/viewer/'); return { cache: r.headers.get('cache-control'), type: r.headers.get('content-type') }; });
    assert(staticHeaders.cache === 'no-store' && /text\/html/.test(staticHeaders.type || ''), 'static no-store');
    const code = runSession(config, 600);
    await page.locator('#login-code').focus();
    await page.keyboard.type(code);
    const initialNamespaces = page.waitForResponse(response => response.url().endsWith('/viewer/api/read') && response.status() === 200);
    await page.keyboard.press('Enter');
    await page.locator('#viewer-view').waitFor({ state: 'visible' });
    await initialNamespaces;
    assert(await page.locator('#viewer-view').isVisible(), 'keyboard Enter exchanges owner code');
    const storage = await page.evaluate(async () => ({ local: localStorage.length, session: sessionStorage.length, cookies: document.cookie, cache: (await caches.keys()).length }));
    assert(storage.local === 0 && storage.session === 0 && storage.cookies === '' && storage.cache === 0, 'no browser credential storage or cache');
    await page.locator("button[data-view='search']").focus();
    await page.keyboard.press('Tab');
    assert(await page.evaluate(() => document.activeElement && document.activeElement.getAttribute('data-view') === 'record'), 'Tab navigation moves through viewer navigation');
    await page.locator("button[data-view='search']").focus();
    await page.keyboard.press('Enter');
    await page.locator('#search-namespace').fill('alpha');
    await page.locator('#search-query').fill('viewer long Unicode');
    await page.locator('#search-query').press('Enter');
    await waitText(page, '#search-results', 'viewer long Unicode');
    const stored = await page.locator('#search-results').textContent();
    assert(stored.includes('viewer long Unicode') && stored.includes('界'), 'long Unicode data rendered');
    await page.locator('#search-results pre.record-body').focus();
    assert(await page.evaluate(() => document.activeElement && document.activeElement.matches('pre.record-body')), 'long record body is keyboard focusable');
    assert(await page.evaluate(() => window.__hotr_stored !== 1 && document.querySelectorAll('#search-results script, #search-results img').length === 0), 'stored markup did not execute');
    assert(await page.locator('#search-results pre.record-body').evaluate(node => new TextEncoder().encode(node.textContent).length === 65536 && node.scrollHeight > node.clientHeight), 'exact maximum UTF-8 record is scrollable');
    await page.keyboard.press('PageDown');
    await page.waitForFunction(() => document.querySelector('#search-results pre.record-body').scrollTop > 0, undefined, {timeout:2000});
    assert(await page.locator('#search-results pre.record-body').evaluate(node => node.scrollTop > 0), 'keyboard scroll reaches long record text');
    await screenshot(page, 'HOTR-18-search.png');
    await page.locator("button[data-view='record']").click();
    await page.locator('#inspect-namespace').fill('alpha');
    await page.locator('#inspect-id').fill('active');
    await page.locator('#expected-revision').fill('1');
    await page.locator('#inspect-id').press('Enter');
    await waitText(page, '#inspect-results', 'revision');
    assert(/conflict/i.test(await page.locator('#inspect-results').textContent()), 'expected revision conflict is rendered');
    await page.waitForFunction(() => document.querySelectorAll('#history-results pre.record-body').length === 2, undefined, { timeout: 10_000 });
    const historyText = await page.locator('#history-results').textContent();
    assert(historyText.includes('r1-historical') && historyText.includes('r2-current') && historyText.includes('javascript:window.__hotr_stored=1'), 'history retains both revisions and plaintext source');
    assert((await page.locator('#inspect-results a').count()) === 0, 'stored source is plaintext not anchor');
    await page.locator('#retained-namespace').fill('alpha');
    await page.locator('#retained-namespace').press('Enter');
    await waitText(page, '#retained-results', 'hidden-retained');
    assert((await page.locator('#retained-results').textContent()).includes('hidden-retained'), 'retained hidden record available to owner');
    for (const [view, trigger, output, expected] of [['clients', '#clients-refresh', '#clients-results', 'viewer-reader'], ['index', '#index-refresh', '#index-results', 'generation'], ['backup', '#backup-refresh', '#backup-results', 'succeeded']]) {
      await page.locator(`button[data-view='${view}']`).click(); const response = page.waitForResponse(response => response.url().endsWith('/viewer/api/read') && response.status() === 200); await page.locator(trigger).click(); await response;
      await page.waitForFunction(({ output, expected }) => (document.querySelector(output)?.textContent || '').toLowerCase().includes(expected), { output, expected }, { timeout: 10_000 });
      assert((await page.locator(output).textContent()).toLowerCase().includes(expected), `${view} navigation and read`);
    }
    await page.locator("button[data-view='search']").click();
    await page.locator('#search-query').fill('no-such-viewer-query'); const emptyResponse = page.waitForResponse(response => response.url().endsWith('/viewer/api/read') && response.status() === 200); await page.locator('#search-query').press('Enter'); await emptyResponse;
    await waitText(page, '#search-results', 'No current visible records');
    assert((await page.locator('#search-results').textContent()).includes('No current visible records'), 'empty search state rendered');
    await page.locator("button[data-view='record']").click(); await page.locator('#inspect-id').fill('missing-viewer-record'); const errorResponse = page.waitForResponse(response => response.url().endsWith('/viewer/api/read') && response.status() === 404); await page.locator('#inspect-id').press('Enter'); await errorResponse;
    await waitText(page, '#inspect-results', 'Could not load this view');
    assert((await page.locator('#inspect-results').textContent()).includes('Could not load this view'), 'read error state rendered');
    const unauthenticated = await page.evaluate(async () => { const r = await fetch('/viewer/api/read', { method: 'POST', headers: { 'Content-Type': 'application/json', 'X-HOTR-Viewer': '1' }, body: JSON.stringify({ operation: 'ping' }) }); return r.status; });
    assert(unauthenticated === 401, 'unauthenticated browser read is denied');
    const foreign = await startForeignPage();
    const foreignPort = foreign.address().port;
    const foreignPage = await context.newPage();
    try { await foreignPage.goto(`http://127.0.0.1:${foreignPort}/`); await foreignPage.evaluate(({ target }) => { window.target = target; window.token = '0'.repeat(64); }, { target: `${origin}/viewer/api/read` }); await foreignPage.locator('#go').click(); await foreignPage.locator('#result').waitFor({ state: 'visible' }); assert((await foreignPage.locator('#result').textContent()) === 'blocked', 'foreign origin browser CSRF blocked'); } finally { await foreignPage.close(); await new Promise(resolve => { foreign.close(resolve); foreign.closeAllConnections(); }); }
    await page.bringToFront();
    if (await page.locator('#login-form').isVisible()) {
      const renewedCode = runSession(config, 600); await page.locator('#login-code').fill(renewedCode); const renewedNamespaces = page.waitForResponse(response => response.url().endsWith('/viewer/api/read') && response.status() === 200); await page.locator('#login-code').press('Enter'); await renewedNamespaces;
    }
    await page.locator("button[data-view='search']").click();
    let releaseHeld;
    let readyHeld;
    const heldReady = new Promise(resolve => { readyHeld = resolve; });
    let held = false;
    let heldDiagnostic = "search request not intercepted";
    await page.route('**/viewer/api/read', async route => {
      const body = route.request().postData() || '';
      if (!held && body.includes('"search"')) {
        held = true;
        try {
          const response = await route.fetch({timeout:5000, headers:{...await route.request().allHeaders(), origin, 'sec-fetch-site':'same-origin'}});
          const payload = await response.json();
          const release = new Promise(resolve => { releaseHeld = resolve; });
          heldDiagnostic = `status ${response.status()}, records ${Array.isArray(payload.records) ? payload.records.length : 'absent'}, error ${payload.error?.code || 'none'}`;
          readyHeld(response.status() === 200 && Array.isArray(payload.records) && payload.records.some(record => record.body.includes('HOTR07canary')));
          await release;
          await route.fulfill({ response }).catch(() => {});
        } catch (error) { heldDiagnostic = `route fetch failed: ${error.message}`; readyHeld(false); }
      } else { await route.continue().catch(() => {}); }
    });
    await page.locator('#search-namespace').fill('alpha');
    await page.locator('#search-query').fill('viewer');
    await page.locator('#search-query').press('Enter');
    const heldResult = await Promise.race([heldReady, new Promise(resolve => setTimeout(() => resolve(false), 10_000))]);
    if (!heldResult) { if (releaseHeld) releaseHeld(); fail(`controlled delayed response failed: ${heldDiagnostic}`); }
    assert(heldResult, 'controlled delayed response contains actual authorized context');
    await page.locator('#logout-button').click();
    releaseHeld();
    await page.waitForTimeout(350);
    assert(await page.locator('#login-form').isVisible() && !(await page.locator('#viewer-view').isVisible()), 'logout clears DOM and controlled pending response cannot repaint');
    const clearAfterLogout = await page.evaluate(() => ({
      form: Array.from(document.querySelectorAll('input')).every(input => input.value === ''),
      privateText: ['#search-results','#retained-results','#inspect-results','#history-results','#clients-results','#index-results','#backup-results','#search-page-label','#retained-page-label','#history-page-label','#clients-page-label'].map(selector => document.querySelector(selector).textContent).join(''),
      options: document.querySelectorAll('#namespace-options option').length,
      pagers: ['#search-pager','#retained-pager','#history-pager','#clients-pager'].every(selector => document.querySelector(selector).hidden)
    }));
    assert(clearAfterLogout.form && clearAfterLogout.privateText === '' && clearAfterLogout.options === 0 && clearAfterLogout.pagers, 'logout clears forms, data lists, pagers and private DOM');
    await page.reload({ waitUntil: 'networkidle' }); assert(await page.locator('#login-form').isVisible(), 'reload remains logged out');
    await page.goto('about:blank'); await page.goBack({waitUntil:'networkidle'}); assert(await page.locator('#login-form').isVisible(), 'back navigation does not restore viewer content');
    const expiryCode = runSession(config, 5); await page.locator('#login-code').fill(expiryCode); const expiryNamespaces = page.waitForResponse(response => response.url().endsWith('/viewer/api/read') && response.status() === 200); await page.locator('#login-code').press('Enter'); await page.locator('#viewer-view').waitFor({ state: 'visible' }); await expiryNamespaces; await page.waitForTimeout(5_600);
    await page.waitForFunction(() => !document.querySelector('#login-view').hidden, undefined, { timeout: 2_000 });
    assert(actualToken && await requestStatus(origin, actualToken) === 401, 'expired captured viewer token is denied by service');
    assert(await page.locator('#login-form').isVisible(), 'expiry timer clears viewer DOM');
    const pagehideCode = runSession(config, 30); await page.locator('#login-code').fill(pagehideCode); await page.locator('#login-code').press('Enter'); await page.locator('#viewer-view').waitFor({ state: 'visible' }); await page.goto('about:blank'); await page.goBack({ waitUntil: 'networkidle' }); assert(await page.locator('#login-form').isVisible(), 'pagehide clears session');
    const lockedCode = runSession(config, 30); await page.locator('#login-code').fill(lockedCode); const lockNamespaces = page.waitForResponse(response => response.url().endsWith('/viewer/api/read') && response.status() === 200); await page.locator('#login-code').press('Enter'); await page.locator('#viewer-view').waitFor({ state: 'visible' }); await lockNamespaces;
    await page.locator("button[data-view='clients']").click(); await waitText(page, '#clients-results', 'viewer-reader');
    lockVault(config);
    await page.waitForFunction(() => !document.querySelector('#login-view').hidden, undefined, { timeout: 10_000 });
    assert(await page.locator('#login-form').isVisible(), 'locked service state clears viewer DOM');
    assert(pageHttpRequests.every(url => new URL(url).origin === origin), 'viewer page made no non-loopback requests');
    await screenshot(page, 'HOTR-18-final.png');
    const profileFiles = fs.readdirSync(profile); assert(profileFiles.includes('SYNTHETIC-ONLY'), 'isolated profile marker retained');
  } finally { await context.close(); }
  evidence.credential_disk_scan = scanEphemeralCredentials(config.run);
  assert(evidence.credential_disk_scan.utf8_utf16le_absent, 'all exchanged viewer credentials absent from retained fixture and Chrome profile');
  newFile(path.join(config.run, 'HOTR-18-browser.json'), JSON.stringify(evidence, null, 2));
  process.stdout.write(JSON.stringify({ prompt: evidence.prompt, result: evidence.result, browser: evidence.browser, headless: evidence.headless, assertions: evidence.assertions }) + '\n');
}
main().catch(error => { process.stderr.write(`HOTR-18 browser gate failed: ${String(error.message).replace(/[a-f0-9]{64}/gi, '[redacted]')}\n`); process.exitCode = 1; });
