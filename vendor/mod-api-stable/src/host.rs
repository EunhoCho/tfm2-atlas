//! Host-side API surface handed to mods: `HostApiV1` plus per-domain
//! function tables.
//!
//! Every table starts with `size` (filled by the host with
//! `size_of::<Table>()` of the host's build). Mod-side wrappers must check a
//! slot's offset against that size before touching it, which is what makes a
//! newer mod degrade gracefully on an older host. All tables are append-only.
//!
//! M0 ships the table skeletons only (`size` header, no slots yet); slots
//! land per milestone (M1 sim/gameplay, M2 save/net/service, M3 ui/scene/
//! asset, M4 data). See docs/mod_stable_abi_design.md §6.

use std::ffi::c_void;

use crate::{
    BuffV1, CcV1, EntityHandleV1, KillLogV1, PlayerHandleV1, ProjectileHandleV1, ProjectileInfoV1,
    StatV1,
};

/// Game version reported to mods. Frozen value type.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GameVersionV1 {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

pub const LOG_LEVEL_ERROR: u32 = 1;
pub const LOG_LEVEL_WARN: u32 = 2;
pub const LOG_LEVEL_INFO: u32 = 3;
pub const LOG_LEVEL_DEBUG: u32 = 4;

/// Simulation queries/actions during a running match. Every slot takes the
/// opaque `state` from the enclosing [`crate::SimCtxV1`]. Entity/player/
/// projectile handles are ids encoded as `id + 1` (0 = null); all slots are
/// safe to call with invalid handles.
#[repr(C)]
pub struct SimVtableV1 {
    pub size: usize,
    // -- game state --
    pub tick: Option<unsafe extern "C" fn(state: *const c_void) -> usize>,
    pub seed: Option<unsafe extern "C" fn(state: *const c_void) -> u64>,
    pub score_diff: Option<unsafe extern "C" fn(state: *const c_void, team: usize) -> i32>,
    pub is_end: Option<unsafe extern "C" fn(state: *const c_void) -> bool>,
    // -- entity access --
    pub entity_count: Option<unsafe extern "C" fn(state: *const c_void) -> usize>,
    pub entity_at:
        Option<unsafe extern "C" fn(state: *const c_void, index: usize) -> EntityHandleV1>,
    pub entity_is_valid:
        Option<unsafe extern "C" fn(state: *const c_void, handle: EntityHandleV1) -> bool>,
    pub entity_stat: Option<
        unsafe extern "C" fn(
            state: *const c_void,
            handle: EntityHandleV1,
            out: *mut StatV1,
        ) -> bool,
    >,
    pub entity_pos: Option<
        unsafe extern "C" fn(
            state: *const c_void,
            handle: EntityHandleV1,
            out_x: *mut u64,
            out_y: *mut u64,
        ) -> bool,
    >,
    pub entity_hp: Option<
        unsafe extern "C" fn(
            state: *const c_void,
            handle: EntityHandleV1,
            out_current: *mut usize,
            out_max: *mut usize,
        ) -> bool,
    >,
    pub entity_team:
        Option<unsafe extern "C" fn(state: *const c_void, handle: EntityHandleV1) -> usize>,
    pub entity_level:
        Option<unsafe extern "C" fn(state: *const c_void, handle: EntityHandleV1) -> usize>,
    pub entity_is_alive:
        Option<unsafe extern "C" fn(state: *const c_void, handle: EntityHandleV1) -> bool>,
    pub entity_is_champion:
        Option<unsafe extern "C" fn(state: *const c_void, handle: EntityHandleV1) -> bool>,
    pub entity_is_tower:
        Option<unsafe extern "C" fn(state: *const c_void, handle: EntityHandleV1) -> bool>,
    pub entity_is_minion:
        Option<unsafe extern "C" fn(state: *const c_void, handle: EntityHandleV1) -> bool>,
    pub entity_shield:
        Option<unsafe extern "C" fn(state: *const c_void, handle: EntityHandleV1) -> usize>,
    pub entity_radius:
        Option<unsafe extern "C" fn(state: *const c_void, handle: EntityHandleV1) -> usize>,
    pub entity_is_targetable:
        Option<unsafe extern "C" fn(state: *const c_void, handle: EntityHandleV1) -> bool>,
    pub entity_buff_count:
        Option<unsafe extern "C" fn(state: *const c_void, handle: EntityHandleV1) -> usize>,
    pub entity_buff_at: Option<
        unsafe extern "C" fn(
            state: *const c_void,
            handle: EntityHandleV1,
            index: usize,
            out: *mut BuffV1,
        ) -> bool,
    >,
    pub entity_cc_count:
        Option<unsafe extern "C" fn(state: *const c_void, handle: EntityHandleV1) -> usize>,
    pub entity_cc_at: Option<
        unsafe extern "C" fn(
            state: *const c_void,
            handle: EntityHandleV1,
            index: usize,
            out: *mut CcV1,
        ) -> bool,
    >,
    // -- player access --
    pub player_count: Option<unsafe extern "C" fn(state: *const c_void) -> usize>,
    pub player_at:
        Option<unsafe extern "C" fn(state: *const c_void, index: usize) -> PlayerHandleV1>,
    pub player_is_valid:
        Option<unsafe extern "C" fn(state: *const c_void, handle: PlayerHandleV1) -> bool>,
    pub player_champion: Option<
        unsafe extern "C" fn(state: *const c_void, handle: PlayerHandleV1) -> EntityHandleV1,
    >,
    pub player_level:
        Option<unsafe extern "C" fn(state: *const c_void, handle: PlayerHandleV1) -> usize>,
    pub player_gold:
        Option<unsafe extern "C" fn(state: *const c_void, handle: PlayerHandleV1) -> usize>,
    /// [`crate::LaneV1`] code of the player's assigned lane.
    pub player_lane:
        Option<unsafe extern "C" fn(state: *const c_void, handle: PlayerHandleV1) -> u32>,
    pub player_team:
        Option<unsafe extern "C" fn(state: *const c_void, handle: PlayerHandleV1) -> usize>,
    pub player_is_alive:
        Option<unsafe extern "C" fn(state: *const c_void, handle: PlayerHandleV1) -> bool>,
    pub player_respawn_time:
        Option<unsafe extern "C" fn(state: *const c_void, handle: PlayerHandleV1) -> usize>,
    pub player_kills:
        Option<unsafe extern "C" fn(state: *const c_void, handle: PlayerHandleV1) -> usize>,
    pub player_deaths:
        Option<unsafe extern "C" fn(state: *const c_void, handle: PlayerHandleV1) -> usize>,
    pub player_assists:
        Option<unsafe extern "C" fn(state: *const c_void, handle: PlayerHandleV1) -> usize>,
    pub player_cs:
        Option<unsafe extern "C" fn(state: *const c_void, handle: PlayerHandleV1) -> usize>,
    // -- combat actions --
    /// `attack_type` carries a [`crate::AttackTypeV1`] code.
    pub deal_damage: Option<
        unsafe extern "C" fn(
            state: *mut c_void,
            attacker: usize,
            target: usize,
            ad: usize,
            ap: usize,
            attack_type: u32,
        ),
    >,
    pub heal: Option<
        unsafe extern "C" fn(state: *mut c_void, caster: usize, target: usize, amount: usize),
    >,
    pub add_buff:
        Option<unsafe extern "C" fn(state: *mut c_void, target: usize, buff: *const BuffV1)>,
    pub apply_cc: Option<unsafe extern "C" fn(state: *mut c_void, target: usize, cc: *const CcV1)>,
    // -- utility --
    pub distance_sq:
        Option<unsafe extern "C" fn(state: *const c_void, id1: usize, id2: usize) -> u64>,
    pub is_visible:
        Option<unsafe extern "C" fn(state: *const c_void, team: usize, id: usize) -> bool>,
    // -- game structure queries --
    pub champion_count: Option<unsafe extern "C" fn(state: *const c_void) -> usize>,
    pub champion_id_at: Option<unsafe extern "C" fn(state: *const c_void, index: usize) -> usize>,
    pub tower_count: Option<unsafe extern "C" fn(state: *const c_void) -> usize>,
    pub tower_id_at: Option<unsafe extern "C" fn(state: *const c_void, index: usize) -> usize>,
    pub kill_log_count: Option<unsafe extern "C" fn(state: *const c_void) -> usize>,
    pub kill_log_at: Option<
        unsafe extern "C" fn(state: *const c_void, index: usize, out: *mut KillLogV1) -> bool,
    >,
    // -- projectiles --
    pub projectile_count: Option<unsafe extern "C" fn(state: *const c_void) -> usize>,
    pub projectile_at:
        Option<unsafe extern "C" fn(state: *const c_void, index: usize) -> ProjectileHandleV1>,
    pub projectile_is_valid:
        Option<unsafe extern "C" fn(state: *const c_void, handle: ProjectileHandleV1) -> bool>,
    pub projectile_info: Option<
        unsafe extern "C" fn(
            state: *const c_void,
            handle: ProjectileHandleV1,
            out: *mut ProjectileInfoV1,
        ) -> bool,
    >,
    // -- direct mutation (custom-mode building blocks) --
    /// Sets current HP directly. Bypasses damage attribution: a kill that
    /// should appear in logs/stats must go through `deal_damage` instead.
    pub entity_set_hp:
        Option<unsafe extern "C" fn(state: *mut c_void, handle: EntityHandleV1, hp: usize) -> bool>,
    /// Teleports the entity.
    pub entity_set_pos: Option<
        unsafe extern "C" fn(state: *mut c_void, handle: EntityHandleV1, x: u64, y: u64) -> bool,
    >,
    /// Replaces the entity's base stat block (buffs still apply on top).
    pub entity_set_base_stat: Option<
        unsafe extern "C" fn(
            state: *mut c_void,
            handle: EntityHandleV1,
            stat: *const StatV1,
        ) -> bool,
    >,
    pub player_set_gold: Option<
        unsafe extern "C" fn(state: *mut c_void, handle: PlayerHandleV1, gold: usize) -> bool,
    >,
    /// Saturating gold delta (negative values floor at zero).
    pub player_add_gold: Option<
        unsafe extern "C" fn(state: *mut c_void, handle: PlayerHandleV1, delta: i64) -> bool,
    >,
    /// Ends the match immediately with the given winner (same mechanism as
    /// the match-hook end decision).
    pub force_end: Option<unsafe extern "C" fn(state: *mut c_void, blue_win: bool)>,
    /// Remaining cooldown ticks of the player's attack/skill/skill2/ult.
    pub player_cooldowns: Option<
        unsafe extern "C" fn(
            state: *const c_void,
            handle: PlayerHandleV1,
            out_attack: *mut usize,
            out_skill: *mut usize,
            out_skill2: *mut usize,
            out_ult: *mut usize,
        ) -> bool,
    >,
    /// Number of items the player currently owns.
    pub player_item_count:
        Option<unsafe extern "C" fn(state: *const c_void, handle: PlayerHandleV1) -> usize>,
    /// Item key of the `index`-th owned item (two-call string output).
    pub player_item_key_at: Option<
        unsafe extern "C" fn(
            state: *const c_void,
            handle: PlayerHandleV1,
            index: usize,
            buf: *mut u8,
            buf_cap: usize,
            out_len: *mut usize,
        ) -> bool,
    >,
    /// Entity display/champion name (two-call string output).
    pub entity_name: Option<
        unsafe extern "C" fn(
            state: *const c_void,
            handle: EntityHandleV1,
            buf: *mut u8,
            buf_cap: usize,
            out_len: *mut usize,
        ) -> bool,
    >,
    /// Queues a registered native effect (by name) to fire `delay_ticks`
    /// from now with the given caster and input.
    pub queue_effect: Option<
        unsafe extern "C" fn(
            state: *mut c_void,
            effect_name: *const u8,
            effect_name_len: usize,
            attack_type: u32,
            caster_id: usize,
            input: *const crate::InputTargetV1,
            delay_ticks: usize,
        ) -> bool,
    >,
    /// Spawns a duration-limited combat unit that chases and attacks nearby
    /// enemies. Returns the new entity id, or usize::MAX on failure.
    pub spawn_unit: Option<
        unsafe extern "C" fn(
            state: *mut c_void,
            name: *const u8,
            name_len: usize,
            summoner_id: usize,
            team: usize,
            x: u64,
            y: u64,
            duration_ticks: u64,
            stat: *const StatV1,
            attack: *const crate::UnitAttackV1,
        ) -> usize,
    >,
    /// Spawns a projectile that applies a registered native effect on hit.
    pub spawn_projectile: Option<
        unsafe extern "C" fn(
            state: *mut c_void,
            name: *const u8,
            name_len: usize,
            effect_name: *const u8,
            effect_name_len: usize,
            spec: *const crate::ProjectileSpawnV1,
        ) -> bool,
    >,
    /// Reads a fragment of a team's live macro strategy document as JSON.
    pub strategy_get_json: Option<
        unsafe extern "C" fn(
            state: *const c_void,
            team: usize,
            path: *const u8,
            path_len: usize,
            buf: *mut u8,
            buf_cap: usize,
            out_len: *mut usize,
        ) -> bool,
    >,
    /// Replaces a fragment of a team's macro strategy document (atomic
    /// reject on schema violation).
    pub strategy_set_json: Option<
        unsafe extern "C" fn(
            state: *mut c_void,
            team: usize,
            path: *const u8,
            path_len: usize,
            json: *const u8,
            json_len: usize,
        ) -> bool,
    >,
    /// Reads a fragment of the player's cumulative match statistics document
    /// (rating, solo kills, exp breakdown, ...) as JSON.
    pub player_statistics_json: Option<
        unsafe extern "C" fn(
            state: *const c_void,
            handle: PlayerHandleV1,
            path: *const u8,
            path_len: usize,
            buf: *mut u8,
            buf_cap: usize,
            out_len: *mut usize,
        ) -> bool,
    >,
    /// Removes every stat buff with the given name from the entity; returns
    /// the number removed.
    pub entity_remove_buff: Option<
        unsafe extern "C" fn(
            state: *mut c_void,
            handle: EntityHandleV1,
            name: *const u8,
            name_len: usize,
        ) -> usize,
    >,
    /// Clears every crowd-control effect on the entity; returns the number
    /// removed.
    pub entity_clear_cc:
        Option<unsafe extern "C" fn(state: *mut c_void, handle: EntityHandleV1) -> usize>,
    // -- level 2 --
    /// Grants a damage-absorbing shield that expires after `duration_ticks`.
    /// Shields stack as separate layers, matching the built-in shield effect.
    pub entity_add_shield: Option<
        unsafe extern "C" fn(
            state: *mut c_void,
            handle: EntityHandleV1,
            amount: usize,
            duration_ticks: usize,
        ) -> bool,
    >,
    /// Drops every shield layer on the entity; returns the number removed.
    pub entity_clear_shield:
        Option<unsafe extern "C" fn(state: *mut c_void, handle: EntityHandleV1) -> usize>,
    /// Ticks between the entity's auto-attacks, after attack-speed buffs.
    pub entity_attack_interval:
        Option<unsafe extern "C" fn(state: *const c_void, handle: EntityHandleV1) -> usize>,
    /// Attack-speed multiplier in percent (100 = unbuffed).
    pub entity_attack_speed_mult:
        Option<unsafe extern "C" fn(state: *const c_void, handle: EntityHandleV1) -> usize>,
    /// Raw HP subtraction with no mitigation, statistics or kill credit —
    /// the pre-level-2 behaviour of `deal_damage`, kept for mods that model
    /// their own damage math. Prefer `deal_damage`.
    pub deal_damage_raw: Option<
        unsafe extern "C" fn(
            state: *mut c_void,
            attacker: usize,
            target: usize,
            ad: usize,
            ap: usize,
            attack_type: u32,
        ),
    >,
}

