'use strict';
const fs = require('node:fs');
const path = require('node:path');
const { spawnSync } = require('node:child_process');
const base = path.resolve(__dirname, '../work/hotr-client-profiles');
for (let item = base; path.dirname(item) !== item; item = path.dirname(item)) {
  if (fs.existsSync(item) && fs.lstatSync(item).isSymbolicLink()) throw new Error('Reparse point refused');
}
fs.mkdirSync(base, {recursive:true});
const profile = fs.mkdtempSync(path.join(base, 'HOTR-HERMES-HELP-'));
fs.writeFileSync(path.join(profile, 'SYNTHETIC-ONLY'), 'HOTR-12A; help-only installation probe\n', {flag:'wx'});
const environment = {HERMES_HOME:profile, PYTHONDONTWRITEBYTECODE:'1', NO_COLOR:'1', DO_NOT_TRACK:'1'};
for (const name of ['SYSTEMROOT','SystemRoot','WINDIR','COMSPEC','ComSpec','PATH','PATHEXT','USERPROFILE','LOCALAPPDATA','APPDATA']) {
  if (process.env[name]) environment[name] = process.env[name];
}
const python = path.join(process.env.LOCALAPPDATA, 'hermes/hermes-agent/venv/Scripts/python.exe');
for (const command of [['--help'], ['chat','--help']]) {
  const result = spawnSync(python, ['-I','-B','-m','hermes_cli.main',...command], {
    cwd:profile, env:environment, windowsHide:true, encoding:'utf8', timeout:30000, maxBuffer:1048576
  });
  console.log(JSON.stringify({command, status:result.status, error:result.error?.code, stdout:result.stdout, stderr:result.stderr}));
  if (result.status !== 0) { process.exitCode=1; break; }
}
