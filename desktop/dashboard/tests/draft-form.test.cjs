const assert = require("node:assert/strict");
const test = require("node:test");

const DraftForm = require("../renderer/draft-form.js");

const draft = {
  current_set: 2,
  excluded_blue: ["blue_old"],
  excluded_red: ["red_old"],
  sets: [
    { set_number: 1, our_side: "blue", blue_bans: [], red_bans: [], blue_picks: ["blue_old"], red_picks: ["red_old"] },
    { set_number: 2, our_side: "red", blue_bans: ["taken"], red_bans: [], blue_picks: [], red_picks: [] },
  ],
};

test("manual modes map our and enemy actions onto physical blue and red sides", () => {
  assert.deepEqual(DraftForm.modeTarget("our_pick", DraftForm.currentSide(draft)), { side: "red", phase: "pick" });
  assert.deepEqual(DraftForm.modeTarget("enemy_ban", DraftForm.currentSide(draft)), { side: "blue", phase: "ban" });
});

test("champion availability follows the active mode and fearless exclusions", () => {
  assert.equal(DraftForm.championAvailability(draft, "our_pick", "red_old").reason, "fearless");
  assert.equal(DraftForm.championAvailability(draft, "enemy_pick", "blue_old").reason, "fearless");
  assert.equal(DraftForm.championAvailability(draft, "our_pick", "taken").reason, "selected");
  assert.equal(DraftForm.championAvailability(draft, "our_pick", "fresh").disabled, false);
});

test("the selected set is resolved by set number rather than array position", () => {
  const shuffled = { ...draft, sets: [draft.sets[1], draft.sets[0]] };
  assert.equal(DraftForm.currentSet(shuffled).set_number, 2);
  assert.equal(DraftForm.currentSide(shuffled), "red");
});

test("each set restores its own player side", () => {
  assert.equal(DraftForm.currentSide({ ...draft, current_set: 1 }), "blue");
  assert.equal(DraftForm.currentSide({ ...draft, current_set: 2 }), "red");
});