/// Frame/debug drawing during a running match.
#[repr(C)]
pub struct FrameVtableV1 {
    pub size: usize,
    pub debug_draw_line: Option<
        unsafe extern "C" fn(state: *mut c_void, x1: u64, y1: u64, x2: u64, y2: u64, color: u32),
    >,
    pub debug_draw_circle:
        Option<unsafe extern "C" fn(state: *mut c_void, x: u64, y: u64, r: u64, color: u32)>,
    pub debug_text: Option<
        unsafe extern "C" fn(
            state: *mut c_void,
            x: u64,
            y: u64,
            text: *const u8,
            text_len: usize,
            color: u32,
        ),
    >,
}

/// Semantic version of a mod-provided runtime service. Frozen value type.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ServiceVersionV1 {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl ServiceVersionV1 {
    pub const fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }
}

/// Opaque service published by one mod and consumed by another. Both sides
/// agree on the concrete `#[repr(C)]` vtable behind `vtable` out of band.
/// Provider-owned; must stay valid while the provider DLL is loaded.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct ModServiceV1 {
    pub data: *mut c_void,
    pub vtable: *const c_void,
}

impl ModServiceV1 {
    pub const NULL: Self = Self {
        data: std::ptr::null_mut(),
        vtable: std::ptr::null(),
    };

