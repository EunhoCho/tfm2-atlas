//! Modder-facing gameplay traits. Implement these in plain Rust; the
//! `StableMod` builder converts them into boundary vtables automatically —
//! no FFI in mod code.
//!
//! These traits never cross the DLL boundary themselves, so they may evolve
//! freely with the wrapper crate.

use crate::{
    AttackTypeV1, BuffV1, CastingTargetV1, CastingTypeV1, ChampionCategoryV1, ChampionTagV1,
    CommandResultV1, InputTargetV1, InputV1, ItemCategoryV1, ItemTagV1, StableCommand,
    StableServerCtx, StableSim, StatV1,
};

/// Champion definition, mirroring the legacy `ModChampionInfo` surface.
pub trait StableChampion: Send + Sync + 'static {
    /// Unique string identifier, or an existing base champion id to rework it.
    fn id(&self) -> String;

    /// Display name (i18n key recommended). Defaults to the id.
    fn name(&self) -> String {
        self.id()
    }

    /// Skill icon (source, rect_tag) for skill index 0..=3.
    fn skill_icon(&self, skill_index: usize) -> (String, String) {
        (
            "asset/base/aseprite_resources/UI_aseprite/skill_icon".to_string(),
            format!("{}_{}", self.id(), skill_index),
        )
    }

    fn category(&self) -> ChampionCategoryV1;
    fn tags(&self) -> Vec<ChampionTagV1>;

    /// Base stats at level 1.
    fn stat(&self) -> StatV1;
    /// Per-level stat growth.
    fn growth(&self) -> StatV1;

    fn attack(&self) -> Box<dyn StableAction>;
    fn skill(&self) -> Box<dyn StableAction>;
    fn skill2(&self) -> Box<dyn StableAction>;
    fn ult(&self) -> Option<Box<dyn StableAction>> {
        None
    }

    fn passive(&self) -> Option<Box<dyn StablePassive>> {
        None
    }

    /// Per-lane suitability weight (0..=100) for each [`LaneV1`], used to seed
    /// the banpick model. The model ships pretrained for the built-in roster,
    /// so a mod champion without a prior starts flat and reads as playable in
    /// every lane. `None` keeps that flat default; real match results
    /// gradually override whatever is seeded here.
    fn lane_prior(&self) -> Option<Vec<(crate::LaneV1, usize)>> {
        None
    }
}

/// Effect produced by an action.
pub struct StableEffectSpec {
    pub range: u64,
    pub growth_range: u64,
    /// Tick offset from action start when the effect triggers.
    pub start_timing: usize,
    pub casting: CastingTypeV1,
    pub target: CastingTargetV1,
    pub attack_type: AttackTypeV1,
    pub effect: Box<dyn StableEffectType>,
}

/// Action definition, mirroring the legacy `ModAction` surface.
pub trait StableAction: Send + Sync + 'static {
    fn clone_box(&self) -> Box<dyn StableAction>;

    /// Animation tag name ("attack", "skill", "skill2", "ult").
    fn action_name(&self) -> String {
        "attack".to_string()
    }

    /// Total animation duration in ticks.
    fn duration(&self) -> usize;

    fn cancelable(&self) -> bool {
        false
    }

    /// Cooldown in ticks given the caster's current stat and level.
    fn cooltime(&self, caster_stat: &StatV1, caster_level: usize) -> usize;

    fn casting_target(&self) -> CastingTargetV1;

    fn effect(&self) -> Option<StableEffectSpec>;

    fn cooltime_use_count(&self, _caster_stat: &StatV1) -> usize {
        1
    }

    fn can_use_with_move(&self) -> bool {
        false
    }

    /// User-visible description (i18n key or formatted string).
    fn description(&self) -> String {
        String::new()
    }
}

