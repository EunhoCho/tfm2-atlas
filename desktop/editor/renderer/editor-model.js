(function exposeEditorModel(root, factory) {
  const model = factory();
  if (typeof module === "object" && module.exports) module.exports = model;
  if (root) root.EditorModel = model;
})(typeof window !== "undefined" ? window : globalThis, function createEditorModel() {
  function orderedTeams(teams) {
    return [...(teams || [])].sort((left, right) =>
      Number(Boolean(right.playerTeam)) - Number(Boolean(left.playerTeam))
      || String(left.name || "").localeCompare(String(right.name || ""))
      || Number(left.id) - Number(right.id));
  }

  function filterEntities(rows, query, teamFilter, teams) {
    const normalized = String(query || "").trim().toLowerCase();
    const selected = (teams || []).find((team) => String(team.id) === String(teamFilter));
    return (rows || []).filter((row) => {
      const matchesQuery = !normalized
        || String(row.name || "").toLowerCase().includes(normalized)
        || String(row.team || "").toLowerCase().includes(normalized);
      const matchesTeam = !selected || row.team === selected.name;
      return matchesQuery && matchesTeam;
    });
  }

  function shouldRefreshForScopes(scopes) {
    return (scopes || []).includes("EDITOR_CHANGED");
  }

  function singleFlight(task) {
    let inFlight = null;
    return (...args) => {
      if (inFlight) return inFlight;
      let result;
      try {
        result = task(...args);
      } catch (error) {
        return Promise.reject(error);
      }
      let current;
      current = Promise.resolve(result).finally(() => {
        if (inFlight === current) inFlight = null;
      });
      inFlight = current;
      return current;
    };
  }

  return { orderedTeams, filterEntities, shouldRefreshForScopes, singleFlight };
});
