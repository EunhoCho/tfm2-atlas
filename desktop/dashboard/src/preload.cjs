const { contextBridge, ipcRenderer } = require("electron");

contextBridge.exposeInMainWorld("tfm2", {
  settings: () => ipcRenderer.invoke("settings:get"),
  setLanguage: (language) => ipcRenderer.invoke("settings:language", language),
  saveProfile: (profile) => ipcRenderer.invoke("settings:profile", profile),
  request: (command, payload = {}) => ipcRenderer.invoke("bridge:request", command, payload),
  exportTierTsv: (tsv) => ipcRenderer.invoke("tsv:export", tsv),
  importTierTsv: () => ipcRenderer.invoke("tsv:import"),
  openExternal: (url) => ipcRenderer.invoke("external:open", url),
  onStateChanged: (listener) => {
    const handler = (_event, message) => listener(message);
    ipcRenderer.on("state:changed", handler);
    return () => ipcRenderer.removeListener("state:changed", handler);
  },
});
