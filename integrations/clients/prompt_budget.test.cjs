'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const { reserveIn } = require('./prompt_budget.cjs');
const root = path.resolve(__dirname, '../../work/hotr-tests');
fs.mkdirSync(root, { recursive: true });
const run = fs.mkdtempSync(path.join(root, 'HOTR-budget-'));
fs.writeFileSync(path.join(run, 'SYNTHETIC-ONLY'), 'HOTR compatibility budget tests; no inference\n', { flag: 'wx' });
const entry = app => ({ app, model:'synthetic-no-model', provider:'none' });

test('shared cap survives distinct callers and never allocates attempt 73', () => {
  const directory = path.join(run, 'shared');
  for (let index = 0; index < 72; index++) {
    assert.equal(reserveIn(directory, entry(`client-${Math.floor(index / 8)}`)), index + 1);
  }
  assert.throws(() => reserveIn(directory, entry('another-client')), /72-prompt/);
  assert.equal(fs.readdirSync(directory).length, 72);
});
test('per-app ceilings charge failed reservations instead of resetting history', () => {
  for (const [app, limit] of [['lamprey', 12], ['hermes', 8]]) {
    const directory = path.join(run, app);
    for (let count = 0; count < limit; count++) reserveIn(directory, entry(app));
    assert.throws(() => reserveIn(directory, entry(app)), /Per-application/);
    assert.equal(fs.readdirSync(directory).length, limit + 1);
  }
});
test('corrupt reservations and paths outside the test boundary fail closed', () => {
  assert.throws(() => reserveIn(path.dirname(root), entry('hermes')), /outside project work/);
  const directory = path.join(run, 'corrupt');
  fs.mkdirSync(directory);
  fs.writeFileSync(path.join(directory, 'attempt-01.json'), '{', { flag:'wx' });
  assert.throws(() => reserveIn(directory, entry('hermes')), SyntaxError);
});
