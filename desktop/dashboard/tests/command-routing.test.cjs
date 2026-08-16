const test = require("node:test");
const assert = require("node:assert/strict");

const { allowsDashboardCommand } = require("../src/commands.cjs");

test("Dashboard accepts only Core statistics, tier, catalog and mock Draft commands", () => {
  for (const command of ["GET_DASHBOARD", "PREVIEW_TIER_PROFILE", "APPLY_TIER_PROFILE", "GET_CATALOG", "GET_MOCK_DRAFT"]) {
    assert.equal(allowsDashboardCommand(command), true, command);
  }
  for (const command of ["GET_EDITOR_DATA", "APPLY_PLAYER_EDIT", "GET_LOCKS", "SET_LOCK", "MOVE_PLAYER"]) {
    assert.equal(allowsDashboardCommand(command), false, command);
  }
});
