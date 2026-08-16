const assert = require("node:assert/strict");
const { EventEmitter } = require("node:events");
const test = require("node:test");

const { loadWindowAndShow } = require("../src/window-lifecycle.cjs");

function fakeWindow(loadEvent) {
  const window = new EventEmitter();
  window.webContents = new EventEmitter();
  window.shown = 0;
  window.focused = 0;
  window.isDestroyed = () => false;
  window.show = () => { window.shown += 1; };
  window.focus = () => { window.focused += 1; };
  window.loadFile = () => {
    if (loadEvent === "ready-to-show") window.emit("ready-to-show");
    if (loadEvent === "did-finish-load") window.webContents.emit("did-finish-load");
    return Promise.resolve();
  };
  return window;
}

test("a fast cached window is revealed even when ready-to-show fires during loadFile", async () => {
  const window = fakeWindow("ready-to-show");

  await loadWindowAndShow(window, "index.html");

  assert.equal(window.shown, 1);
  assert.equal(window.focused, 1);
});

test("did-finish-load reveals a window when ready-to-show is not emitted", async () => {
  const window = fakeWindow("did-finish-load");

  await loadWindowAndShow(window, "index.html");

  assert.equal(window.shown, 1);
  assert.equal(window.focused, 1);
});
