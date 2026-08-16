const test = require("node:test");
const assert = require("node:assert/strict");

const { routeEditorCommand } = require("../src/commands.cjs");

test("Editor routes catalog reads to Core and mutations to the Editor service", () => {
  assert.equal(routeEditorCommand("GET_CATALOG"), "core");
  for (const command of ["GET_EDITOR_DATA", "APPLY_PLAYER_EDIT", "GET_LOCKS", "SET_LOCK", "MOVE_PLAYER"]) {
    assert.equal(routeEditorCommand(command), "editor", command);
  }
  for (const command of ["GET_DASHBOARD", "APPLY_TIER_PROFILE", "GET_MOCK_DRAFT", "SET_DRAFT_SETTINGS"]) {
    assert.equal(routeEditorCommand(command), null, command);
  }
});
