const { app, BrowserWindow, ipcMain, shell } = require("electron");
const fs = require("node:fs");
const path = require("node:path");

const { BridgeClient, StateEventClient } = require("./bridge.cjs");
const { routeEditorCommand } = require("./commands.cjs");
const { loadWindowAndShow } = require("./window-lifecycle.cjs");

const coreBridge = new BridgeClient({ port: 28452 });
const editorBridge = new BridgeClient({ port: 28453 });
const editorEvents = new StateEventClient({ port: 28453 });
let mainWindow;

function settingsPath() { return path.join(app.getPath("userData"), "settings.json"); }
function loadSettings() {
  try { const value = JSON.parse(fs.readFileSync(settingsPath(), "utf8")); return { language: value.language === "en" ? "en" : "ko" }; }
  catch { return { language: "ko" }; }
}
function saveLanguage(language) {
  const next = { language: language === "en" ? "en" : "ko" };
  fs.mkdirSync(path.dirname(settingsPath()), { recursive: true });
  fs.writeFileSync(settingsPath(), JSON.stringify(next, null, 2), "utf8");
  return next;
}
function createWindow() {
  if (mainWindow && !mainWindow.isDestroyed()) { mainWindow.show(); mainWindow.focus(); return mainWindow; }
  mainWindow = new BrowserWindow({
    width: 1500, height: 950, minWidth: 1100, minHeight: 700, show: false,
    title: "TFM2 Atlas Editor", icon: path.join(__dirname, "..", "assets", "atlas-editor.png"), backgroundColor: "#080c12",
    webPreferences: { preload: path.join(__dirname, "preload.cjs"), contextIsolation: true, nodeIntegration: false, sandbox: true },
  });
  mainWindow.removeMenu();
  loadWindowAndShow(mainWindow, path.join(__dirname, "..", "renderer", "editor.html"));
  mainWindow.on("closed", () => { mainWindow = null; });
  return mainWindow;
}

ipcMain.handle("settings:get", () => loadSettings());
ipcMain.handle("settings:language", (_event, language) => saveLanguage(language));
ipcMain.handle("bridge:request", (_event, command, payload) => {
  const route = routeEditorCommand(command);
  if (route === "core") return coreBridge.request(command, payload || {});
  if (route === "editor") return editorBridge.request(command, payload || {});
  throw new Error("command_not_allowed");
});
ipcMain.handle("external:open", (_event, url) => {
  if (!/^https:\/\/github\.com\/jal-io\/tfm2-editor\/?$/.test(url)) return false;
  shell.openExternal(url); return true;
});

const gotLock = app.requestSingleInstanceLock();
if (!gotLock) app.quit();
else {
  app.on("second-instance", () => { if (mainWindow?.isMinimized()) mainWindow.restore(); createWindow(); });
  app.whenReady().then(() => {
    editorEvents.start((message) => { if (mainWindow && !mainWindow.isDestroyed()) mainWindow.webContents.send("state:changed", message); });
    createWindow();
  });
  app.on("activate", () => { if (BrowserWindow.getAllWindows().length === 0) createWindow(); });
  app.on("window-all-closed", () => { editorEvents.stop(); app.quit(); });
}
