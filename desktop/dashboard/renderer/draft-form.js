(function (root, factory) {
  const api = factory();
  if (typeof module === "object" && module.exports) module.exports = api;
  else root.DraftForm = api;
}(typeof globalThis !== "undefined" ? globalThis : this, () => {
  function oppositeSide(side) {
    return side === "red" ? "blue" : "red";
  }

  function modeTarget(mode, playerSide) {
    const own = String(mode || "").startsWith("our_");
    return {
      side: own ? playerSide : oppositeSide(playerSide),
      phase: String(mode || "").endsWith("_pick") ? "pick" : "ban",
    };
  }

  function currentSet(draft = {}) {
    return (draft.sets || []).find((set) => Number(set.set_number) === Number(draft.current_set))
      || { set_number: draft.current_set || 1, our_side: "blue", blue_bans: [], red_bans: [], blue_picks: [], red_picks: [] };
  }

  function currentSide(draft = {}) {
    return currentSet(draft).our_side === "red" ? "red" : "blue";
  }

  function selectedIds(draft) {
    const set = currentSet(draft);
    return new Set([
      ...(set.blue_bans || []), ...(set.red_bans || []),
      ...(set.blue_picks || []), ...(set.red_picks || []),
    ]);
  }

  function championAvailability(draft, mode, championId) {
    const target = modeTarget(mode, currentSide(draft));
    if (selectedIds(draft).has(championId)) return { disabled: true, reason: "selected" };
    const excluded = target.side === "blue" ? draft.excluded_blue : draft.excluded_red;
    if ((excluded || []).includes(championId)) return { disabled: true, reason: "fearless" };
    const set = currentSet(draft);
    const slot = set[`${target.side}_${target.phase}s`] || [];
    const capacity = target.phase === "ban" ? 3 : 5;
    if (slot.length >= capacity) return { disabled: true, reason: "full" };
    return { disabled: false, reason: null };
  }

  return { championAvailability, currentSet, currentSide, modeTarget, oppositeSide, selectedIds };
}));
