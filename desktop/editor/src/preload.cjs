const { contextBridge, ipcRenderer } = require("electron");

contextBridge.exposeInMainWorld("tfm2", {
  settings: () => ipcRenderer.invoke("settings:get"),
  setLanguage: (language) => ipcRenderer.invoke("settings:language", language),
  request: (command, payload = {}) => ipcRenderer.invoke("bridge:request", command, payload),
  openExternal: (url) => ipcRenderer.invoke("external:open", url),
  onStateChanged: (listener) => {
    const handler = (_event, message) => listener(message);
    ipcRenderer.on("state:changed", handler);
    return () => ipcRenderer.removeListener("state:changed", handler);
  },
});
