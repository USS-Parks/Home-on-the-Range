'use strict';

// Loaded while the installed application's first statement is paused. Only
// paths, window visibility and debugger inheritance change; application IPC,
// providers, hooks, permissions and MCP dispatch remain the installed code.
const { app, BrowserWindow, globalShortcut } = require('electron');
const fs = require('node:fs');
const path = require('node:path');
const profile = fs.realpathSync(process.env.HOTR_LAMPREY_PROFILE);
if (app.isReady() || !fs.readFileSync(path.join(profile, 'SYNTHETIC-ONLY'), 'utf8').startsWith('HOTR-12-LAMPREY')) {
  throw new Error('Refusing late or unmarked application isolation');
}
app.setPath('userData', profile);
app.setPath('sessionData', path.join(profile, 'session-data'));
app.setPath('crashDumps', path.join(profile, 'crash-dumps'));
app.setAppLogsPath(path.join(profile, 'logs'));
process.execArgv = process.execArgv.filter(arg => !arg.startsWith('--inspect'));
const workers = require('node:worker_threads');
const OriginalWorker = workers.Worker;
workers.Worker = class extends OriginalWorker {
  constructor(file, options = {}) {
    super(file, { ...options, execArgv: (options.execArgv || process.execArgv).filter(arg => !arg.startsWith('--inspect')) });
  }
};
BrowserWindow.prototype.show = () => {};
BrowserWindow.prototype.showInactive = () => {};
BrowserWindow.prototype.focus = () => {};
globalShortcut.register = () => false;
global.__hotrIsolation = () => ({
  version: app.getVersion(), packaged: app.isPackaged, appPath: app.getAppPath(),
  userData: app.getPath('userData'), sessionData: app.getPath('sessionData'),
  logs: app.getPath('logs'), crashDumps: app.getPath('crashDumps'),
  windows: BrowserWindow.getAllWindows().map(window => ({ visible: window.isVisible(), focused: window.isFocused() }))
});
global.__hotrQuit = () => app.quit();
