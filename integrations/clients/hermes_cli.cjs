'use strict';

// Drive the installed Hermes CLI. Its own MCP client, provider and session DB
// provide the evidence; no provider or tool dispatch is replaced by this driver.
const fs = require('node:fs');
const path = require('node:path');
const crypto = require('node:crypto');
const assert = require('node:assert/strict');
const { spawnSync } = require('node:child_process');
const { DatabaseSync } = require('node:sqlite');
const { reserve } = require('./prompt_budget.cjs');
const { normalize } = require('./hermes_results.cjs');
const root = path.resolve(__dirname, '../..');
const allowed = ['hotr_health', 'hotr_search', 'hotr_get', 'hotr_create', 'hotr_revise'];
const model = 'claude-sonnet-5';
const secret = process.env.ANTHROPIC_API_KEY || '';
const redact = value => secret ? String(value || '').split(secret).join('[REDACTED PROVIDER CREDENTIAL]') : String(value || '');

function checked(value, parent, directory = false) {
  assert.equal(typeof value, 'string');
  // Rust's canonical Windows paths use the verbatim drive prefix. Strip that
  // spelling before walking ancestors; do not resolve links before checking.
  const local = value.startsWith('\\\\?\\') && /^[A-Za-z]:[\\/]/.test(value.slice(4))
    ? value.slice(4) : value;
  for (let item = path.resolve(local); path.dirname(item) !== item; item = path.dirname(item)) {
    assert.equal(fs.lstatSync(item).isSymbolicLink(), false, 'Reparse point refused');
  }
  const absolute = fs.realpathSync(local);
  const relative = path.relative(parent, absolute);
  assert.ok(relative && !relative.startsWith('..') && !path.isAbsolute(relative), 'Path escaped synthetic workspace');
  assert.equal(fs.statSync(absolute).isDirectory(), directory);
  return absolute;
}

function openState(profile) {
  const file = checked(path.join(profile, 'state.db'), profile);
  return new DatabaseSync(file, {readOnly:true});
}

