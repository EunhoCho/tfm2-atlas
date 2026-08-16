const assert = require("node:assert/strict");
const test = require("node:test");

const { mergeChampionRows, activeChampionRows, formatChampionPositions } = require("../renderer/champions.js");

test("champions registered by another mod appear before they have match samples", () => {
  const analyticsRows = [{ champion_id: "archer", tier: "one", eligible: true, pick_count: 12 }];
  const championInfo = [
    { champion_id: "archer", display_name: "궁수", category: "Ranged" },
    { champion_id: "other_mod_dragon", display_name: "모드 드래곤", category: "Mage" },
  ];
  const rows = mergeChampionRows(analyticsRows, championInfo);

  assert.equal(rows.length, 2);
  assert.equal(rows[0].champion_id, "archer");
  assert.equal(rows[0].display_name, "궁수");
  assert.equal(rows[0].tier, "one");
  assert.deepEqual(rows[1], {
    champion_id: "other_mod_dragon",
    display_name: "모드 드래곤",
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
      champion_id: "other_mod_dragon",
      roles: ["top", "jungle", "mid", "bot", "support"].map((role) => ({ role, matches: 0, wins: 0, share: 0, win_rate: null })),
      primary_roles: ["top", "jungle", "mid", "bot", "support"],
      total_matches: 0,
      required_matches: 5,
      sufficient: false,
      used_patches: [],
    },
  });
});

test("sidebar contains only champions active in the current patched sheet", () => {
  const merged = mergeChampionRows(
    [
      { champion_id: "archer", tier: "one", pick_count: 12 },
      { champion_id: "removed_old_patch", tier: "two", pick_count: 30 },
    ],
    [{ champion_id: "archer", display_name: "궁수" }],
  );

  assert.deepEqual(activeChampionRows(merged, [{ champion_id: "archer" }]).map((row) => row.champion_id), ["archer"]);
});

test("game positions keep the displayed primary and secondary lane order", () => {
  const labels = { top: "탑", mid: "미드" };

  assert.equal(
    formatChampionPositions(
      { positions: ["top", "mid"], main_position: "top" },
      (position) => labels[position],
      "역할 구분 없음",
    ),
    "탑 · 미드",
  );
  assert.equal(
    formatChampionPositions(
      { positions: [], main_position: "mid" },
      (position) => labels[position],
      "역할 구분 없음",
    ),
    "미드",
  );
});
