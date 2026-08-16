function mergeChampionRows(analyticsRows = [], championInfo = []) {
  const infoById = new Map(championInfo
    .filter((info) => info?.champion_id)
    .map((info) => [info.champion_id, info]));
  const rows = analyticsRows.map((row) => ({
    ...row,
    display_name: infoById.get(row.champion_id)?.display_name || row.display_name || row.champion_id,
  }));
  const known = new Set(rows.map((row) => row.champion_id));
  for (const info of championInfo) {
    if (!info?.champion_id || known.has(info.champion_id)) continue;
    known.add(info.champion_id);
    rows.push({
      champion_id: info.champion_id,
      display_name: info.display_name || info.champion_id,
      tier: "no_tier",
      overall: null,
      eligible: false,
      pick_count: 0,
      wins: 0,
      ban_count: 0,
      win_rate: null,
      pick_rate: null,
      ban_rate: null,
      average_dealt: null,
      average_taken: null,
      average_healing: null,
      patch_history: [],
      top_items: [],
      synergies: [],
      matchups: [],
      score: null,
      role_profile: {
        champion_id: info.champion_id,
        roles: ["top", "jungle", "mid", "bot", "support"].map((role) => ({ role, matches: 0, wins: 0, share: 0, win_rate: null })),
        primary_roles: ["top", "jungle", "mid", "bot", "support"],
        total_matches: 0,
        required_matches: 5,
        sufficient: false,
        used_patches: [],
      },
    });
  }
  return rows;
}

function activeChampionRows(rows = [], championInfo = []) {
  const active = new Set(championInfo.filter((info) => info?.champion_id).map((info) => info.champion_id));
  return rows.filter((row) => active.has(row.champion_id));
}

function championPositions(champion = {}) {
  const source = Array.isArray(champion.positions) && champion.positions.length
    ? champion.positions
    : champion.main_position ? [champion.main_position] : [];
  return [...new Set(source.filter(Boolean))].slice(0, 2);
}

function formatChampionPositions(champion, positionLabel, unknownLabel) {
  const positions = championPositions(champion);
  return positions.length
    ? positions.map((position) => positionLabel(position)).join(" · ")
    : unknownLabel;
}

if (typeof module !== "undefined" && module.exports) module.exports = {
  mergeChampionRows,
  activeChampionRows,
  championPositions,
  formatChampionPositions,
};
if (typeof globalThis !== "undefined") {
  globalThis.mergeChampionRows = mergeChampionRows;
  globalThis.activeChampionRows = activeChampionRows;
  globalThis.championPositions = championPositions;
  globalThis.formatChampionPositions = formatChampionPositions;
}