    pub fn is_null(&self) -> bool {
        self.vtable.is_null()
    }

    /// # Safety
    /// The caller must know the provider's actual vtable type for this
    /// service id + version.
    pub unsafe fn vtable_as<T>(&self) -> Option<&T> {
        if self.vtable.is_null() {
            None
        } else {
            Some(&*(self.vtable as *const T))
        }
    }
}

/// Mod-to-mod runtime service registry. Registration and queries are only
/// valid while a mod's entry runs (dependency order guarantees providers
/// load first); outside loading both slots fail with `false`.
#[repr(C)]
pub struct ServiceVtableV1 {
    pub size: usize,
    /// Publish a service under the currently loading mod's id.
    pub register_service: Option<
        unsafe extern "C" fn(
            service_id: *const u8,
            service_id_len: usize,
            version: ServiceVersionV1,
            service: ModServiceV1,
        ) -> bool,
    >,
    /// `version_req` is a semver requirement string (e.g. ">=1.0.0, <2.0.0").
    pub query_service: Option<
        unsafe extern "C" fn(
            provider_mod_id: *const u8,
            provider_mod_id_len: usize,
            service_id: *const u8,
            service_id_len: usize,
            version_req: *const u8,
            version_req_len: usize,
            out_service: *mut ModServiceV1,
        ) -> bool,
    >,
}

