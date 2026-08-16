//! Gameplay content vtables — what a mod's champions/actions/effects/
//! passives/items look like across the boundary.
//!
//! Each registered object is `{ userdata, vtable }`. The host owns the
//! object after registration and calls `destroy` (from the object's own
//! vtable) when it drops its wrapper, so allocator ownership never crosses
//! sides. `clone` returns fresh userdata sharing the same vtable.
//!
//! String getters return [`StrV1`] views borrowed from the object; they must
//! stay valid until that object's `destroy`. The mod-side wrapper guarantees
//! this by caching formatted strings inside the shim object.

use std::ffi::c_void;

use crate::{BuffV1, FrameVtableV1, InputTargetV1, SimVtableV1, StatV1, StrV1};

/// Per-callback simulation context handed to gameplay callbacks. Valid only
/// for the duration of the callback; never store it.
#[repr(C)]
pub struct SimCtxV1 {
    pub size: usize,
    pub sim: *const SimVtableV1,
    pub frame: *const FrameVtableV1,
    /// Opaque host state passed back to every sim/frame slot.
    pub state: *mut c_void,
}

/// A registered object: opaque mod-side state plus its function table.
macro_rules! reg_struct {
    ($(#[$m:meta])* $name:ident, $vtable:ident) => {
        $(#[$m])*
        #[repr(C)]
        #[derive(Clone, Copy, Debug)]
        pub struct $name {
            pub userdata: *mut c_void,
            pub vtable: *const $vtable,
        }

        impl $name {
            pub const NULL: Self =
                Self { userdata: std::ptr::null_mut(), vtable: std::ptr::null() };

            pub fn is_null(&self) -> bool {
                self.vtable.is_null()
            }
        }
    };
}

reg_struct!(ChampionRegV1, ChampionVtableV1);
reg_struct!(ActionRegV1, ActionVtableV1);
reg_struct!(EffectTypeRegV1, EffectTypeVtableV1);
reg_struct!(PassiveRegV1, PassiveVtableV1);
reg_struct!(ItemRegV1, ItemVtableV1);
reg_struct!(MatchHookRegV1, MatchHookVtableV1);

/// Custom match logic on top of an existing game mode ("mode variant").
/// Runs inside the deterministic simulation — implementations must be
/// deterministic. `check_match_end` returns true and writes `out_blue_win`
/// to end the match immediately with that winner.
#[repr(C)]
pub struct MatchHookVtableV1 {
    pub size: usize,
    pub destroy: Option<unsafe extern "C" fn(userdata: *mut c_void)>,
    pub on_match_start: Option<unsafe extern "C" fn(userdata: *mut c_void, sim: *mut SimCtxV1)>,
    pub on_match_tick:
        Option<unsafe extern "C" fn(userdata: *mut c_void, sim: *mut SimCtxV1, rng_seed: u64)>,
    pub check_match_end: Option<
        unsafe extern "C" fn(
            userdata: *mut c_void,
            sim: *mut SimCtxV1,
            out_blue_win: *mut bool,
        ) -> bool,
    >,
}

/// Named native effect referenced from `.data_champion` actions via
/// `DataEffectDef::Native { effect_ref }`.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct NativeEffectRegV1 {
    pub name: StrV1,
    pub effect: EffectTypeRegV1,
}

/// Effect produced by an action. `casting`/`target`/`attack_type` carry
/// [`crate::CastingTypeV1`]/[`crate::CastingTargetV1`]/[`crate::AttackTypeV1`] codes.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct EffectSpecV1 {
    pub range: u64,
    pub growth_range: u64,
    pub start_timing: usize,
    pub casting: u32,
    pub target: u32,
    pub attack_type: u32,
    pub effect: EffectTypeRegV1,
}

