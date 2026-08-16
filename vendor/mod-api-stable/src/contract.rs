//! Machine-readable dump of the ABI contract, consumed by the guard tests.
//!
//! Line kinds and their rules:
//! - `field T.name offset=N type=...` — immutable once published. A frozen
//!   snapshot line must reappear verbatim in every future dump.
//! - `layout T align=N` — immutable once published.
//! - `const NAME = N` / `symbol NAME` — immutable once published (append-only).
//! - `size T = N` / `abi_level = N` — may grow when fields/slots are appended
//!   together with an ABI_LEVEL bump; excluded from the frozen-line rule.

use std::mem::{align_of, offset_of, size_of};

use crate::*;

macro_rules! dump_struct {
    ($out:ident, $T:ident { $($field:tt : $fty:ty),* $(,)? }) => {{
        $out.push_str(&format!("layout {} align={}\n", stringify!($T), align_of::<$T>()));
        $out.push_str(&format!("size {} = {}\n", stringify!($T), size_of::<$T>()));
        $(
            {
                // Compile-time check that the listed field exists with the
                // listed type, so the dump cannot drift from the definition.
                let _check: fn(&$T) -> &$fty = |s| &s.$field;
                $out.push_str(&format!(
                    "field {}.{} offset={} type={}\n",
                    stringify!($T),
                    stringify!($field),
                    offset_of!($T, $field),
                    stringify!($fty).replace(' ', ""),
                ));
            }
        )*
    }};
}