/// Client-side mod save data access (namespace = the mod's own id, enforced
/// by the host). Byte outputs use the two-call convention. Writes are
/// rejected when the client has no write authority (multiplayer guest) or
/// when no game session is active.
#[repr(C)]
pub struct SaveVtableV1 {
    pub size: usize,
    pub can_write: Option<unsafe extern "C" fn(state: *const c_void) -> bool>,
    pub version: Option<unsafe extern "C" fn(state: *const c_void) -> usize>,
    pub set_version: Option<unsafe extern "C" fn(state: *mut c_void, version: usize) -> bool>,
    pub contains_key:
        Option<unsafe extern "C" fn(state: *const c_void, key: *const u8, key_len: usize) -> bool>,
    pub get: Option<
        unsafe extern "C" fn(
            state: *const c_void,
            key: *const u8,
            key_len: usize,
            buf: *mut u8,
            buf_cap: usize,
            out_len: *mut usize,
        ) -> bool,
    >,
    pub set: Option<
        unsafe extern "C" fn(
            state: *mut c_void,
            key: *const u8,
            key_len: usize,
            value: *const u8,
            value_len: usize,
        ) -> bool,
    >,
    pub remove:
        Option<unsafe extern "C" fn(state: *mut c_void, key: *const u8, key_len: usize) -> bool>,
    pub key_count: Option<unsafe extern "C" fn(state: *const c_void) -> usize>,
    pub key_at: Option<
        unsafe extern "C" fn(
            state: *const c_void,
            index: usize,
            buf: *mut u8,
            buf_cap: usize,
            out_len: *mut usize,
        ) -> bool,
    >,
}