/// Champion definition table.
#[repr(C)]
pub struct ChampionVtableV1 {
    pub size: usize,
    pub destroy: Option<unsafe extern "C" fn(userdata: *mut c_void)>,
    pub id: Option<unsafe extern "C" fn(userdata: *const c_void) -> StrV1>,
    pub name: Option<unsafe extern "C" fn(userdata: *const c_void) -> StrV1>,
    /// Writes the icon (source, rect_tag) for a skill index. Returns false if
    /// the mod has no icon for that index (host falls back to defaults).
    pub skill_icon: Option<
        unsafe extern "C" fn(
            userdata: *const c_void,
            skill_index: usize,
            out_source: *mut StrV1,
            out_tag: *mut StrV1,
        ) -> bool,
    >,
    /// [`crate::ChampionCategoryV1`] code.
    pub category: Option<unsafe extern "C" fn(userdata: *const c_void) -> u32>,
    /// Writes up to `cap` [`crate::ChampionTagV1`] codes; returns the total count.
    pub tags:
        Option<unsafe extern "C" fn(userdata: *const c_void, out: *mut u32, cap: usize) -> usize>,
    pub stat: Option<unsafe extern "C" fn(userdata: *const c_void, out: *mut StatV1)>,
    pub growth: Option<unsafe extern "C" fn(userdata: *const c_void, out: *mut StatV1)>,
    /// Creates a NEW action object each call; ownership passes to the host.
    pub attack:
        Option<unsafe extern "C" fn(userdata: *const c_void, out: *mut ActionRegV1) -> bool>,
    pub skill: Option<unsafe extern "C" fn(userdata: *const c_void, out: *mut ActionRegV1) -> bool>,
    pub skill2:
        Option<unsafe extern "C" fn(userdata: *const c_void, out: *mut ActionRegV1) -> bool>,
    /// Returns false when the champion has no ult.
    pub ult: Option<unsafe extern "C" fn(userdata: *const c_void, out: *mut ActionRegV1) -> bool>,
    /// Returns false when the champion has no passive.
    pub passive:
        Option<unsafe extern "C" fn(userdata: *const c_void, out: *mut PassiveRegV1) -> bool>,
    // -- level 2 --
    /// Writes a per-lane suitability weight (0..=100) into `out[0..5]`, indexed
    /// by [`crate::LaneV1`] code, and returns true. The banpick model ships
    /// pretrained for the built-in roster, so a mod champion otherwise starts
    /// with a flat profile and reads as playable in every lane. The weights
    /// seed pseudo-observations at registration; real match results then take
    /// over. Returns false to keep the flat default.
    pub lane_prior:
        Option<unsafe extern "C" fn(userdata: *const c_void, out: *mut usize, cap: usize) -> bool>,
}

/// Action definition table.
#[repr(C)]
pub struct ActionVtableV1 {
    pub size: usize,
    pub destroy: Option<unsafe extern "C" fn(userdata: *mut c_void)>,
    /// New userdata sharing this vtable.
    pub clone: Option<unsafe extern "C" fn(userdata: *const c_void) -> *mut c_void>,
    pub action_name: Option<unsafe extern "C" fn(userdata: *const c_void) -> StrV1>,
    pub duration: Option<unsafe extern "C" fn(userdata: *const c_void) -> usize>,
    pub cancelable: Option<unsafe extern "C" fn(userdata: *const c_void) -> bool>,
    pub cooltime: Option<
        unsafe extern "C" fn(
            userdata: *const c_void,
            caster_stat: *const StatV1,
            caster_level: usize,
        ) -> usize,
    >,
    /// [`crate::CastingTargetV1`] code.
    pub casting_target: Option<unsafe extern "C" fn(userdata: *const c_void) -> u32>,
    /// Creates a NEW effect object inside `out`; returns false if the action
    /// has no effect. Ownership of the effect object passes to the host.
    pub effect:
        Option<unsafe extern "C" fn(userdata: *const c_void, out: *mut EffectSpecV1) -> bool>,
    pub cooltime_use_count:
        Option<unsafe extern "C" fn(userdata: *const c_void, caster_stat: *const StatV1) -> usize>,
    pub can_use_with_move: Option<unsafe extern "C" fn(userdata: *const c_void) -> bool>,
    pub description: Option<unsafe extern "C" fn(userdata: *const c_void) -> StrV1>,
}

/// Effect logic table.
#[repr(C)]
pub struct EffectTypeVtableV1 {
    pub size: usize,
    pub destroy: Option<unsafe extern "C" fn(userdata: *mut c_void)>,
    pub apply: Option<
        unsafe extern "C" fn(
            userdata: *mut c_void,
            sim: *mut SimCtxV1,
            rng_seed: u64,
            caster_id: usize,
            input: *const InputTargetV1,
        ),
    >,
    pub expected_damage: Option<
        unsafe extern "C" fn(
            userdata: *const c_void,
            caster_stat: *const StatV1,
            out_ad: *mut usize,
            out_ap: *mut usize,
        ),
    >,
    pub expected_heal:
        Option<unsafe extern "C" fn(userdata: *const c_void, caster_stat: *const StatV1) -> usize>,
    pub expected_shield:
        Option<unsafe extern "C" fn(userdata: *const c_void, caster_stat: *const StatV1) -> usize>,
    /// Returns false when the effect has no expected CC.
    pub expected_cc_time:
        Option<unsafe extern "C" fn(userdata: *const c_void, out: *mut usize) -> bool>,
    /// Returns false when the effect has no expected buff.
    pub expected_buff: Option<
        unsafe extern "C" fn(
            userdata: *const c_void,
            caster_stat: *const StatV1,
            out: *mut BuffV1,
        ) -> bool,
    >,
    /// Returns false when not a movement effect.
    pub expected_move_distance: Option<
        unsafe extern "C" fn(
            userdata: *const c_void,
            out_ticks: *mut usize,
            out_distance: *mut u64,
        ) -> bool,
    >,
    pub expected_rush_effect: Option<unsafe extern "C" fn(userdata: *const c_void) -> bool>,
    pub auto_target: Option<unsafe extern "C" fn(userdata: *const c_void) -> bool>,
    pub on_caster: Option<unsafe extern "C" fn(userdata: *const c_void) -> bool>,
    pub can_move: Option<unsafe extern "C" fn(userdata: *const c_void) -> bool>,
    /// Returns false when not a linear projectile.
    pub linear_move_speed:
        Option<unsafe extern "C" fn(userdata: *const c_void, out: *mut usize) -> bool>,
}