/// Effect logic, mirroring the legacy `ModEffectType` surface.
pub trait StableEffectType: Send + Sync + 'static {
    /// Apply the effect. `rng_seed` is deterministic; derive an RNG from it
    /// when randomness is needed.
    fn apply(&self, sim: &mut StableSim<'_>, rng_seed: u64, caster_id: usize, input: InputTargetV1);

    /// Expected (AD, AP) raw damage for AI evaluation.
    fn expected_damage(&self, _caster_stat: &StatV1) -> (usize, usize) {
        (0, 0)
    }

    fn expected_heal(&self, _caster_stat: &StatV1) -> usize {
        0
    }

    fn expected_shield(&self, _caster_stat: &StatV1) -> usize {
        0
    }

    fn expected_cc_time(&self) -> Option<usize> {
        None
    }

    fn expected_buff(&self, _caster_stat: &StatV1) -> Option<BuffV1> {
        None
    }

    /// Expected (ticks, distance) for movement effects.
    fn expected_move_distance(&self) -> Option<(usize, u64)> {
        None
    }

    fn expected_rush_effect(&self) -> bool {
        false
    }

    fn auto_target(&self) -> bool {
        false
    }

    fn on_caster(&self) -> bool {
        false
    }

    fn can_move(&self) -> bool {
        false
    }

    fn linear_move_speed(&self) -> Option<usize> {
        None
    }
}

