const assert = require("node:assert/strict");
const test = require("node:test");

const LatestRequest = require("../renderer/latest-request.js");

test("profile previews run one at a time and coalesce to the newest selection", async () => {
  let releaseFirst;
  let active = 0;
  let maximumActive = 0;
  const runs = [];
  const applied = [];
  const runner = LatestRequest.create({
    execute: async (value) => {
      runs.push(value);
      active += 1;
      maximumActive = Math.max(maximumActive, active);
      if (value === "A") await new Promise((resolve) => { releaseFirst = resolve; });
      active -= 1;
      return `${value}-result`;
    },
    apply: (value) => applied.push(value),
  });

  const first = runner.submit("A");
  await new Promise((resolve) => setImmediate(resolve));
  const second = runner.submit("B");
  const third = runner.submit("C");
  releaseFirst();
  await Promise.all([first, second, third]);

  assert.equal(maximumActive, 1);
  assert.deepEqual(runs, ["A", "C"]);
  assert.deepEqual(applied, ["C-result"]);
});
