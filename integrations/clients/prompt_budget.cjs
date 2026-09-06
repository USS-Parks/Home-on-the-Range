'use strict';

// The owner approved one shared 72-prompt compatibility allowance. Reservations
// are durable before inference; failed starts remain charged. No ledger reset.
const fs = require('node:fs');
const path = require('node:path');
const assert = require('node:assert/strict');
const ROOT = path.resolve(__dirname, '../..');
const DIRECTORY = path.join(ROOT, 'work/hotr-evidence/HOTR-compatibility-budget-20260906');

function reserveIn(directory, entry) {
  const relative = path.relative(path.join(ROOT, 'work'), path.resolve(directory));
  assert.ok(relative && !relative.startsWith('..') && !path.isAbsolute(relative), 'Budget directory outside project work');
  assert.match(entry.app, /^[a-z][a-z0-9-]{0,40}$/);
  assert.ok(typeof entry.model === 'string' && entry.model.length <= 200);
  assert.ok(typeof entry.provider === 'string' && entry.provider.length <= 100);
  for (let item = directory; path.dirname(item) !== item; item = path.dirname(item)) {
    if (fs.existsSync(item)) assert.equal(fs.lstatSync(item).isSymbolicLink(), false, 'Budget path is a reparse point');
  }
  fs.mkdirSync(directory, { recursive: true });
  for (let attempt = 1; attempt <= 72; attempt++) {
    const file = path.join(directory, `attempt-${String(attempt).padStart(2, '0')}.json`);
    let fd;
    try { fd = fs.openSync(file, 'wx'); } catch (error) {
      if (error.code === 'EEXIST') continue;
      throw error;
    }
    try {
      fs.writeFileSync(fd, JSON.stringify({ ...entry, attempt, started_unix: Date.now() / 1000 }) + '\n');
      fs.fsyncSync(fd);
    } finally { fs.closeSync(fd); }
    // Count after reserving: concurrent contenders fail conservatively. A
    // partial/corrupt reservation also fails closed, never grants another slot.
    const rows = fs.readdirSync(directory).filter(name => /^attempt-\d\d\.json$/.test(name))
      .map(name => JSON.parse(fs.readFileSync(path.join(directory, name), 'utf8')));
    assert.ok(rows.filter(row => row.app === entry.app).length <= (entry.app === 'lamprey' ? 12 : 8), 'Per-application compatibility budget exhausted');
    return attempt;
  }
  throw new Error('Shared 72-prompt compatibility budget exhausted');
}

function reserve(entry) { return reserveIn(DIRECTORY, entry); }
module.exports = { reserve, reserveIn };
