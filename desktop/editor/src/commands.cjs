const CORE_COMMANDS = new Set(["GET_CATALOG"]);
const EDITOR_COMMANDS = new Set([
  "GET_EDITOR_DATA",
  "APPLY_EDITOR_SETTINGS",
  "GET_PLAYER_MASTERY",
  "SET_PLAYER_MASTERY",
  "APPLY_PLAYER_EDIT",
  "APPLY_STAFF_EDIT",
  "MOVE_PLAYER",
  "GET_LOCKS",
  "SET_LOCK",
  "UNLOCK",
]);

function routeEditorCommand(command) {
  if (CORE_COMMANDS.has(command)) return "core";
  if (EDITOR_COMMANDS.has(command)) return "editor";
  return null;
}

module.exports = { CORE_COMMANDS, EDITOR_COMMANDS, routeEditorCommand };