/// Passive callback table.
#[repr(C)]
pub struct PassiveVtableV1 {
    pub size: usize,
    pub destroy: Option<unsafe extern "C" fn(userdata: *mut c_void)>,
    pub clone: Option<unsafe extern "C" fn(userdata: *const c_void) -> *mut c_void>,
    pub on_spawn: Option<
        unsafe extern "C" fn(
            userdata: *mut c_void,
            sim: *mut SimCtxV1,
            player: usize,
            entity: usize,
        ),
    >,
    pub on_attack: Option<
        unsafe extern "C" fn(
            userdata: *mut c_void,
            sim: *mut SimCtxV1,
            player: usize,
            entity: usize,
            target: usize,
            damage: *mut usize,
        ),
    >,
    pub on_damaged: Option<
        unsafe extern "C" fn(
            userdata: *mut c_void,
            sim: *mut SimCtxV1,
            player: usize,
            entity: usize,
            attacker: usize,
            damage: usize,
        ),
    >,
    pub on_kill: Option<
        unsafe extern "C" fn(
            userdata: *mut c_void,
            sim: *mut SimCtxV1,
            player: usize,
            entity: usize,
        ),
    >,
    pub on_update: Option<
        unsafe extern "C" fn(
            userdata: *mut c_void,
            sim: *mut SimCtxV1,
            rng_seed: u64,
            player: usize,
            entity: usize,
        ),
    >,
    pub on_base_attack: Option<
        unsafe extern "C" fn(
            userdata: *mut c_void,
            sim: *mut SimCtxV1,
            rng_seed: u64,
            player: usize,
            entity: usize,
        ),
    >,
    pub on_assist: Option<
        unsafe extern "C" fn(
            userdata: *mut c_void,
            sim: *mut SimCtxV1,
            player: usize,
            entity: usize,
        ),
    >,
    pub on_dead:
        Option<unsafe extern "C" fn(userdata: *mut c_void, sim: *mut SimCtxV1, player: usize)>,
    // -- level 2 --
    /// Same as `on_kill` plus the killed entity. Preferred when present;
    /// `entity` stays the passive owner's own champion.
    pub on_kill_ex: Option<
        unsafe extern "C" fn(
            userdata: *mut c_void,
            sim: *mut SimCtxV1,
            player: usize,
            entity: usize,
            victim: usize,
        ),
    >,
}

