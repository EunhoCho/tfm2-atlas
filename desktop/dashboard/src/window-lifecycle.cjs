function loadWindowAndShow(window, filePath) {
  let revealed = false;
  const reveal = () => {
    if (revealed || window.isDestroyed()) return;
    revealed = true;
    window.show();
    window.focus();
  };

  window.once("ready-to-show", reveal);
  window.webContents.once("did-finish-load", reveal);
  return window.loadFile(filePath);
}

module.exports = { loadWindowAndShow };