/// Client-side mod command/event messaging. Events are consumed by first
/// draining them into the callback context (`take_events`) and then reading
/// each drained event by index with the two-call convention.
#[repr(C)]
pub struct NetVtableV1 {
    pub size: usize,
    pub send_command: Option<
        unsafe extern "C" fn(
            state: *mut c_void,
            command: *const u8,
            command_len: usize,
            payload: *const u8,
            payload_len: usize,
        ) -> bool,
    >,
    /// Drains this mod's pending events into the context; returns the count.
    pub take_events: Option<unsafe extern "C" fn(state: *mut c_void) -> usize>,
    pub taken_event_at: Option<
        unsafe extern "C" fn(
            state: *const c_void,
            index: usize,
            name_buf: *mut u8,
            name_cap: usize,
            out_name_len: *mut usize,
            payload_buf: *mut u8,
            payload_cap: usize,
            out_payload_len: *mut usize,
        ) -> bool,
    >,
}

/// Management data read accessors. Only live inside an active game session
/// (InGame scene); every slot fails gracefully elsewhere. Grown append-only
/// on demand.
#[repr(C)]
pub struct DataVtableV1 {
    pub size: usize,
    pub player_team_id: Option<unsafe extern "C" fn(state: *const c_void, out: *mut usize) -> bool>,
    /// Writes up to `cap` team ids; returns the total count.
    pub team_ids:
        Option<unsafe extern "C" fn(state: *const c_void, out: *mut usize, cap: usize) -> usize>,
    pub team_name: Option<
        unsafe extern "C" fn(
            state: *const c_void,
            team_id: usize,
            buf: *mut u8,
            buf_cap: usize,
            out_len: *mut usize,
        ) -> bool,
    >,
    pub athlete_ids:
        Option<unsafe extern "C" fn(state: *const c_void, out: *mut usize, cap: usize) -> usize>,
    pub athlete_name: Option<
        unsafe extern "C" fn(
            state: *const c_void,
            athlete_id: usize,
            buf: *mut u8,
            buf_cap: usize,
            out_len: *mut usize,
        ) -> bool,
    >,
    /// Read-only view of the active save's settings documents (same
    /// target/path semantics as the server-side slot).
    pub setting_get_json: Option<
        unsafe extern "C" fn(
            state: *const c_void,
            target: u32,
            path: *const u8,
            path_len: usize,
            buf: *mut u8,
            buf_cap: usize,
            out_len: *mut usize,
        ) -> bool,
    >,
    /// Writes up to `cap` record ids of the given [`crate::RecordKindV1`];
    /// returns the total count.
    pub record_ids: Option<
        unsafe extern "C" fn(state: *const c_void, kind: u32, out: *mut usize, cap: usize) -> usize,
    >,
    /// Reads a fragment of a management record as JSON (read-only; the
    /// server-side team/athlete slots are the write path).
    pub record_get_json: Option<
        unsafe extern "C" fn(
            state: *const c_void,
            kind: u32,
            record_id: usize,
            path: *const u8,
            path_len: usize,
            buf: *mut u8,
            buf_cap: usize,
            out_len: *mut usize,
        ) -> bool,
    >,
    /// Current in-game date/time.
    pub game_time: Option<
        unsafe extern "C" fn(
            state: *const c_void,
            out_year: *mut i32,
            out_month: *mut u32,
            out_day: *mut u32,
            out_hour: *mut u32,
            out_minute: *mut u32,
        ) -> bool,
    >,
    /// Player team's all-time official head-to-head against `opponent_team_id`.
    pub head_to_head: Option<
        unsafe extern "C" fn(
            state: *const c_void,
            opponent_team_id: usize,
            out_wins: *mut usize,
            out_losses: *mut usize,
        ) -> bool,
    >,
    /// Final result of a finished competition: champion and runner-up team
    /// ids. `is_tournament` picks tournament vs league playoffs.
    pub competition_result: Option<
        unsafe extern "C" fn(
            state: *const c_void,
            is_tournament: bool,
            competition_id: usize,
            out_champion_team: *mut usize,
            out_runner_up_team: *mut usize,
        ) -> bool,
    >,
    /// Read-only JSON view of the season schedule document (array of year
    /// schedules), same path semantics as the other doc slots.
    pub schedule_get_json: Option<
        unsafe extern "C" fn(
            state: *const c_void,
            path: *const u8,
            path_len: usize,
            buf: *mut u8,
            buf_cap: usize,
            out_len: *mut usize,
        ) -> bool,
    >,
    /// Number of selectable champions in the active (patched) sheet.
    pub champion_count: Option<unsafe extern "C" fn(state: *const c_void) -> usize>,
    /// Name of the `index`-th selectable champion (two-call string output).
    pub champion_name_at: Option<
        unsafe extern "C" fn(
            state: *const c_void,
            index: usize,
            buf: *mut u8,
            buf_cap: usize,
            out_len: *mut usize,
        ) -> bool,
    >,
    /// Static identity of a champion by name. The brief's `name` field is
    /// set to the caller's own name argument.
    pub champion_brief: Option<
        unsafe extern "C" fn(
            state: *const c_void,
            name: *const u8,
            name_len: usize,
            out: *mut crate::ChampionBriefV1,
        ) -> bool,
    >,
    /// Current in-client screen ([`crate::ClientSceneKindV1`] code;
    /// u32::MAX outside InGame).
    pub client_scene_kind: Option<unsafe extern "C" fn(state: *const c_void) -> u32>,
    /// Variant name of the current main-screen tab (e.g. "Home", "Squad",
    /// "PlayerDetail"). False when the client is not on the Main screen.
    pub client_main_tab_name: Option<
        unsafe extern "C" fn(
            state: *const c_void,
            buf: *mut u8,
            buf_cap: usize,
            out_len: *mut usize,
        ) -> bool,
    >,
}