/// Render the full current contract. Guard tests compare this against the
/// committed snapshot in `contract/abi_v1.txt`.
pub fn contract_dump() -> String {
    let mut out = String::new();

    out.push_str(&format!("abi_level = {}\n", ABI_LEVEL));
    out.push_str(&format!("symbol {}\n", MOD_REQUIRED_ABI_LEVEL_SYMBOL));
    out.push_str(&format!("symbol {}\n", MOD_STABLE_ENTRY_SYMBOL));
    out.push_str(&format!("const LOG_LEVEL_ERROR = {}\n", LOG_LEVEL_ERROR));
    out.push_str(&format!("const LOG_LEVEL_WARN = {}\n", LOG_LEVEL_WARN));
    out.push_str(&format!("const LOG_LEVEL_INFO = {}\n", LOG_LEVEL_INFO));
    out.push_str(&format!("const LOG_LEVEL_DEBUG = {}\n", LOG_LEVEL_DEBUG));

    for (name, variants) in [
        ("ChampionCategoryV1", ChampionCategoryV1::VARIANTS),
        ("ChampionTagV1", ChampionTagV1::VARIANTS),
        ("ItemTagV1", ItemTagV1::VARIANTS),
        ("ItemCategoryV1", ItemCategoryV1::VARIANTS),
        ("AttackTypeV1", AttackTypeV1::VARIANTS),
        ("DamageTypeV1", DamageTypeV1::VARIANTS),
        ("CastingTypeV1", CastingTypeV1::VARIANTS),
        ("CastingTargetV1", CastingTargetV1::VARIANTS),
        ("LaneV1", LaneV1::VARIANTS),
        ("CcKindV1", CcKindV1::VARIANTS),
        ("BuffDurationV1", BuffDurationV1::VARIANTS),
        ("InputTargetKindV1", InputTargetKindV1::VARIANTS),
        ("EventTargetKindV1", EventTargetKindV1::VARIANTS),
        ("CommandResultV1", CommandResultV1::VARIANTS),
        ("SceneKindV1", SceneKindV1::VARIANTS),
        ("UiEventKindV1", UiEventKindV1::VARIANTS),
        ("InputKindV1", InputKindV1::VARIANTS),
        ("DraftPhaseV1", DraftPhaseV1::VARIANTS),
        ("DifficultyV1", DifficultyV1::VARIANTS),
        ("DraftDecisionKindV1", DraftDecisionKindV1::VARIANTS),
        ("AiDecisionKindV1", AiDecisionKindV1::VARIANTS),
        ("GameModeKindV1", GameModeKindV1::VARIANTS),
        ("SettingTargetV1", SettingTargetV1::VARIANTS),
        ("RecordKindV1", RecordKindV1::VARIANTS),
        ("TextAlignXV1", TextAlignXV1::VARIANTS),
        ("TextAlignYV1", TextAlignYV1::VARIANTS),
        ("ProjectileMoveKindV1", ProjectileMoveKindV1::VARIANTS),
        ("ManagementEventKindV1", ManagementEventKindV1::VARIANTS),
        ("ClientSceneKindV1", ClientSceneKindV1::VARIANTS),
        ("InputEventKindV1", InputEventKindV1::VARIANTS),
    ] {
        for (variant, code) in variants {
            out.push_str(&format!("enumcode {}.{} = {}\n", name, variant, code));
        }
    }

    dump_struct!(
        out,
        GameVersionV1 {
            major: u32,
            minor: u32,
            patch: u32
        }
    );

    dump_struct!(out, EntityHandleV1 { 0: u64 });
    dump_struct!(out, PlayerHandleV1 { 0: u64 });
    dump_struct!(out, ProjectileHandleV1 { 0: u64 });

    dump_struct!(out, StrV1 { ptr: *const u8, len: usize });

    dump_struct!(
        out,
        StatV1 {
            attack: usize,
            magic_power: usize,
            hp: usize,
            defence: usize,
            magic_resistance: usize,
            move_speed: usize,
            hp_regen: usize,
            stack: usize,
            crit_chance: usize,
        }
    );

    dump_struct!(
        out,
        BuffV1 {
            name_buf: [u8; BUFF_NAME_CAP],
            name_len: usize,
            duration_kind: u32,
            duration_tick: usize,
            attack: i32,
            attack_mult: i32,
            magic_power: i32,
            magic_power_mult: i32,
            defence: i32,
            defence_mult: i32,
            hp: i32,
            hp_regen: i32,
            magic_resistance: i32,
            magic_resistance_mult: i32,
            vamp: i32,
            hp_mult: i32,
            move_speed_mult: i32,
            attack_speed_mult: i32,
            skill_cooldown_mult: i32,
            ult_cooldown_mult: i32,
            radius_mult: i32,
            crit_chance: i32,
            damage_reflect: usize,
            damaged_amplify: usize,
            damaged_reduce: usize,
            defence_penetration: usize,
            magic_resistance_penetration: usize,
            toughness: usize,
            heal_reduce: usize,
            range: usize,
            base_attack_enemy_max_hp_damage: usize,
            self_max_hp_damage: usize,
            skill_enemy_max_hp_damage: usize,
            dot_amplify: usize,
            base_attack_damaged_reduce: usize,
            skill_damaged_reduce: usize,
            cc_immune: bool,
            undying: bool,
            ignore_wall: bool,
        }
    );

    dump_struct!(
        out,
        CcV1 {
            kind: u32,
            tick: u64,
            dx: i64,
            dy: i64,
            speed: u64,
            target: usize,
            name_buf: [u8; BUFF_NAME_CAP],
            name_len: usize,
        }
    );

    dump_struct!(
        out,
        InputTargetV1 {
            kind: u32,
            target_id: usize,
            dir_x: i64,
            dir_y: i64,
            x: u64,
            y: u64,
        }
    );

    dump_struct!(
        out,
        KillLogV1 {
            tick: usize,
            killer_team: usize,
            killer_position: u32,
            killed_position: u32,
            assist_count: u32,
            assist_positions: [u32; 4],
        }
    );

    dump_struct!(
        out,
        ProjectileInfoV1 {
            x: u64,
            y: u64,
            caster_id: usize,
            team: usize,
            is_end: bool,
        }
    );

    dump_struct!(out, SimCtxV1 {
        size: usize,
        sim: *const SimVtableV1,
        frame: *const FrameVtableV1,
        state: *mut std::ffi::c_void,
    });

    dump_struct!(out, ChampionRegV1 { userdata: *mut std::ffi::c_void, vtable: *const ChampionVtableV1 });
    dump_struct!(out, ActionRegV1 { userdata: *mut std::ffi::c_void, vtable: *const ActionVtableV1 });
    dump_struct!(out, EffectTypeRegV1 { userdata: *mut std::ffi::c_void, vtable: *const EffectTypeVtableV1 });
    dump_struct!(out, PassiveRegV1 { userdata: *mut std::ffi::c_void, vtable: *const PassiveVtableV1 });
    dump_struct!(out, ItemRegV1 { userdata: *mut std::ffi::c_void, vtable: *const ItemVtableV1 });
    dump_struct!(
        out,
        NativeEffectRegV1 {
            name: StrV1,
            effect: EffectTypeRegV1
        }
    );

    dump_struct!(
        out,
        EffectSpecV1 {
            range: u64,
            growth_range: u64,
            start_timing: usize,
            casting: u32,
            target: u32,
            attack_type: u32,
            effect: EffectTypeRegV1,
        }
    );

    dump_struct!(out, ChampionVtableV1 {
        size: usize,
        destroy: Option<unsafe extern "C" fn(*mut std::ffi::c_void)>,
        id: Option<unsafe extern "C" fn(*const std::ffi::c_void) -> StrV1>,
        name: Option<unsafe extern "C" fn(*const std::ffi::c_void) -> StrV1>,
        skill_icon: Option<unsafe extern "C" fn(*const std::ffi::c_void, usize, *mut StrV1, *mut StrV1) -> bool>,
        category: Option<unsafe extern "C" fn(*const std::ffi::c_void) -> u32>,
        tags: Option<unsafe extern "C" fn(*const std::ffi::c_void, *mut u32, usize) -> usize>,
        stat: Option<unsafe extern "C" fn(*const std::ffi::c_void, *mut StatV1)>,
        growth: Option<unsafe extern "C" fn(*const std::ffi::c_void, *mut StatV1)>,
        attack: Option<unsafe extern "C" fn(*const std::ffi::c_void, *mut ActionRegV1) -> bool>,
        skill: Option<unsafe extern "C" fn(*const std::ffi::c_void, *mut ActionRegV1) -> bool>,
        skill2: Option<unsafe extern "C" fn(*const std::ffi::c_void, *mut ActionRegV1) -> bool>,
        ult: Option<unsafe extern "C" fn(*const std::ffi::c_void, *mut ActionRegV1) -> bool>,
        passive: Option<unsafe extern "C" fn(*const std::ffi::c_void, *mut PassiveRegV1) -> bool>,
        lane_prior: Option<unsafe extern "C" fn(*const std::ffi::c_void, *mut usize, usize) -> bool>,
    });

    dump_struct!(out, ActionVtableV1 {
        size: usize,
        destroy: Option<unsafe extern "C" fn(*mut std::ffi::c_void)>,
        clone: Option<unsafe extern "C" fn(*const std::ffi::c_void) -> *mut std::ffi::c_void>,
        action_name: Option<unsafe extern "C" fn(*const std::ffi::c_void) -> StrV1>,
        duration: Option<unsafe extern "C" fn(*const std::ffi::c_void) -> usize>,
        cancelable: Option<unsafe extern "C" fn(*const std::ffi::c_void) -> bool>,
        cooltime: Option<unsafe extern "C" fn(*const std::ffi::c_void, *const StatV1, usize) -> usize>,
        casting_target: Option<unsafe extern "C" fn(*const std::ffi::c_void) -> u32>,
        effect: Option<unsafe extern "C" fn(*const std::ffi::c_void, *mut EffectSpecV1) -> bool>,
        cooltime_use_count: Option<unsafe extern "C" fn(*const std::ffi::c_void, *const StatV1) -> usize>,
        can_use_with_move: Option<unsafe extern "C" fn(*const std::ffi::c_void) -> bool>,
        description: Option<unsafe extern "C" fn(*const std::ffi::c_void) -> StrV1>,
    });

    dump_struct!(out, EffectTypeVtableV1 {
        size: usize,
        destroy: Option<unsafe extern "C" fn(*mut std::ffi::c_void)>,
        apply: Option<unsafe extern "C" fn(*mut std::ffi::c_void, *mut SimCtxV1, u64, usize, *const InputTargetV1)>,
        expected_damage: Option<unsafe extern "C" fn(*const std::ffi::c_void, *const StatV1, *mut usize, *mut usize)>,
        expected_heal: Option<unsafe extern "C" fn(*const std::ffi::c_void, *const StatV1) -> usize>,
        expected_shield: Option<unsafe extern "C" fn(*const std::ffi::c_void, *const StatV1) -> usize>,
        expected_cc_time: Option<unsafe extern "C" fn(*const std::ffi::c_void, *mut usize) -> bool>,
        expected_buff: Option<unsafe extern "C" fn(*const std::ffi::c_void, *const StatV1, *mut BuffV1) -> bool>,
        expected_move_distance: Option<unsafe extern "C" fn(*const std::ffi::c_void, *mut usize, *mut u64) -> bool>,
        expected_rush_effect: Option<unsafe extern "C" fn(*const std::ffi::c_void) -> bool>,
        auto_target: Option<unsafe extern "C" fn(*const std::ffi::c_void) -> bool>,
        on_caster: Option<unsafe extern "C" fn(*const std::ffi::c_void) -> bool>,
        can_move: Option<unsafe extern "C" fn(*const std::ffi::c_void) -> bool>,
        linear_move_speed: Option<unsafe extern "C" fn(*const std::ffi::c_void, *mut usize) -> bool>,
    });

    dump_struct!(out, PassiveVtableV1 {
        size: usize,
        destroy: Option<unsafe extern "C" fn(*mut std::ffi::c_void)>,
        clone: Option<unsafe extern "C" fn(*const std::ffi::c_void) -> *mut std::ffi::c_void>,
        on_spawn: Option<unsafe extern "C" fn(*mut std::ffi::c_void, *mut SimCtxV1, usize, usize)>,
        on_attack: Option<unsafe extern "C" fn(*mut std::ffi::c_void, *mut SimCtxV1, usize, usize, usize, *mut usize)>,
        on_damaged: Option<unsafe extern "C" fn(*mut std::ffi::c_void, *mut SimCtxV1, usize, usize, usize, usize)>,
        on_kill: Option<unsafe extern "C" fn(*mut std::ffi::c_void, *mut SimCtxV1, usize, usize)>,
        on_update: Option<unsafe extern "C" fn(*mut std::ffi::c_void, *mut SimCtxV1, u64, usize, usize)>,
        on_base_attack: Option<unsafe extern "C" fn(*mut std::ffi::c_void, *mut SimCtxV1, u64, usize, usize)>,
        on_assist: Option<unsafe extern "C" fn(*mut std::ffi::c_void, *mut SimCtxV1, usize, usize)>,
        on_dead: Option<unsafe extern "C" fn(*mut std::ffi::c_void, *mut SimCtxV1, usize)>,
        on_kill_ex: Option<unsafe extern "C" fn(*mut std::ffi::c_void, *mut SimCtxV1, usize, usize, usize)>,
    });

    dump_struct!(out, ItemVtableV1 {
        size: usize,
        destroy: Option<unsafe extern "C" fn(*mut std::ffi::c_void)>,
        clone: Option<unsafe extern "C" fn(*const std::ffi::c_void) -> *mut std::ffi::c_void>,
        key: Option<unsafe extern "C" fn(*const std::ffi::c_void) -> StrV1>,
        icon: Option<unsafe extern "C" fn(*const std::ffi::c_void) -> StrV1>,
        price: Option<unsafe extern "C" fn(*const std::ffi::c_void) -> usize>,
        tier: Option<unsafe extern "C" fn(*const std::ffi::c_void) -> usize>,
        stat: Option<unsafe extern "C" fn(*const std::ffi::c_void, *mut BuffV1)>,
        next_tier: Option<unsafe extern "C" fn(*const std::ffi::c_void, *mut StrV1, usize) -> usize>,
        previous_tier: Option<unsafe extern "C" fn(*const std::ffi::c_void, *mut StrV1, usize) -> usize>,
        tags: Option<unsafe extern "C" fn(*const std::ffi::c_void, *mut u32, usize) -> usize>,
        category: Option<unsafe extern "C" fn(*const std::ffi::c_void) -> u32>,
        on_attack: Option<unsafe extern "C" fn(*mut std::ffi::c_void, *mut SimCtxV1, usize, usize, *mut usize, u32)>,
        update: Option<unsafe extern "C" fn(*mut std::ffi::c_void, *mut SimCtxV1, u64, usize)>,
        on_spawn: Option<unsafe extern "C" fn(*mut std::ffi::c_void, *mut SimCtxV1, usize)>,
        on_healed: Option<unsafe extern "C" fn(*mut std::ffi::c_void, *mut SimCtxV1, usize, bool, usize, usize)>,
        on_damaged: Option<unsafe extern "C" fn(*mut std::ffi::c_void, *mut SimCtxV1, usize, usize, usize, usize)>,
        on_kill: Option<unsafe extern "C" fn(*mut std::ffi::c_void, *mut SimCtxV1, u64, usize, usize)>,
        on_skill_hit: Option<unsafe extern "C" fn(*mut std::ffi::c_void, *mut SimCtxV1, u64, usize, usize)>,
        on_attack_ex: Option<unsafe extern "C" fn(*mut std::ffi::c_void, *mut SimCtxV1, usize, usize, *mut usize, u32, u32, bool)>,
        on_damaged_ex: Option<unsafe extern "C" fn(*mut std::ffi::c_void, *mut SimCtxV1, usize, usize, usize, usize, u32, u32, bool)>,
        on_kill_ex: Option<unsafe extern "C" fn(*mut std::ffi::c_void, *mut SimCtxV1, u64, usize, usize, usize)>,
        on_skill_hit_ex: Option<unsafe extern "C" fn(*mut std::ffi::c_void, *mut SimCtxV1, u64, usize, usize, bool)>,
        on_base_attack: Option<unsafe extern "C" fn(*mut std::ffi::c_void, *mut SimCtxV1, u64, usize, usize)>,
        on_assist: Option<unsafe extern "C" fn(*mut std::ffi::c_void, *mut SimCtxV1, usize, usize)>,
        on_dead: Option<unsafe extern "C" fn(*mut std::ffi::c_void, *mut SimCtxV1, usize)>,
        on_cc: Option<unsafe extern "C" fn(*mut std::ffi::c_void, *mut SimCtxV1, u64, usize, usize)>,
        on_upgrade: Option<unsafe extern "C" fn(*mut std::ffi::c_void, StrV1) -> u64>,
        on_upgraded_from: Option<unsafe extern "C" fn(*mut std::ffi::c_void, StrV1, u64)>,
    });

    dump_struct!(out, SimVtableV1 {
        size: usize,
        tick: Option<unsafe extern "C" fn(*const std::ffi::c_void) -> usize>,
        seed: Option<unsafe extern "C" fn(*const std::ffi::c_void) -> u64>,
        score_diff: Option<unsafe extern "C" fn(*const std::ffi::c_void, usize) -> i32>,
        is_end: Option<unsafe extern "C" fn(*const std::ffi::c_void) -> bool>,
        entity_count: Option<unsafe extern "C" fn(*const std::ffi::c_void) -> usize>,
        entity_at: Option<unsafe extern "C" fn(*const std::ffi::c_void, usize) -> EntityHandleV1>,
        entity_is_valid: Option<unsafe extern "C" fn(*const std::ffi::c_void, EntityHandleV1) -> bool>,
        entity_stat: Option<unsafe extern "C" fn(*const std::ffi::c_void, EntityHandleV1, *mut StatV1) -> bool>,
        entity_pos: Option<unsafe extern "C" fn(*const std::ffi::c_void, EntityHandleV1, *mut u64, *mut u64) -> bool>,
        entity_hp: Option<unsafe extern "C" fn(*const std::ffi::c_void, EntityHandleV1, *mut usize, *mut usize) -> bool>,
        entity_team: Option<unsafe extern "C" fn(*const std::ffi::c_void, EntityHandleV1) -> usize>,
        entity_level: Option<unsafe extern "C" fn(*const std::ffi::c_void, EntityHandleV1) -> usize>,
        entity_is_alive: Option<unsafe extern "C" fn(*const std::ffi::c_void, EntityHandleV1) -> bool>,
        entity_is_champion: Option<unsafe extern "C" fn(*const std::ffi::c_void, EntityHandleV1) -> bool>,
        entity_is_tower: Option<unsafe extern "C" fn(*const std::ffi::c_void, EntityHandleV1) -> bool>,
        entity_is_minion: Option<unsafe extern "C" fn(*const std::ffi::c_void, EntityHandleV1) -> bool>,
        entity_shield: Option<unsafe extern "C" fn(*const std::ffi::c_void, EntityHandleV1) -> usize>,
        entity_radius: Option<unsafe extern "C" fn(*const std::ffi::c_void, EntityHandleV1) -> usize>,
        entity_is_targetable: Option<unsafe extern "C" fn(*const std::ffi::c_void, EntityHandleV1) -> bool>,
        entity_buff_count: Option<unsafe extern "C" fn(*const std::ffi::c_void, EntityHandleV1) -> usize>,
        entity_buff_at: Option<unsafe extern "C" fn(*const std::ffi::c_void, EntityHandleV1, usize, *mut BuffV1) -> bool>,
        entity_cc_count: Option<unsafe extern "C" fn(*const std::ffi::c_void, EntityHandleV1) -> usize>,
        entity_cc_at: Option<unsafe extern "C" fn(*const std::ffi::c_void, EntityHandleV1, usize, *mut CcV1) -> bool>,
        player_count: Option<unsafe extern "C" fn(*const std::ffi::c_void) -> usize>,
        player_at: Option<unsafe extern "C" fn(*const std::ffi::c_void, usize) -> PlayerHandleV1>,
        player_is_valid: Option<unsafe extern "C" fn(*const std::ffi::c_void, PlayerHandleV1) -> bool>,
        player_champion: Option<unsafe extern "C" fn(*const std::ffi::c_void, PlayerHandleV1) -> EntityHandleV1>,
        player_level: Option<unsafe extern "C" fn(*const std::ffi::c_void, PlayerHandleV1) -> usize>,
        player_gold: Option<unsafe extern "C" fn(*const std::ffi::c_void, PlayerHandleV1) -> usize>,
        player_lane: Option<unsafe extern "C" fn(*const std::ffi::c_void, PlayerHandleV1) -> u32>,
        player_team: Option<unsafe extern "C" fn(*const std::ffi::c_void, PlayerHandleV1) -> usize>,
        player_is_alive: Option<unsafe extern "C" fn(*const std::ffi::c_void, PlayerHandleV1) -> bool>,
        player_respawn_time: Option<unsafe extern "C" fn(*const std::ffi::c_void, PlayerHandleV1) -> usize>,
        player_kills: Option<unsafe extern "C" fn(*const std::ffi::c_void, PlayerHandleV1) -> usize>,
        player_deaths: Option<unsafe extern "C" fn(*const std::ffi::c_void, PlayerHandleV1) -> usize>,
        player_assists: Option<unsafe extern "C" fn(*const std::ffi::c_void, PlayerHandleV1) -> usize>,
        player_cs: Option<unsafe extern "C" fn(*const std::ffi::c_void, PlayerHandleV1) -> usize>,
        deal_damage: Option<unsafe extern "C" fn(*mut std::ffi::c_void, usize, usize, usize, usize, u32)>,
        heal: Option<unsafe extern "C" fn(*mut std::ffi::c_void, usize, usize, usize)>,
        add_buff: Option<unsafe extern "C" fn(*mut std::ffi::c_void, usize, *const BuffV1)>,
        apply_cc: Option<unsafe extern "C" fn(*mut std::ffi::c_void, usize, *const CcV1)>,
        distance_sq: Option<unsafe extern "C" fn(*const std::ffi::c_void, usize, usize) -> u64>,
        is_visible: Option<unsafe extern "C" fn(*const std::ffi::c_void, usize, usize) -> bool>,
        champion_count: Option<unsafe extern "C" fn(*const std::ffi::c_void) -> usize>,
        champion_id_at: Option<unsafe extern "C" fn(*const std::ffi::c_void, usize) -> usize>,
        tower_count: Option<unsafe extern "C" fn(*const std::ffi::c_void) -> usize>,
        tower_id_at: Option<unsafe extern "C" fn(*const std::ffi::c_void, usize) -> usize>,
        kill_log_count: Option<unsafe extern "C" fn(*const std::ffi::c_void) -> usize>,
        kill_log_at: Option<unsafe extern "C" fn(*const std::ffi::c_void, usize, *mut KillLogV1) -> bool>,
        projectile_count: Option<unsafe extern "C" fn(*const std::ffi::c_void) -> usize>,
        projectile_at: Option<unsafe extern "C" fn(*const std::ffi::c_void, usize) -> ProjectileHandleV1>,
        projectile_is_valid: Option<unsafe extern "C" fn(*const std::ffi::c_void, ProjectileHandleV1) -> bool>,
        projectile_info: Option<unsafe extern "C" fn(*const std::ffi::c_void, ProjectileHandleV1, *mut ProjectileInfoV1) -> bool>,
        entity_set_hp: Option<unsafe extern "C" fn(*mut std::ffi::c_void, EntityHandleV1, usize) -> bool>,
        entity_set_pos: Option<unsafe extern "C" fn(*mut std::ffi::c_void, EntityHandleV1, u64, u64) -> bool>,
        entity_set_base_stat: Option<unsafe extern "C" fn(*mut std::ffi::c_void, EntityHandleV1, *const StatV1) -> bool>,
        player_set_gold: Option<unsafe extern "C" fn(*mut std::ffi::c_void, PlayerHandleV1, usize) -> bool>,
        player_add_gold: Option<unsafe extern "C" fn(*mut std::ffi::c_void, PlayerHandleV1, i64) -> bool>,
        force_end: Option<unsafe extern "C" fn(*mut std::ffi::c_void, bool)>,
        player_cooldowns: Option<unsafe extern "C" fn(*const std::ffi::c_void, PlayerHandleV1, *mut usize, *mut usize, *mut usize, *mut usize) -> bool>,
        player_item_count: Option<unsafe extern "C" fn(*const std::ffi::c_void, PlayerHandleV1) -> usize>,
        player_item_key_at: Option<unsafe extern "C" fn(*const std::ffi::c_void, PlayerHandleV1, usize, *mut u8, usize, *mut usize) -> bool>,
        entity_name: Option<unsafe extern "C" fn(*const std::ffi::c_void, EntityHandleV1, *mut u8, usize, *mut usize) -> bool>,
        queue_effect: Option<unsafe extern "C" fn(*mut std::ffi::c_void, *const u8, usize, u32, usize, *const InputTargetV1, usize) -> bool>,
        spawn_unit: Option<unsafe extern "C" fn(*mut std::ffi::c_void, *const u8, usize, usize, usize, u64, u64, u64, *const StatV1, *const UnitAttackV1) -> usize>,
        spawn_projectile: Option<unsafe extern "C" fn(*mut std::ffi::c_void, *const u8, usize, *const u8, usize, *const ProjectileSpawnV1) -> bool>,
        strategy_get_json: Option<unsafe extern "C" fn(*const std::ffi::c_void, usize, *const u8, usize, *mut u8, usize, *mut usize) -> bool>,
        strategy_set_json: Option<unsafe extern "C" fn(*mut std::ffi::c_void, usize, *const u8, usize, *const u8, usize) -> bool>,
        player_statistics_json: Option<unsafe extern "C" fn(*const std::ffi::c_void, PlayerHandleV1, *const u8, usize, *mut u8, usize, *mut usize) -> bool>,
        entity_remove_buff: Option<unsafe extern "C" fn(*mut std::ffi::c_void, EntityHandleV1, *const u8, usize) -> usize>,
        entity_clear_cc: Option<unsafe extern "C" fn(*mut std::ffi::c_void, EntityHandleV1) -> usize>,
        entity_add_shield: Option<unsafe extern "C" fn(*mut std::ffi::c_void, EntityHandleV1, usize, usize) -> bool>,
        entity_clear_shield: Option<unsafe extern "C" fn(*mut std::ffi::c_void, EntityHandleV1) -> usize>,
        entity_attack_interval: Option<unsafe extern "C" fn(*const std::ffi::c_void, EntityHandleV1) -> usize>,
        entity_attack_speed_mult: Option<unsafe extern "C" fn(*const std::ffi::c_void, EntityHandleV1) -> usize>,
        deal_damage_raw: Option<unsafe extern "C" fn(*mut std::ffi::c_void, usize, usize, usize, usize, u32)>,
    });

    dump_struct!(
        out,
        UnitAttackV1 {
            attack_ratio: usize,
            attack: usize,
            range: usize,
            cooltime: usize,
            duration: usize,
            start_timing: usize,
            cancelable: bool,
            attack_type: u32,
        }
    );

    dump_struct!(
        out,
        ProjectileSpawnV1 {
            caster_id: usize,
            team: usize,
            x: u64,
            y: u64,
            radius: u64,
            speed: u64,
            move_kind: u32,
            target_id: usize,
            target_x: u64,
            target_y: u64,
            penetrate: bool,
            attack_type: u32,
            casting_type: u32,
            casting_target: u32,
        }
    );

    dump_struct!(out, FrameVtableV1 {
        size: usize,
        debug_draw_line: Option<unsafe extern "C" fn(*mut std::ffi::c_void, u64, u64, u64, u64, u32)>,
        debug_draw_circle: Option<unsafe extern "C" fn(*mut std::ffi::c_void, u64, u64, u64, u32)>,
        debug_text: Option<unsafe extern "C" fn(*mut std::ffi::c_void, u64, u64, *const u8, usize, u32)>,
    });

    dump_struct!(
        out,
        ServiceVersionV1 {
            major: u32,
            minor: u32,
            patch: u32
        }
    );
    dump_struct!(out, ModServiceV1 { data: *mut std::ffi::c_void, vtable: *const std::ffi::c_void });

    dump_struct!(out, ServiceVtableV1 {
        size: usize,
        register_service: Option<unsafe extern "C" fn(*const u8, usize, ServiceVersionV1, ModServiceV1) -> bool>,
        query_service: Option<unsafe extern "C" fn(*const u8, usize, *const u8, usize, *const u8, usize, *mut ModServiceV1) -> bool>,
    });

    dump_struct!(out, ServerCtxV1 {
        size: usize,
        vtable: *const ServerVtableV1,
        state: *mut std::ffi::c_void,
    });

    dump_struct!(out, ServerVtableV1 {
        size: usize,
        save_version: Option<unsafe extern "C" fn(*const std::ffi::c_void) -> usize>,
        save_set_version: Option<unsafe extern "C" fn(*mut std::ffi::c_void, usize) -> bool>,
        save_contains_key: Option<unsafe extern "C" fn(*const std::ffi::c_void, *const u8, usize) -> bool>,
        save_get: Option<unsafe extern "C" fn(*const std::ffi::c_void, *const u8, usize, *mut u8, usize, *mut usize) -> bool>,
        save_set: Option<unsafe extern "C" fn(*mut std::ffi::c_void, *const u8, usize, *const u8, usize) -> bool>,
        save_remove: Option<unsafe extern "C" fn(*mut std::ffi::c_void, *const u8, usize) -> bool>,
        save_key_count: Option<unsafe extern "C" fn(*const std::ffi::c_void) -> usize>,
        save_key_at: Option<unsafe extern "C" fn(*const std::ffi::c_void, usize, *mut u8, usize, *mut usize) -> bool>,
        emit_event: Option<unsafe extern "C" fn(*mut std::ffi::c_void, u32, usize, *const u8, usize, *const u8, usize) -> bool>,
        player_team_id: Option<unsafe extern "C" fn(*const std::ffi::c_void, usize, *mut usize) -> bool>,
        setting_get_json: Option<unsafe extern "C" fn(*const std::ffi::c_void, u32, *const u8, usize, *mut u8, usize, *mut usize) -> bool>,
        setting_set_json: Option<unsafe extern "C" fn(*mut std::ffi::c_void, u32, *const u8, usize, *const u8, usize) -> bool>,
        team_get_json: Option<unsafe extern "C" fn(*const std::ffi::c_void, usize, *const u8, usize, *mut u8, usize, *mut usize) -> bool>,
        team_set_json: Option<unsafe extern "C" fn(*mut std::ffi::c_void, usize, *const u8, usize, *const u8, usize) -> bool>,
        athlete_get_json: Option<unsafe extern "C" fn(*const std::ffi::c_void, usize, *const u8, usize, *mut u8, usize, *mut usize) -> bool>,
        athlete_set_json: Option<unsafe extern "C" fn(*mut std::ffi::c_void, usize, *const u8, usize, *const u8, usize) -> bool>,
        record_ids: Option<unsafe extern "C" fn(*const std::ffi::c_void, u32, *mut usize, usize) -> usize>,
        record_get_json: Option<unsafe extern "C" fn(*const std::ffi::c_void, u32, usize, *const u8, usize, *mut u8, usize, *mut usize) -> bool>,
        record_set_json: Option<unsafe extern "C" fn(*mut std::ffi::c_void, u32, usize, *const u8, usize, *const u8, usize) -> bool>,
        news_push: Option<unsafe extern "C" fn(*mut std::ffi::c_void, usize, *const u8, usize, *const u8, usize, *const u8, usize) -> bool>,
        management_event_next: Option<unsafe extern "C" fn(*const std::ffi::c_void, u64, *mut u64, *mut u32, *mut u8, usize, *mut usize) -> bool>,
        force_transfer: Option<unsafe extern "C" fn(*mut std::ffi::c_void, usize, usize, f64) -> bool>,
    });

    dump_struct!(out, ServerExtVtableV1 {
        size: usize,
        destroy: Option<unsafe extern "C" fn(*mut std::ffi::c_void)>,
        on_server_start: Option<unsafe extern "C" fn(*mut std::ffi::c_void, *mut ServerCtxV1)>,
        before_management_tick: Option<unsafe extern "C" fn(*mut std::ffi::c_void, *mut ServerCtxV1)>,
        after_management_tick: Option<unsafe extern "C" fn(*mut std::ffi::c_void, *mut ServerCtxV1)>,
        handle_command: Option<unsafe extern "C" fn(*mut std::ffi::c_void, *mut ServerCtxV1, *const u8, usize, *const u8, usize, usize, bool, usize, bool) -> u32>,
    });

    dump_struct!(out, ServerExtRegV1 { userdata: *mut std::ffi::c_void, vtable: *const ServerExtVtableV1 });

    dump_struct!(out, SaveVtableV1 {
        size: usize,
        can_write: Option<unsafe extern "C" fn(*const std::ffi::c_void) -> bool>,
        version: Option<unsafe extern "C" fn(*const std::ffi::c_void) -> usize>,
        set_version: Option<unsafe extern "C" fn(*mut std::ffi::c_void, usize) -> bool>,
        contains_key: Option<unsafe extern "C" fn(*const std::ffi::c_void, *const u8, usize) -> bool>,
        get: Option<unsafe extern "C" fn(*const std::ffi::c_void, *const u8, usize, *mut u8, usize, *mut usize) -> bool>,
        set: Option<unsafe extern "C" fn(*mut std::ffi::c_void, *const u8, usize, *const u8, usize) -> bool>,
        remove: Option<unsafe extern "C" fn(*mut std::ffi::c_void, *const u8, usize) -> bool>,
        key_count: Option<unsafe extern "C" fn(*const std::ffi::c_void) -> usize>,
        key_at: Option<unsafe extern "C" fn(*const std::ffi::c_void, usize, *mut u8, usize, *mut usize) -> bool>,
    });

    dump_struct!(out, NetVtableV1 {
        size: usize,
        send_command: Option<unsafe extern "C" fn(*mut std::ffi::c_void, *const u8, usize, *const u8, usize) -> bool>,
        take_events: Option<unsafe extern "C" fn(*mut std::ffi::c_void) -> usize>,
        taken_event_at: Option<unsafe extern "C" fn(*const std::ffi::c_void, usize, *mut u8, usize, *mut usize, *mut u8, usize, *mut usize) -> bool>,
    });

    dump_struct!(out, DataVtableV1 {
        size: usize,
        player_team_id: Option<unsafe extern "C" fn(*const std::ffi::c_void, *mut usize) -> bool>,
        team_ids: Option<unsafe extern "C" fn(*const std::ffi::c_void, *mut usize, usize) -> usize>,
        team_name: Option<unsafe extern "C" fn(*const std::ffi::c_void, usize, *mut u8, usize, *mut usize) -> bool>,
        athlete_ids: Option<unsafe extern "C" fn(*const std::ffi::c_void, *mut usize, usize) -> usize>,
        athlete_name: Option<unsafe extern "C" fn(*const std::ffi::c_void, usize, *mut u8, usize, *mut usize) -> bool>,
        setting_get_json: Option<unsafe extern "C" fn(*const std::ffi::c_void, u32, *const u8, usize, *mut u8, usize, *mut usize) -> bool>,
        record_ids: Option<unsafe extern "C" fn(*const std::ffi::c_void, u32, *mut usize, usize) -> usize>,
        record_get_json: Option<unsafe extern "C" fn(*const std::ffi::c_void, u32, usize, *const u8, usize, *mut u8, usize, *mut usize) -> bool>,
        game_time: Option<unsafe extern "C" fn(*const std::ffi::c_void, *mut i32, *mut u32, *mut u32, *mut u32, *mut u32) -> bool>,
        head_to_head: Option<unsafe extern "C" fn(*const std::ffi::c_void, usize, *mut usize, *mut usize) -> bool>,
        competition_result: Option<unsafe extern "C" fn(*const std::ffi::c_void, bool, usize, *mut usize, *mut usize) -> bool>,
        schedule_get_json: Option<unsafe extern "C" fn(*const std::ffi::c_void, *const u8, usize, *mut u8, usize, *mut usize) -> bool>,
        champion_count: Option<unsafe extern "C" fn(*const std::ffi::c_void) -> usize>,
        champion_name_at: Option<unsafe extern "C" fn(*const std::ffi::c_void, usize, *mut u8, usize, *mut usize) -> bool>,
        champion_brief: Option<unsafe extern "C" fn(*const std::ffi::c_void, *const u8, usize, *mut ChampionBriefV1) -> bool>,
        client_scene_kind: Option<unsafe extern "C" fn(*const std::ffi::c_void) -> u32>,
        client_main_tab_name: Option<unsafe extern "C" fn(*const std::ffi::c_void, *mut u8, usize, *mut usize) -> bool>,
    });

    dump_struct!(out, UiVtableV1 {
        size: usize,
        query_exists: Option<unsafe extern "C" fn(*const std::ffi::c_void, *const u8, usize) -> bool>,
        get_visible: Option<unsafe extern "C" fn(*const std::ffi::c_void, *const u8, usize, *mut bool) -> bool>,
        set_visible: Option<unsafe extern "C" fn(*mut std::ffi::c_void, *const u8, usize, bool) -> bool>,
        spawn_template: Option<unsafe extern "C" fn(*mut std::ffi::c_void, *const u8, usize, *const u8, usize, bool) -> bool>,
        register_click: Option<unsafe extern "C" fn(*mut std::ffi::c_void, *const u8, usize, *const u8, usize, Option<UiClickHandlerFn>, *mut std::ffi::c_void) -> bool>,
        get_text: Option<unsafe extern "C" fn(*const std::ffi::c_void, *const u8, usize, *mut u8, usize, *mut usize) -> bool>,
        set_text: Option<unsafe extern "C" fn(*mut std::ffi::c_void, *const u8, usize, *const u8, usize) -> bool>,
        register_path_events: Option<unsafe extern "C" fn(*mut std::ffi::c_void, *const u8, usize, Option<UiClickHandlerFn>, *mut std::ffi::c_void) -> bool>,
        current_event: Option<unsafe extern "C" fn(*const std::ffi::c_void, *mut u32, *mut u8, usize, *mut usize, *mut u8, usize, *mut usize) -> bool>,
        spawn_source: Option<unsafe extern "C" fn(*mut std::ffi::c_void, *const u8, usize, *const u8, usize) -> bool>,
        set_properties: Option<unsafe extern "C" fn(*mut std::ffi::c_void, *const u8, usize, *const u8, usize) -> bool>,
        remove_node: Option<unsafe extern "C" fn(*mut std::ffi::c_void, *const u8, usize) -> bool>,
        runner_name: Option<unsafe extern "C" fn(*const std::ffi::c_void, *const u8, usize, *mut u8, usize, *mut usize) -> bool>,
        state_get_json: Option<unsafe extern "C" fn(*const std::ffi::c_void, *const u8, usize, *mut u8, usize, *mut usize) -> bool>,
        state_set_json: Option<unsafe extern "C" fn(*mut std::ffi::c_void, *const u8, usize, *const u8, usize) -> bool>,
        node_rect: Option<unsafe extern "C" fn(*const std::ffi::c_void, *const u8, usize, bool, *mut f32, *mut f32, *mut f32, *mut f32) -> bool>,
        child_count: Option<unsafe extern "C" fn(*const std::ffi::c_void, *const u8, usize, *mut usize) -> bool>,
        child_name_at: Option<unsafe extern "C" fn(*const std::ffi::c_void, *const u8, usize, usize, *mut u8, usize, *mut usize) -> bool>,
        input_event_count: Option<unsafe extern "C" fn(*const std::ffi::c_void) -> usize>,
        input_event_at: Option<unsafe extern "C" fn(*const std::ffi::c_void, usize, *mut u32, *mut u8, usize, *mut usize) -> bool>,
    });

    dump_struct!(out, SceneVtableV1 {
        size: usize,
        kind: Option<unsafe extern "C" fn(*const std::ffi::c_void) -> u32>,
        new_game_team_id: Option<unsafe extern "C" fn(*const std::ffi::c_void, *mut usize) -> bool>,
        new_game_set_team_id: Option<unsafe extern "C" fn(*mut std::ffi::c_void, usize) -> bool>,
        new_game_with_tutorial: Option<unsafe extern "C" fn(*const std::ffi::c_void, *mut bool) -> bool>,
        new_game_set_with_tutorial: Option<unsafe extern "C" fn(*mut std::ffi::c_void, bool) -> bool>,
        new_game_manager_name: Option<unsafe extern "C" fn(*const std::ffi::c_void, *mut u8, usize, *mut usize) -> bool>,
        new_game_set_manager_name: Option<unsafe extern "C" fn(*mut std::ffi::c_void, *const u8, usize) -> bool>,
        new_game_option_get_json: Option<unsafe extern "C" fn(*const std::ffi::c_void, *const u8, usize, *mut u8, usize, *mut usize) -> bool>,
        new_game_option_set_json: Option<unsafe extern "C" fn(*mut std::ffi::c_void, *const u8, usize, *const u8, usize) -> bool>,
    });

    dump_struct!(out, AssetVtableV1 {
        size: usize,
        i18n_get: Option<unsafe extern "C" fn(*const std::ffi::c_void, *const u8, usize, *mut u8, usize, *mut usize) -> bool>,
        play_sound: Option<unsafe extern "C" fn(*mut std::ffi::c_void, *const u8, usize, f32) -> bool>,
        play_bgm: Option<unsafe extern "C" fn(*mut std::ffi::c_void, *const StrV1, usize) -> bool>,
        stop_bgm: Option<unsafe extern "C" fn(*mut std::ffi::c_void) -> bool>,
    });

    dump_struct!(out, DrawVtableV1 {
        size: usize,
        can_draw: Option<unsafe extern "C" fn(*const std::ffi::c_void) -> bool>,
        map_size: Option<unsafe extern "C" fn(*const std::ffi::c_void, *const u8, usize, *mut f32, *mut f32) -> bool>,
        set_camera: Option<unsafe extern "C" fn(*mut std::ffi::c_void, *const u8, usize, f32, f32, f32, f32) -> bool>,
        rect: Option<unsafe extern "C" fn(*mut std::ffi::c_void, *const u8, usize, f32, f32, f32, f32, i32, f32, u32) -> bool>,
        circle: Option<unsafe extern "C" fn(*mut std::ffi::c_void, *const u8, usize, f32, f32, f32, i32, u32) -> bool>,
        line: Option<unsafe extern "C" fn(*mut std::ffi::c_void, *const u8, usize, f32, f32, f32, f32, f32, i32, u32) -> bool>,
        text: Option<unsafe extern "C" fn(*mut std::ffi::c_void, *const u8, usize, *const u8, usize, *const u8, usize, f32, f32, f32, f32, i32, f32, u32, u32, u32) -> bool>,
        svg: Option<unsafe extern "C" fn(*mut std::ffi::c_void, *const u8, usize, *const u8, usize, f32, f32, f32, f32, i32, u32) -> bool>,
        sprite: Option<unsafe extern "C" fn(*mut std::ffi::c_void, *const u8, usize, *const u8, usize, f32, f32, i32, f32, bool, bool, f32, f32, f32, f32, f32, f32, bool) -> bool>,
    });

    dump_struct!(out, ClientCtxV1 {
        size: usize,
        ui: *const UiVtableV1,
        scene: *const SceneVtableV1,
        asset: *const AssetVtableV1,
        save: *const SaveVtableV1,
        net: *const NetVtableV1,
        state: *mut std::ffi::c_void,
        data: *const DataVtableV1,
        draw: *const DrawVtableV1,
    });

    dump_struct!(out, ExtensionVtableV1 {
        size: usize,
        destroy: Option<unsafe extern "C" fn(*mut std::ffi::c_void)>,
        on_init: Option<unsafe extern "C" fn(*mut std::ffi::c_void, *mut ClientCtxV1)>,
        pre_update: Option<unsafe extern "C" fn(*mut std::ffi::c_void, *mut ClientCtxV1, u64)>,
        post_update: Option<unsafe extern "C" fn(*mut std::ffi::c_void, *mut ClientCtxV1, u64)>,
        on_end: Option<unsafe extern "C" fn(*mut std::ffi::c_void)>,
        pre_render: Option<unsafe extern "C" fn(*mut std::ffi::c_void, *mut ClientCtxV1)>,
        post_render: Option<unsafe extern "C" fn(*mut std::ffi::c_void, *mut ClientCtxV1)>,
    });

    dump_struct!(out, ExtensionRegV1 { userdata: *mut std::ffi::c_void, vtable: *const ExtensionVtableV1 });

    dump_struct!(
        out,
        InputV1 {
            kind: u32,
            target: InputTargetV1,
            x: u64,
            y: u64
        }
    );

    out.push_str(&format!(
        "const CHAMPION_BRIEF_TAG_CAP = {}\n",
        CHAMPION_BRIEF_TAG_CAP
    ));

    dump_struct!(
        out,
        ChampionBriefV1 {
            name: StrV1,
            category: u32,
            tag_count: u32,
            tags: [u32; CHAMPION_BRIEF_TAG_CAP],
            stat: StatV1,
            growth: StatV1,
        }
    );

    dump_struct!(out, DraftCtxV1 {
        size: usize,
        phase: u32,
        difficulty: u32,
        is_explore: bool,
        available_ptr: *const usize,
        available_len: usize,
        ally_ban_ptr: *const usize,
        ally_ban_len: usize,
        enemy_ban_ptr: *const usize,
        enemy_ban_len: usize,
        ally_pick_ptr: *const usize,
        ally_pick_len: usize,
        enemy_pick_ptr: *const usize,
        enemy_pick_len: usize,
        champion_briefs_ptr: *const ChampionBriefV1,
        champion_briefs_len: usize,
    });

    dump_struct!(out, DraftHookVtableV1 {
        size: usize,
        destroy: Option<unsafe extern "C" fn(*mut std::ffi::c_void)>,
        id: Option<unsafe extern "C" fn(*const std::ffi::c_void) -> StrV1>,
        priority: Option<unsafe extern "C" fn(*const std::ffi::c_void) -> i32>,
        score_ban: Option<unsafe extern "C" fn(*const std::ffi::c_void, *const DraftCtxV1, usize, f32, *mut f32) -> u32>,
        score_pick: Option<unsafe extern "C" fn(*const std::ffi::c_void, *const DraftCtxV1, usize, f32, *mut f32) -> u32>,
        decide_ban: Option<unsafe extern "C" fn(*const std::ffi::c_void, *const DraftCtxV1, *mut usize) -> bool>,
        decide_pick: Option<unsafe extern "C" fn(*const std::ffi::c_void, *const DraftCtxV1, *mut usize) -> bool>,
    });

    dump_struct!(out, ItemBuildCtxV1 {
        size: usize,
        team: usize,
        lane: u32,
        champion_key: StrV1,
        item_keys_ptr: *const StrV1,
        item_keys_len: usize,
        item_categories_ptr: *const u32,
        item_tiers_ptr: *const usize,
        base_build_ptr: *const usize,
        base_build_len: usize,
        ally_champions_ptr: *const StrV1,
        ally_champions_len: usize,
        enemy_champions_ptr: *const StrV1,
        enemy_champions_len: usize,
    });

    dump_struct!(out, ItemBuildHookVtableV1 {
        size: usize,
        destroy: Option<unsafe extern "C" fn(*mut std::ffi::c_void)>,
        id: Option<unsafe extern "C" fn(*const std::ffi::c_void) -> StrV1>,
        priority: Option<unsafe extern "C" fn(*const std::ffi::c_void) -> i32>,
        score_item: Option<unsafe extern "C" fn(*const std::ffi::c_void, *const ItemBuildCtxV1, usize, f32, *mut f32) -> u32>,
        decide_build: Option<unsafe extern "C" fn(*const std::ffi::c_void, *const ItemBuildCtxV1, *mut usize, usize) -> usize>,
    });

    dump_struct!(out, ItemBuildHookRegV1 {
        userdata: *mut std::ffi::c_void,
        vtable: *const ItemBuildHookVtableV1,
    });

    dump_struct!(out, DraftHookRegV1 { userdata: *mut std::ffi::c_void, vtable: *const DraftHookVtableV1 });

    dump_struct!(
        out,
        AiInitV1 {
            size: usize,
            player_id: usize,
            athlete_id: usize,
            team: usize,
            lane: u32,
            champion_name: StrV1,
        }
    );

    dump_struct!(out, AiCtxV1 {
        size: usize,
        vtable: *const AiVtableV1,
        state: *mut std::ffi::c_void,
        sim: *const SimVtableV1,
        sim_state: *mut std::ffi::c_void,
    });

    dump_struct!(out, AiVtableV1 {
        size: usize,
        player_id: Option<unsafe extern "C" fn(*const std::ffi::c_void) -> usize>,
        athlete_id: Option<unsafe extern "C" fn(*const std::ffi::c_void) -> usize>,
        team: Option<unsafe extern "C" fn(*const std::ffi::c_void) -> usize>,
        lane: Option<unsafe extern "C" fn(*const std::ffi::c_void) -> u32>,
        champion_name: Option<unsafe extern "C" fn(*const std::ffi::c_void, *mut u8, usize, *mut usize) -> bool>,
        tick: Option<unsafe extern "C" fn(*const std::ffi::c_void) -> usize>,
        hp: Option<unsafe extern "C" fn(*const std::ffi::c_void, *mut usize) -> bool>,
        max_hp: Option<unsafe extern "C" fn(*const std::ffi::c_void, *mut usize) -> bool>,
        hp_ratio_percent: Option<unsafe extern "C" fn(*const std::ffi::c_void, *mut usize) -> bool>,
        is_valid_input: Option<unsafe extern "C" fn(*const std::ffi::c_void, *const InputV1) -> bool>,
        run_away_input: Option<unsafe extern "C" fn(*mut std::ffi::c_void, *mut InputV1) -> bool>,
        run_away_without_skill_input: Option<unsafe extern "C" fn(*mut std::ffi::c_void, *mut InputV1) -> bool>,
        recall_input: Option<unsafe extern "C" fn(*mut std::ffi::c_void, *mut InputV1) -> bool>,
        is_safe_to_recall: Option<unsafe extern "C" fn(*mut std::ffi::c_void) -> bool>,
    });

    dump_struct!(out, PlayerAiVtableV1 {
        size: usize,
        destroy: Option<unsafe extern "C" fn(*mut std::ffi::c_void)>,
        clone: Option<unsafe extern "C" fn(*const std::ffi::c_void) -> *mut std::ffi::c_void>,
        id: Option<unsafe extern "C" fn(*const std::ffi::c_void) -> StrV1>,
        priority: Option<unsafe extern "C" fn(*const std::ffi::c_void) -> i32>,
        matches: Option<unsafe extern "C" fn(*const std::ffi::c_void, *const AiInitV1) -> bool>,
        think: Option<unsafe extern "C" fn(*mut std::ffi::c_void, *mut AiCtxV1, *const InputV1, *mut InputV1) -> u32>,
    });

    dump_struct!(out, PlayerAiRegV1 { userdata: *mut std::ffi::c_void, vtable: *const PlayerAiVtableV1 });

    dump_struct!(out, JsonDocVtableV1 {
        size: usize,
        get_json: Option<unsafe extern "C" fn(*const std::ffi::c_void, *const u8, usize, *mut u8, usize, *mut usize) -> bool>,
        set_json: Option<unsafe extern "C" fn(*mut std::ffi::c_void, *const u8, usize, *const u8, usize) -> bool>,
    });

    dump_struct!(out, JsonDocCtxV1 {
        size: usize,
        vtable: *const JsonDocVtableV1,
        state: *mut std::ffi::c_void,
    });

    dump_struct!(out, MapCustomizerVtableV1 {
        size: usize,
        destroy: Option<unsafe extern "C" fn(*mut std::ffi::c_void)>,
        customize: Option<unsafe extern "C" fn(*mut std::ffi::c_void, u32, *mut JsonDocCtxV1)>,
    });

    dump_struct!(out, MapCustomizerRegV1 { userdata: *mut std::ffi::c_void, vtable: *const MapCustomizerVtableV1 });

    dump_struct!(out, MatchHookVtableV1 {
        size: usize,
        destroy: Option<unsafe extern "C" fn(*mut std::ffi::c_void)>,
        on_match_start: Option<unsafe extern "C" fn(*mut std::ffi::c_void, *mut SimCtxV1)>,
        on_match_tick: Option<unsafe extern "C" fn(*mut std::ffi::c_void, *mut SimCtxV1, u64)>,
        check_match_end: Option<unsafe extern "C" fn(*mut std::ffi::c_void, *mut SimCtxV1, *mut bool) -> bool>,
    });

    dump_struct!(out, MatchHookRegV1 { userdata: *mut std::ffi::c_void, vtable: *const MatchHookVtableV1 });

    dump_struct!(out, HostApiV1 {
        size: usize,
        host_abi_level: u32,
        game_version: GameVersionV1,
        log: Option<unsafe extern "C" fn(u32, *const u8, usize)>,
        sim: *const SimVtableV1,
        frame: *const FrameVtableV1,
        service: *const ServiceVtableV1,
        save: *const SaveVtableV1,
        net: *const NetVtableV1,
        data: *const DataVtableV1,
        ui: *const UiVtableV1,
        scene: *const SceneVtableV1,
        asset: *const AssetVtableV1,
    });

    dump_struct!(out, ModExportV1 {
        size: usize,
        required_abi_level: u32,
        mod_id_ptr: *const u8,
        mod_id_len: usize,
        destroy: Option<unsafe extern "C" fn(*mut ModExportV1)>,
        champions_ptr: *const ChampionRegV1,
        champions_len: usize,
        items_ptr: *const ItemRegV1,
        items_len: usize,
        native_effects_ptr: *const NativeEffectRegV1,
        native_effects_len: usize,
        server_ext: ServerExtRegV1,
        extension: ExtensionRegV1,
        draft_hooks_ptr: *const DraftHookRegV1,
        draft_hooks_len: usize,
        player_ai_ptr: *const PlayerAiRegV1,
        player_ai_len: usize,
        map_customizer: MapCustomizerRegV1,
        match_hook: MatchHookRegV1,
        item_build_hooks_ptr: *const ItemBuildHookRegV1,
        item_build_hooks_len: usize,
    });

    out
}
