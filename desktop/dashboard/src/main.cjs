const { app, BrowserWindow, dialog, ipcMain, shell } = require("electron");
const fs = require("node:fs");
const path = require("node:path");

const { BridgeClient, StateEventClient } = require("./bridge.cjs");
const { allowsDashboardCommand } = require("./commands.cjs");
const { messages } = require("./i18n.cjs");
const { normalizeProfile } = require("./profile.cjs");
const { loadWindowAndShow } = require("./window-lifecycle.cjs");

const bridge = new BridgeClient({ port: 28452 });
const stateEvents = new StateEventClient({ port: 28452 });
const MAX_TSV_BYTES = 4 * 1024 * 1024;
let mainWindow;

function settingsPath() { return path.join(app.getPath("userData"), "settings.json"); }
function loadSettings() {
  try {
    const value = JSON.parse(fs.readFileSync(settingsPath(), "utf8"));
    return { language: value.language === "en" ? "en" : "ko", lastProfile: value.lastProfile ? normalizeProfile(value.lastProfile) : null };
  } catch { return { language: "ko", lastProfile: null }; }
}
function saveSettings(patch) {
  const next = { ...loadSettings(), ...patch };
  fs.mkdirSync(path.dirname(settingsPath()), { recursive: true });
  fs.writeFileSync(settingsPath(), JSON.stringify(next, null, 2), "utf8");
  return next;
}
function createWindow() {
  if (mainWindow && !mainWindow.isDestroyed()) { mainWindow.show(); mainWindow.focus(); return mainWindow; }
  mainWindow = new BrowserWindow({
    width: 1500, height: 950, minWidth: 1100, minHeight: 700, show: false,
    title: "TFM2 Atlas Dashboard", icon: path.join(__dirname, "..", "assets", "atlas-dashboard.png"), backgroundColor: "#080c12",
    webPreferences: { preload: path.join(__dirname, "preload.cjs"), contextIsolation: true, nodeIntegration: false, sandbox: true },
  });
  mainWindow.removeMenu();
  loadWindowAndShow(mainWindow, path.join(__dirname, "..", "renderer", "index.html"));
  mainWindow.on("closed", () => { mainWindow = null; });
  return mainWindow;
}

ipcMain.handle("settings:get", () => ({ ...loadSettings(), messages }));
ipcMain.handle("settings:language", (_event, language) => saveSettings({ language: language === "en" ? "en" : "ko" }));
ipcMain.handle("settings:profile", (_event, profile) => saveSettings({ lastProfile: normalizeProfile(profile) }));
ipcMain.handle("bridge:request", (_event, command, payload) => {
  if (!allowsDashboardCommand(command)) throw new Error("command_not_allowed");
  return bridge.request(command, payload || {});
});
ipcMain.handle("tsv:export", async (_event, tsv) => {
  if (typeof tsv !== "string" || Buffer.byteLength(tsv, "utf8") > MAX_TSV_BYTES) throw new Error("invalid_tsv_size");
  const result = await dialog.showSaveDialog(mainWindow, { title: "Export champion tier policy", defaultPath: "champion_tier_policy.tsv", filters: [{ name: "TSV", extensions: ["tsv"] }] });
  if (result.canceled || !result.filePath) return { ok: false, canceled: true };
  fs.writeFileSync(result.filePath, tsv, "utf8");
  return { ok: true, path: result.filePath };
});
ipcMain.handle("tsv:import", async () => {
  const result = await dialog.showOpenDialog(mainWindow, { title: "Validate champion tier policy", properties: ["openFile"], filters: [{ name: "TSV", extensions: ["tsv"] }] });
  if (result.canceled || result.filePaths.length !== 1) return { ok: false, canceled: true };
  const filePath = result.filePaths[0];
  if (fs.statSync(filePath).size > MAX_TSV_BYTES) throw new Error("tier_tsv_too_large");
  return { ok: true, path: filePath, tsv: fs.readFileSync(filePath, "utf8") };
});
ipcMain.handle("external:open", (_event, url) => {
  if (!/^https:\/\/(gall\.dcinside\.com)\//.test(url)) return false;
  shell.openExternal(url); return true;
});

const gotLock = app.requestSingleInstanceLock();
if (!gotLock) app.quit();
else {
  app.on("second-instance", () => { if (mainWindow?.isMinimized()) mainWindow.restore(); createWindow(); });
  app.whenReady().then(() => {
    stateEvents.start((message) => { if (mainWindow && !mainWindow.isDestroyed()) mainWindow.webContents.send("state:changed", message); });
    createWindow();
  });
  app.on("activate", () => { if (BrowserWindow.getAllWindows().length === 0) createWindow(); });
  app.on("window-all-closed", () => { stateEvents.stop(); app.quit(); });
}