function main() {
  const input = fs.readFileSync(0);
  assert.ok(input.length <= 16384, 'Driver request limit');
  const request = JSON.parse(input);
  assert.ok(['preflight','turn'].includes(request.mode));
  const profile = checked(request.profile, path.join(root, 'work/hotr-client-profiles'), true);
  assert.equal(fs.readFileSync(path.join(profile, 'SYNTHETIC-ONLY'), 'utf8'), 'HOTR-12A; isolated Hermes proof\n');
  const hotr = checked(request.hotr, path.join(root, 'work/hotr-build'));
  const credential = checked(request.credential, path.join(root, 'work/hotr-tests'));
  const python = path.join(process.env.LOCALAPPDATA, 'hermes/hermes-agent/venv/Scripts/python.exe');
  const source = path.join(process.env.LOCALAPPDATA, 'hermes/hermes-agent');
  const version = fs.readFileSync(path.join(source, 'pyproject.toml'), 'utf8').match(/^version\s*=\s*"([^"]+)"/m)?.[1];
  assert.ok(version, 'Installed Hermes version unavailable');
  const environment = {
    HERMES_HOME:profile, HERMES_MAX_TOKENS:'1024', PYTHONDONTWRITEBYTECODE:'1',
    PYTHONUTF8:'1', NO_COLOR:'1', DO_NOT_TRACK:'1',
  };
  for (const name of ['SYSTEMROOT','SystemRoot','WINDIR','COMSPEC','ComSpec','PATH','PATHEXT','USERPROFILE','LOCALAPPDATA','APPDATA','TEMP','TMP']) {
    if (process.env[name]) environment[name] = process.env[name];
  }
  const configFile = path.join(profile, 'config.yaml');
  if (request.mode === 'preflight') {
    const config = {
      model:{default:model,provider:'anthropic',max_tokens:1024}, fallback_providers:[],
      toolsets:['mcp-hotr'], agent:{max_turns:8,api_max_retries:1},
      memory:{memory_enabled:false,user_profile_enabled:false,provider:''},
      compression:{enabled:false}, plugins:{enabled:[]}, hooks:{},
      auxiliary:{title_generation:{enabled:false},background_review:{enabled:false}},
      checkpoints:{enabled:false},
      mcp_servers:{hotr:{command:hotr,args:['mcp','--credential',credential],trust:'full',
        connect_timeout:15,tool_timeout:15,lazy:false,
        tools:{include:allowed,resources:false,prompts:false}}},
    };
    // JSON is valid YAML. Only this newly marked profile is written.
    fs.writeFileSync(configFile, JSON.stringify(config, null, 2)+'\n', {flag:'wx'});
    const probe = spawnSync(python, ['-I','-B','-m','hermes_cli.main','mcp','test','hotr'], {
      cwd:profile, env:environment, windowsHide:true, encoding:'utf8', timeout:45000,maxBuffer:1048576,
    });
    const stdout = redact(probe.stdout);
    const pass = probe.status === 0 && /Connected\s*\(/.test(stdout)
      && /Tools discovered:\s*5/.test(stdout) && allowed.every(name => stdout.includes(name));
    return {result:pass?'PREFLIGHT_PASS':'FAIL',version,model_prompts:0,exit_code:probe.status,
      error:probe.error?.code,stdout,stderr:redact(probe.stderr),
      installed_main_sha256:crypto.createHash('sha256').update(fs.readFileSync(path.join(source,'hermes_cli/main.py'))).digest('hex'),
      active_profile_changed:false};
  }
  checked(configFile, profile);
  assert.ok(secret.length > 20, 'Existing Anthropic credential unavailable');
  assert.ok(typeof request.prompt === 'string' && Buffer.byteLength(request.prompt) <= 8192);
  assert.match(request.label, /^[a-z0-9-]{1,80}$/);
  let previous = 0;
  if (fs.existsSync(path.join(profile, 'state.db'))) {
    const db = openState(profile);
    try { previous = db.prepare('SELECT coalesce(max(id),0) AS id FROM messages').get().id; }
    finally { db.close(); }
  }
  environment.ANTHROPIC_API_KEY = secret;
  const attempt = reserve({app:'hermes',model,provider:'anthropic',label:request.label});
  const result = spawnSync(python, ['-I','-B','-m','hermes_cli.main','chat','--cli',
    '--query-file','-','--provider','anthropic','--model',model,'--toolsets','mcp-hotr',
    '--max-turns','8','--run-budget','160','--ignore-rules'], {
    cwd:profile,env:environment,input:request.prompt,windowsHide:true,encoding:'utf8',timeout:180000,maxBuffer:8*1024*1024,
  });
  const response = {result:'FAIL',version,model,attempt,exit_code:result.status,error:result.error?.code,
    stdout:redact(result.stdout),stderr:redact(result.stderr),calls:[],tool_results:[],sessions:[],usage:[]};
  if (!fs.existsSync(path.join(profile, 'state.db'))) return response;
  const db = openState(profile);
  try {
    const messages = db.prepare('SELECT id,session_id,role,content,tool_call_id,tool_calls,tool_name FROM messages WHERE id > ? ORDER BY id LIMIT 100').all(previous);
    assert.ok(messages.length < 100, 'Native transcript exceeded message limit');
    for (const message of messages) {
      if (message.role === 'assistant' && message.tool_calls) {
        for (const call of JSON.parse(message.tool_calls)) response.calls.push(call);
      }
      if (message.role === 'tool') response.tool_results.push({id:message.tool_call_id,name:message.tool_name,result:message.content});
    }
    const ids = [...new Set(messages.map(message => message.session_id))];
    for (const id of ids) {
      response.sessions.push(db.prepare('SELECT id,model,billing_provider,tool_call_count FROM sessions WHERE id = ?').get(id));
      response.usage.push(...db.prepare('SELECT model,billing_provider,api_call_count,input_tokens,output_tokens FROM session_model_usage WHERE session_id = ?').all(id));
    }
    response.native_calls = response.calls;
    response.native_tool_results = response.tool_results;
    try { Object.assign(response, normalize(response.native_calls, response.native_tool_results)); }
    catch (error) { response.error = redact(error.message); return response; }
    response.result = result.status === 0
      && response.sessions.length === 1 && response.sessions.every(session => session.model === model)
      && response.usage.length > 0 && response.usage.every(row =>
        row.model === model && row.billing_provider === 'anthropic' && row.api_call_count > 0)
      ? 'PASS' : 'FAIL';
  } finally { db.close(); }
  return response;
}

try { const result = main(); console.log(redact(JSON.stringify(result))); if (result.result === 'FAIL') process.exitCode = 1; }
catch (error) { console.log(JSON.stringify({result:'FAIL',error:redact(error.message)})); process.exitCode = 1; }