/// Item definition + callback table.
#[repr(C)]
pub struct ItemVtableV1 {
    pub size: usize,
    pub destroy: Option<unsafe extern "C" fn(userdata: *mut c_void)>,
    pub clone: Option<unsafe extern "C" fn(userdata: *const c_void) -> *mut c_void>,
    pub key: Option<unsafe extern "C" fn(userdata: *const c_void) -> StrV1>,
    pub icon: Option<unsafe extern "C" fn(userdata: *const c_void) -> StrV1>,
    pub price: Option<unsafe extern "C" fn(userdata: *const c_void) -> usize>,
    pub tier: Option<unsafe extern "C" fn(userdata: *const c_void) -> usize>,
    pub stat: Option<unsafe extern "C" fn(userdata: *const c_void, out: *mut BuffV1)>,
    /// Writes up to `cap` item-key strings; returns the total count. Returned
    /// views stay valid until this object's `destroy`.
    pub next_tier:
        Option<unsafe extern "C" fn(userdata: *const c_void, out: *mut StrV1, cap: usize) -> usize>,
    pub previous_tier:
        Option<unsafe extern "C" fn(userdata: *const c_void, out: *mut StrV1, cap: usize) -> usize>,
    /// Writes up to `cap` [`crate::ItemTagV1`] codes; returns the total count.
    pub tags:
        Option<unsafe extern "C" fn(userdata: *const c_void, out: *mut u32, cap: usize) -> usize>,
    /// [`crate::ItemCategoryV1`] code.
    pub category: Option<unsafe extern "C" fn(userdata: *const c_void) -> u32>,
    /// `damage_type` carries a [`crate::DamageTypeV1`] code.
    pub on_attack: Option<
        unsafe extern "C" fn(
            userdata: *mut c_void,
            sim: *mut SimCtxV1,
            caster: usize,
            target: usize,
            damage: *mut usize,
            damage_type: u32,
        ),
    >,
    pub update: Option<
        unsafe extern "C" fn(
            userdata: *mut c_void,
            sim: *mut SimCtxV1,
            rng_seed: u64,
            player: usize,
        ),
    >,
    pub on_spawn:
        Option<unsafe extern "C" fn(userdata: *mut c_void, sim: *mut SimCtxV1, player: usize)>,
    /// `has_caster == false` means a caster-less heal (e.g. regen).
    pub on_healed: Option<
        unsafe extern "C" fn(
            userdata: *mut c_void,
            sim: *mut SimCtxV1,
            caster: usize,
            has_caster: bool,
            entity: usize,
            heal: usize,
        ),
    >,
    pub on_damaged: Option<
        unsafe extern "C" fn(
            userdata: *mut c_void,
            sim: *mut SimCtxV1,
            player: usize,
            entity: usize,
            attacker: usize,
            damage: usize,
        ),
    >,
    pub on_kill: Option<
        unsafe extern "C" fn(
            userdata: *mut c_void,
            sim: *mut SimCtxV1,
            rng_seed: u64,
            player: usize,
            entity: usize,
        ),
    >,
    pub on_skill_hit: Option<
        unsafe extern "C" fn(
            userdata: *mut c_void,
            sim: *mut SimCtxV1,
            rng_seed: u64,
            caster: usize,
            target: usize,
        ),
    >,
    // -- level 2 --
    /// Same as `on_attack` plus the [`crate::AttackTypeV1`] code and crit flag.
    /// Preferred over `on_attack` when present.
    pub on_attack_ex: Option<
        unsafe extern "C" fn(
            userdata: *mut c_void,
            sim: *mut SimCtxV1,
            caster: usize,
            target: usize,
            damage: *mut usize,
            damage_type: u32,
            attack_type: u32,
            is_crit: bool,
        ),
    >,
    /// Same as `on_damaged` plus damage/attack type codes and the crit flag.
    pub on_damaged_ex: Option<
        unsafe extern "C" fn(
            userdata: *mut c_void,
            sim: *mut SimCtxV1,
            player: usize,
            entity: usize,
            attacker: usize,
            damage: usize,
            damage_type: u32,
            attack_type: u32,
            is_crit: bool,
        ),
    >,
    /// Same as `on_kill` plus the killed entity. `entity` stays the item
    /// owner's own champion.
    pub on_kill_ex: Option<
        unsafe extern "C" fn(
            userdata: *mut c_void,
            sim: *mut SimCtxV1,
            rng_seed: u64,
            player: usize,
            entity: usize,
            victim: usize,
        ),
    >,
    /// Same as `on_skill_hit` plus `is_ally`, which is true for ally-targeted
    /// skills (shields, heals, buffs). `on_skill_hit` only sees enemy hits.
    pub on_skill_hit_ex: Option<
        unsafe extern "C" fn(
            userdata: *mut c_void,
            sim: *mut SimCtxV1,
            rng_seed: u64,
            caster: usize,
            target: usize,
            is_ally: bool,
        ),
    >,
    /// Auto-attack only, unlike `on_attack` which also fires for skills.
    pub on_base_attack: Option<
        unsafe extern "C" fn(
            userdata: *mut c_void,
            sim: *mut SimCtxV1,
            rng_seed: u64,
            player: usize,
            entity: usize,
        ),
    >,
    pub on_assist: Option<
        unsafe extern "C" fn(
            userdata: *mut c_void,
            sim: *mut SimCtxV1,
            player: usize,
            entity: usize,
        ),
    >,
    pub on_dead:
        Option<unsafe extern "C" fn(userdata: *mut c_void, sim: *mut SimCtxV1, player: usize)>,
    /// Fires when the owner is hit by hard crowd control; `caster` applied it.
    pub on_cc: Option<
        unsafe extern "C" fn(
            userdata: *mut c_void,
            sim: *mut SimCtxV1,
            rng_seed: u64,
            player: usize,
            caster: usize,
        ),
    >,
    /// Fires on the item being replaced by an upgrade. The returned value is
    /// handed to the successor's `on_upgraded_from`, so stack counts and other
    /// accumulated state can carry over.
    pub on_upgrade: Option<unsafe extern "C" fn(userdata: *mut c_void, next_key: StrV1) -> u64>,
    /// Fires on the successor item with whatever its predecessor's
    /// `on_upgrade` returned.
    pub on_upgraded_from:
        Option<unsafe extern "C" fn(userdata: *mut c_void, prev_key: StrV1, carry: u64)>,
}
