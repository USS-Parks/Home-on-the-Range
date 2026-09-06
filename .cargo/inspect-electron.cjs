'use strict';
// Inspect installed package code only. No application launch or profile reads.
const fs = require('node:fs');
const path = require('node:path');
const crypto = require('node:crypto');
const asar = require(path.join(process.env.USERPROFILE, 'Documents/Claude/Lamprey Harness/node_modules/@electron/asar'));
for (const app of ['Qwen', 'Chatbox', 'Grok Bot', '@openworkdesktop', '@opencode-aidesktop']) {
  const archive = path.join(process.env.LOCALAPPDATA, 'Programs', app, 'resources/app.asar');
  if (!fs.existsSync(archive)) continue;
  const manifest = JSON.parse(asar.extractFile(archive, 'package.json'));
  const entries = asar.listPackage(archive).filter(name => !name.includes('node_modules') && /mcp|preload|main\.(js|cjs)|package\.json/i.test(name)).slice(0, 45);
  const entry = manifest.main?.replace(/^\.\//, '');
  let main = '', inspection_error;
  try { main = entry ? asar.extractFile(archive, path.normalize(entry)).toString('utf8') : ''; }
  catch (error) { inspection_error = error.message; }
  const matches = [...main.matchAll(/mcp.{0,55}|userData.{0,80}|setPath.{0,100}|preload.{0,100}|requestSingleInstanceLock.{0,80}|--user-data-dir.{0,80}/gi)]
    .slice(0, 35).map(item => item[0]);
  console.log(JSON.stringify({ app, version:manifest.version, main:entry, inspection_error, main_sha256:main ? crypto.createHash('sha256').update(main).digest('hex') : null, entries, matches }));
}