/// Passive ability reacting to game events, mirroring `ModPassive`.
pub trait StablePassive: Send + Sync + 'static {
    fn clone_box(&self) -> Box<dyn StablePassive>;

    fn on_spawn(&mut self, _sim: &mut StableSim<'_>, _player: usize, _entity: usize) {}

    fn on_attack(
        &mut self,
        _sim: &mut StableSim<'_>,
        _player: usize,
        _entity: usize,
        _target: usize,
        _damage: &mut usize,
    ) {
    }

    fn on_damaged(
        &mut self,
        _sim: &mut StableSim<'_>,
        _player: usize,
        _entity: usize,
        _attacker: usize,
        _damage: usize,
    ) {
    }

    /// `_entity` is the owner's own champion; `_victim` is what was killed.
    fn on_kill(
        &mut self,
        _sim: &mut StableSim<'_>,
        _player: usize,
        _entity: usize,
        _victim: usize,
    ) {
    }

    fn on_update(
        &mut self,
        _sim: &mut StableSim<'_>,
        _rng_seed: u64,
        _player: usize,
        _entity: usize,
    ) {
    }

    fn on_base_attack(
        &mut self,
        _sim: &mut StableSim<'_>,
        _rng_seed: u64,
        _player: usize,
        _entity: usize,
    ) {
    }

    fn on_assist(&mut self, _sim: &mut StableSim<'_>, _player: usize, _entity: usize) {}

    fn on_dead(&mut self, _sim: &mut StableSim<'_>, _player: usize) {}
}

/// Client lifecycle hooks, mirroring the legacy `ModExtension`. UI mutation
/// and save/net access go through the ctx wrapper; inside UI click handlers
/// only ui/asset calls are live.
pub trait StableExtension: Send + Sync + 'static {
    fn on_init(&self, _ctx: &mut crate::StableClient<'_>) {}
    fn pre_update(&self, _ctx: &mut crate::StableClient<'_>, _dt_micros: u64) {}
    fn post_update(&self, _ctx: &mut crate::StableClient<'_>, _dt_micros: u64) {}
    fn on_end(&self) {}

    /// Render hooks, mirroring legacy `pre_render`/`post_render`. The ctx is
    /// read-only here (UI/save/net mutation calls fail gracefully); push draw
    /// commands through the `draw_*` methods, which are live only in these
    /// two hooks. `pre_render` runs before the frame's own commands (drawn
    /// under them), `post_render` after (drawn over them, z permitting).
    fn pre_render(&self, _ctx: &mut crate::StableClient<'_>) {}
    fn post_render(&self, _ctx: &mut crate::StableClient<'_>) {}
}

/// Authoritative management-server hooks, mirroring `ModServerExtension`.
/// Server-side save writes and client-event emission go through the ctx;
/// direct Database access is not part of the stable contract.
pub trait StableServerExtension: Send + Sync + 'static {
    fn on_server_start(&self, _ctx: &mut StableServerCtx<'_>) {}
    fn before_management_tick(&self, _ctx: &mut StableServerCtx<'_>) {}
    fn after_management_tick(&self, _ctx: &mut StableServerCtx<'_>) {}

    fn handle_command(
        &self,
        _ctx: &mut StableServerCtx<'_>,
        _command: &StableCommand<'_>,
    ) -> CommandResultV1 {
        CommandResultV1::Pass
    }
}

/// Custom match logic on top of an existing game mode — the "mode variant"
/// hook. Runs inside the deterministic simulation: keep it deterministic
/// (derive randomness only from `rng_seed`), the same contract mod
/// champions/effects live under.
pub trait StableMatchHook: Send + Sync + 'static {
    /// After players spawned, before the first tick completes.
    fn on_match_start(&self, _sim: &mut StableSim<'_>) {}

    /// Every tick, after the core world tick and input phase.
    fn on_match_tick(&self, _sim: &mut StableSim<'_>, _rng_seed: u64) {}

    /// `Some(true)` ends the match now with a blue win, `Some(false)` red.
    fn check_match_end(&self, _sim: &mut StableSim<'_>) -> Option<bool> {
        None
    }
}

/// Adjusts the freshly built map of a match (tower/camp/lane/nexus/fountain
/// placement, wall/bush grids) through a JSON document view. Runs at match
/// creation for every game mode; must be deterministic.
///
/// Note: offline-baked pathfinding/visibility stays based on the original
/// geometry, so large wall/bush rework has limits — object placement
/// (towers, camps, lanes, nexus, fountains) is fully live.
pub trait StableMapCustomizer: Send + Sync + 'static {
    fn customize(&self, mode: Option<crate::GameModeKindV1>, doc: &mut crate::StableJsonDoc<'_>);
}

/// Draft (ban/pick) score hook, mirroring `ModDraftScoreHook`.
pub trait StableDraftHook: Send + Sync + 'static {
    fn id(&self) -> String;

    fn priority(&self) -> i32 {
        0
    }

    fn score_ban(
        &self,
        _ctx: &crate::StableDraftContext<'_>,
        _candidate: usize,
        _base_score: f32,
    ) -> crate::StableDraftDecision {
        crate::StableDraftDecision::Pass
    }

    fn score_pick(
        &self,
        _ctx: &crate::StableDraftContext<'_>,
        _candidate: usize,
        _base_score: f32,
    ) -> crate::StableDraftDecision {
        crate::StableDraftDecision::Pass
    }

    /// Full override: return the exact champion id to ban. Ignored when the
    /// id is not among the current candidates; of multiple hooks the last
    /// deciding one wins.
    fn decide_ban(&self, _ctx: &crate::StableDraftContext<'_>) -> Option<usize> {
        None
    }

    /// Full override for picks, same rules as `decide_ban`.
    fn decide_pick(&self, _ctx: &crate::StableDraftContext<'_>) -> Option<usize> {
        None
    }
}

/// Per-tick player input AI, mirroring `ModPlayerInputAi`. Return `None`
/// from `think` to keep the built-in input, or `Some(input)` to replace it.
pub trait StablePlayerAi: Send + Sync + 'static {
    fn clone_box(&self) -> Box<dyn StablePlayerAi>;

    fn id(&self) -> String;

    fn priority(&self) -> i32 {
        0
    }

    fn matches(&self, _init: &crate::StableAiInit) -> bool {
        true
    }

    fn think(
        &mut self,
        ctx: &mut crate::StableAiContext<'_>,
        base_input: Option<InputV1>,
    ) -> Option<InputV1>;
}

/// Item definition + callbacks, mirroring `ModItemInfo`.
pub trait StableItem: Send + Sync + 'static {
    fn clone_box(&self) -> Box<dyn StableItem>;

    fn key(&self) -> String;

    /// Icon tag in the item icon sheet. Defaults to the key.
    fn icon(&self) -> String {
        self.key()
    }

    fn price(&self) -> usize;
    fn tier(&self) -> usize;

    /// Stat modifiers granted by owning this item.
    fn stat(&self) -> BuffV1;

    fn next_tier(&self) -> Vec<String> {
        Vec::new()
    }

    fn previous_tier(&self) -> Vec<String> {
        Vec::new()
    }

    fn tags(&self) -> Vec<ItemTagV1> {
        Vec::new()
    }

    fn category(&self) -> ItemCategoryV1 {
        ItemCategoryV1::Ad
    }

    /// Fires for every hit the owner lands — auto-attacks *and* skills. Use
    /// [`StableItem::on_base_attack`] for auto-attack-only effects.
    fn on_attack(
        &mut self,
        _sim: &mut StableSim<'_>,
        _caster: usize,
        _target: usize,
        _damage: &mut usize,
        _damage_type: crate::DamageTypeV1,
        _attack_type: AttackTypeV1,
        _is_crit: bool,
    ) {
    }

    /// Auto-attack only.
    fn on_base_attack(
        &mut self,
        _sim: &mut StableSim<'_>,
        _rng_seed: u64,
        _player: usize,
        _entity: usize,
    ) {
    }

    fn update(&mut self, _sim: &mut StableSim<'_>, _rng_seed: u64, _player: usize) {}

    fn on_spawn(&mut self, _sim: &mut StableSim<'_>, _player: usize) {}

    fn on_healed(
        &mut self,
        _sim: &mut StableSim<'_>,
        _caster: Option<usize>,
        _entity: usize,
        _heal: usize,
    ) {
    }

    fn on_damaged(
        &mut self,
        _sim: &mut StableSim<'_>,
        _player: usize,
        _entity: usize,
        _attacker: usize,
        _damage: usize,
        _damage_type: crate::DamageTypeV1,
        _attack_type: AttackTypeV1,
        _is_crit: bool,
    ) {
    }

    /// `_entity` is the owner's own champion; `_victim` is what was killed.
    fn on_kill(
        &mut self,
        _sim: &mut StableSim<'_>,
        _rng_seed: u64,
        _player: usize,
        _entity: usize,
        _victim: usize,
    ) {
    }

    fn on_assist(&mut self, _sim: &mut StableSim<'_>, _player: usize, _entity: usize) {}

    fn on_dead(&mut self, _sim: &mut StableSim<'_>, _player: usize) {}

    /// Fires when the owner is hit by hard crowd control; `_caster` applied it.
    fn on_cc(&mut self, _sim: &mut StableSim<'_>, _rng_seed: u64, _player: usize, _caster: usize) {}

    /// `_is_ally` is true for ally-targeted skills (shields, heals, buffs).
    fn on_skill_hit(
        &mut self,
        _sim: &mut StableSim<'_>,
        _rng_seed: u64,
        _caster: usize,
        _target: usize,
        _is_ally: bool,
    ) {
    }

    /// Fires on this item as it is replaced by an upgrade. Whatever is
    /// returned is handed to the successor's [`StableItem::on_upgraded_from`],
    /// so stack counts and other accumulated state survive the upgrade.
    fn on_upgrade(&mut self, _next_key: &str) -> u64 {
        0
    }

    /// Fires on the successor with the predecessor's `on_upgrade` value.
    fn on_upgraded_from(&mut self, _prev_key: &str, _carry: u64) {}
}

/// Item-build hook, the typed way to steer which final items the AI targets.
/// Runs once per player when a match's builds are decided.
pub trait StableItemBuildHook: Send + Sync + 'static {
    fn id(&self) -> String;

    fn priority(&self) -> i32 {
        0
    }

    /// Adjusts the engine's score for one candidate final item.
    fn score_item(
        &self,
        _ctx: &crate::StableItemBuildContext<'_>,
        _candidate: usize,
        _base_score: f32,
    ) -> crate::StableDraftDecision {
        crate::StableDraftDecision::Pass
    }

    /// Full override: return the exact item indices to build, or an empty
    /// vec to keep the engine's build. Indices outside the item list are
    /// dropped; of multiple hooks the highest priority that decides wins.
    fn decide_build(&self, _ctx: &crate::StableItemBuildContext<'_>) -> Vec<usize> {
        Vec::new()
    }
}