/// UI node query/mutation, handler registration, template spawn. Node paths
/// are dot-joined id selectors (e.g. "body.popup.buttons.close"), matching
/// the internal UI query language.
#[repr(C)]
pub struct UiVtableV1 {
    pub size: usize,
    pub query_exists: Option<
        unsafe extern "C" fn(state: *const c_void, path: *const u8, path_len: usize) -> bool,
    >,
    pub get_visible: Option<
        unsafe extern "C" fn(
            state: *const c_void,
            path: *const u8,
            path_len: usize,
            out_visible: *mut bool,
        ) -> bool,
    >,
    pub set_visible: Option<
        unsafe extern "C" fn(
            state: *mut c_void,
            path: *const u8,
            path_len: usize,
            visible: bool,
        ) -> bool,
    >,
    /// Loads a `NodeTemplate` asset and pushes it as a child of `parent`.
    pub spawn_template: Option<
        unsafe extern "C" fn(
            state: *mut c_void,
            parent_path: *const u8,
            parent_path_len: usize,
            template_asset: *const u8,
            template_asset_len: usize,
            visible: bool,
        ) -> bool,
    >,
    /// Registers a click handler for (path, item). The callback fires with a
    /// reduced context (ui/asset only); userdata lives until process exit.
    pub register_click: Option<
        unsafe extern "C" fn(
            state: *mut c_void,
            path: *const u8,
            path_len: usize,
            item: *const u8,
            item_len: usize,
            callback: Option<crate::UiClickHandlerFn>,
            userdata: *mut c_void,
        ) -> bool,
    >,
    /// Reads a label/text node's text (two-call output).
    pub get_text: Option<
        unsafe extern "C" fn(
            state: *const c_void,
            path: *const u8,
            path_len: usize,
            buf: *mut u8,
            buf_cap: usize,
            out_len: *mut usize,
        ) -> bool,
    >,
    /// Sets a label node's text.
    pub set_text: Option<
        unsafe extern "C" fn(
            state: *mut c_void,
            path: *const u8,
            path_len: usize,
            text: *const u8,
            text_len: usize,
        ) -> bool,
    >,
    /// Registers a handler for EVERY UI event whose path equals `path`.
    /// Inside the callback, `current_event` describes the firing event.
    pub register_path_events: Option<
        unsafe extern "C" fn(
            state: *mut c_void,
            path: *const u8,
            path_len: usize,
            callback: Option<crate::UiClickHandlerFn>,
            userdata: *mut c_void,
        ) -> bool,
    >,
    /// Inside a `register_path_events` callback: the firing event's kind
    /// ([`crate::UiEventKindV1`]), path, and payload JSON. False elsewhere.
    pub current_event: Option<
        unsafe extern "C" fn(
            state: *const c_void,
            out_kind: *mut u32,
            path_buf: *mut u8,
            path_cap: usize,
            out_path_len: *mut usize,
            payload_buf: *mut u8,
            payload_cap: usize,
            out_payload_len: *mut usize,
        ) -> bool,
    >,
    /// Parses `.ui` template source (`id:runner { key: value; ... }`, children
    /// and style refs included) and pushes the built subtree as a child of
    /// `parent`. Any registered runner kind can be created this way. False on
    /// parse error, unknown parent, or trailing junk after the template.
    pub spawn_source: Option<
        unsafe extern "C" fn(
            state: *mut c_void,
            parent_path: *const u8,
            parent_path_len: usize,
            source: *const u8,
            source_len: usize,
        ) -> bool,
    >,
    /// Applies `.ui` property source (`key: value; ...`) to an existing node:
    /// layout keys, `visible`/`disable`, and the node's runner properties in
    /// one pass. False on parse error/trailing junk or unknown node.
    pub set_properties: Option<
        unsafe extern "C" fn(
            state: *mut c_void,
            path: *const u8,
            path_len: usize,
            source: *const u8,
            source_len: usize,
        ) -> bool,
    >,
    /// Removes the node at `path` (children included). False if absent.
    pub remove_node:
        Option<unsafe extern "C" fn(state: *mut c_void, path: *const u8, path_len: usize) -> bool>,
    /// Registered runner name of the node at `path` ("label", "checkbox",
    /// ...; two-call output). True with an empty string when the node exists
    /// but its runner kind is not in the host registry.
    pub runner_name: Option<
        unsafe extern "C" fn(
            state: *const c_void,
            path: *const u8,
            path_len: usize,
            buf: *mut u8,
            buf_cap: usize,
            out_len: *mut usize,
        ) -> bool,
    >,
    /// Runtime interaction state of the node's runner as a JSON object
    /// (two-call output). Keys per kind: label {"text"}, checkbox
    /// {"selected","text"}, text_edit {"text","is_editing"}, slider
    /// {"ratio"}, selectable {"selected","text"}, dropdown
    /// {"selected_item"}; other kinds yield {}.
    pub state_get_json: Option<
        unsafe extern "C" fn(
            state: *const c_void,
            path: *const u8,
            path_len: usize,
            buf: *mut u8,
            buf_cap: usize,
            out_len: *mut usize,
        ) -> bool,
    >,
    /// Writes runtime interaction state from a JSON object. Writable keys per
    /// kind: checkbox {"selected"}, text_edit {"text"}, slider {"ratio"},
    /// selectable {"selected"}. Rejected atomically (false) on any unknown
    /// key, wrong type, or non-writable runner kind.
    pub state_set_json: Option<
        unsafe extern "C" fn(
            state: *mut c_void,
            path: *const u8,
            path_len: usize,
            json: *const u8,
            json_len: usize,
        ) -> bool,
    >,
    /// Computed layout rect of a node from the last layout pass.
    /// `contents` picks the contents rect instead of the node rect.
    pub node_rect: Option<
        unsafe extern "C" fn(
            state: *const c_void,
            path: *const u8,
            path_len: usize,
            contents: bool,
            out_x: *mut f32,
            out_y: *mut f32,
            out_w: *mut f32,
            out_h: *mut f32,
        ) -> bool,
    >,
    /// Number of direct children of a node (false when the path is missing).
    pub child_count: Option<
        unsafe extern "C" fn(
            state: *const c_void,
            path: *const u8,
            path_len: usize,
            out_count: *mut usize,
        ) -> bool,
    >,
    /// Id of the `index`-th direct child of a node.
    pub child_name_at: Option<
        unsafe extern "C" fn(
            state: *const c_void,
            path: *const u8,
            path_len: usize,
            index: usize,
            buf: *mut u8,
            buf_cap: usize,
            out_len: *mut usize,
        ) -> bool,
    >,
    /// Number of raw keyboard events recorded this frame (hotkey surface).
    pub input_event_count: Option<unsafe extern "C" fn(state: *const c_void) -> usize>,
    /// The `index`-th raw keyboard event this frame: kind =
    /// [`crate::InputEventKindV1`] code, key = engine key name (e.g. "Tab",
    /// "F5") via the two-call convention.
    pub input_event_at: Option<
        unsafe extern "C" fn(
            state: *const c_void,
            index: usize,
            out_kind: *mut u32,
            key_buf: *mut u8,
            key_cap: usize,
            out_key_len: *mut usize,
        ) -> bool,
    >,
}

