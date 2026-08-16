const assert = require("node:assert/strict");
const test = require("node:test");

const RefreshPlan = require("../renderer/refresh-plan.js");

test("unrelated editor events do not replace dashboard profile controls", () => {
  assert.deepEqual(RefreshPlan.fromScopes(["EDITOR_CHANGED"], "statistics"), {
    relevant: false,
    analytics: false,
    catalog: false,
    draft: false,
  });
});

test("draft events refresh draft data only while the draft page is visible", () => {
  assert.equal(RefreshPlan.fromScopes(["DRAFT_CHANGED"], "statistics").relevant, false);
  assert.deepEqual(RefreshPlan.fromScopes(["DRAFT_CHANGED"], "draft"), {
    relevant: true,
    analytics: false,
    catalog: false,
    draft: true,
  });
});
