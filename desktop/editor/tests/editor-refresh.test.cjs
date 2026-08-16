const assert = require("node:assert/strict");
const test = require("node:test");

const EditorModel = require("../renderer/editor-model.js");

test("index and analytics events do not trigger an Editor-wide record reload", () => {
  assert.equal(EditorModel.shouldRefreshForScopes(["INDEX_CHANGED"]), false);
  assert.equal(EditorModel.shouldRefreshForScopes(["ANALYTICS_CHANGED"]), false);
  assert.equal(EditorModel.shouldRefreshForScopes(["CATALOG_CHANGED"]), false);
  assert.equal(EditorModel.shouldRefreshForScopes(["EDITOR_CHANGED"]), true);
});

test("overlapping Editor refreshes share one bridge request", async () => {
  let calls = 0;
  let complete;
  const request = EditorModel.singleFlight(() => {
    calls += 1;
    return new Promise((resolve) => { complete = resolve; });
  });

  const first = request();
  const second = request();
  assert.equal(first, second);
  assert.equal(calls, 1);

  complete("overview");
  assert.deepEqual(await Promise.all([first, second]), ["overview", "overview"]);

  const third = request();
  assert.equal(calls, 2);
  complete("next");
  assert.equal(await third, "next");
});