/// Current scene kind ([`crate::SceneKindV1`] code; u32::MAX when unknown)
/// plus per-scene accessors (size-guarded appends).
#[repr(C)]
pub struct SceneVtableV1 {
    pub size: usize,
    pub kind: Option<unsafe extern "C" fn(state: *const c_void) -> u32>,
    // -- NewGame scene accessors (fail outside the NewGame scene) --
    /// Team the new game will start with.
    pub new_game_team_id:
        Option<unsafe extern "C" fn(state: *const c_void, out: *mut usize) -> bool>,
    pub new_game_set_team_id:
        Option<unsafe extern "C" fn(state: *mut c_void, team_id: usize) -> bool>,
    pub new_game_with_tutorial:
        Option<unsafe extern "C" fn(state: *const c_void, out: *mut bool) -> bool>,
    pub new_game_set_with_tutorial:
        Option<unsafe extern "C" fn(state: *mut c_void, with_tutorial: bool) -> bool>,
    /// Manager name (two-call string output / UTF-8 input).
    pub new_game_manager_name: Option<
        unsafe extern "C" fn(
            state: *const c_void,
            buf: *mut u8,
            buf_cap: usize,
            out_len: *mut usize,
        ) -> bool,
    >,
    pub new_game_set_manager_name:
        Option<unsafe extern "C" fn(state: *mut c_void, name: *const u8, name_len: usize) -> bool>,
    /// Reads a fragment of the new game's play-option document (mode, rules,
    /// champion pool, ...) as JSON.
    pub new_game_option_get_json: Option<
        unsafe extern "C" fn(
            state: *const c_void,
            path: *const u8,
            path_len: usize,
            buf: *mut u8,
            buf_cap: usize,
            out_len: *mut usize,
        ) -> bool,
    >,
    /// Replaces a fragment of the play-option document (atomic reject on
    /// schema violation).
    pub new_game_option_set_json: Option<
        unsafe extern "C" fn(
            state: *mut c_void,
            path: *const u8,
            path_len: usize,
            json: *const u8,
            json_len: usize,
        ) -> bool,
    >,
}

/// i18n and path-based asset queries.
#[repr(C)]
pub struct AssetVtableV1 {
    pub size: usize,
    /// Resolves an i18n key (two-call string output).
    pub i18n_get: Option<
        unsafe extern "C" fn(
            state: *const c_void,
            key: *const u8,
            key_len: usize,
            buf: *mut u8,
            buf_cap: usize,
            out_len: *mut usize,
        ) -> bool,
    >,
    /// Plays a sound asset once (e.g. "asset/<mod_id>/sound/sfx/x"). The
    /// request is queued and played by the game's own frame loop.
    pub play_sound: Option<
        unsafe extern "C" fn(
            state: *mut c_void,
            path: *const u8,
            path_len: usize,
            volume: f32,
        ) -> bool,
    >,
    /// Replaces the background-music playlist with the given sound assets.
    pub play_bgm: Option<
        unsafe extern "C" fn(state: *mut c_void, paths: *const crate::StrV1, count: usize) -> bool,
    >,
    /// Stops the current background music.
    pub stop_bgm: Option<unsafe extern "C" fn(state: *mut c_void) -> bool>,
}

/// Root host API passed to `tfm2_mod_entry_stable` and to every mod callback
/// that needs host access. Owned by the host; valid for the process lifetime.
///
/// Table pointers are never null on a conforming host, but their pointed-to
/// tables may be size-only (no slots) — mods must probe via table `size` and
/// `Option` slots rather than assume capability.
#[repr(C)]
pub struct HostApiV1 {
    pub size: usize,
    /// Host's [`crate::ABI_LEVEL`]. Mods compare against slot introduction
    /// levels for optional-feature detection.
    pub host_abi_level: u32,
    pub game_version: GameVersionV1,
    /// Write a message into the game log. `msg` is UTF-8, not NUL-terminated.
    pub log: Option<unsafe extern "C" fn(level: u32, msg: *const u8, msg_len: usize)>,
    pub sim: *const SimVtableV1,
    pub frame: *const FrameVtableV1,
    pub service: *const ServiceVtableV1,
    pub save: *const SaveVtableV1,
    pub net: *const NetVtableV1,
    pub data: *const DataVtableV1,
    pub ui: *const UiVtableV1,
    pub scene: *const SceneVtableV1,
    pub asset: *const AssetVtableV1,
}
