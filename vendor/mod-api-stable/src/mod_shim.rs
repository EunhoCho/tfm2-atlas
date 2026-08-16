//! Mod-side shims: convert `Stable*` trait objects into boundary vtables.
//! Compiled into the mod DLL by way of the `StableMod` builder; modders never
//! touch this directly.
//!
//! Rules enforced here:
//! - every extern fn catch-unwinds (a panicking extern "C" fn would abort);
//! - string getters return views into caches owned by the shim container so
//!   they stay valid until `destroy`;
//! - `destroy`/`clone` run in this DLL, so allocation ownership never
//!   crosses the boundary.

use std::ffi::c_void;
use std::mem::size_of;
use std::panic::{catch_unwind, AssertUnwindSafe};

use crate::{
    ActionRegV1, ActionVtableV1, AiCtxV1, AiDecisionKindV1, AiInitV1, BuffV1, ChampionRegV1,
    ChampionVtableV1, ClientCtxV1, CommandResultV1, DraftCtxV1, DraftDecisionKindV1,
    DraftHookRegV1, DraftHookVtableV1, EffectSpecV1, EffectTypeRegV1, EffectTypeVtableV1,
    ExtensionRegV1, ExtensionVtableV1, InputTargetV1, InputV1, ItemRegV1, ItemVtableV1,
    PassiveRegV1, PassiveVtableV1, PlayerAiRegV1, PlayerAiVtableV1, ServerCtxV1, ServerExtRegV1,
    ServerExtVtableV1, SimCtxV1, StableAction, StableAiContext, StableAiInit, StableChampion,
    StableClient, StableCommand, StableDraftContext, StableDraftDecision, StableDraftHook,
    StableEffectType, StableExtension, StableItem, StablePassive, StablePlayerAi, StableServerCtx,
    StableServerExtension, StableSim, StatV1, StrV1,
};

/// catch_unwind with a default result — every extern shim body goes through
/// this so a mod panic degrades instead of aborting the process.
fn guarded<T>(default: T, body: impl FnOnce() -> T) -> T {
    catch_unwind(AssertUnwindSafe(body)).unwrap_or(default)
}

const SKILL_ICON_SLOTS: usize = 4;

// ---------------------------------------------------------------------------
// Champion
// ---------------------------------------------------------------------------

struct ChampionShim {
    inner: Box<dyn StableChampion>,
    id: String,
    name: String,
    icons: Vec<(String, String)>,
}

unsafe fn champion<'a>(userdata: *const c_void) -> &'a ChampionShim {
    &*(userdata as *const ChampionShim)
}

unsafe extern "C" fn champion_destroy(userdata: *mut c_void) {
    guarded((), || {
        drop(unsafe { Box::from_raw(userdata as *mut ChampionShim) })
    });
}

unsafe extern "C" fn champion_id(userdata: *const c_void) -> StrV1 {
    guarded(StrV1::EMPTY, || StrV1::from_str(&champion(userdata).id))
}

unsafe extern "C" fn champion_name(userdata: *const c_void) -> StrV1 {
    guarded(StrV1::EMPTY, || StrV1::from_str(&champion(userdata).name))
}

unsafe extern "C" fn champion_skill_icon(
    userdata: *const c_void,
    skill_index: usize,
    out_source: *mut StrV1,
    out_tag: *mut StrV1,
) -> bool {
    guarded(false, || {
        let shim = champion(userdata);
        let Some((source, tag)) = shim.icons.get(skill_index) else {
            return false;
        };
        *out_source = StrV1::from_str(source);
        *out_tag = StrV1::from_str(tag);
        true
    })
}

unsafe extern "C" fn champion_category(userdata: *const c_void) -> u32 {
    guarded(0, || champion(userdata).inner.category().code())
}

unsafe extern "C" fn champion_tags(userdata: *const c_void, out: *mut u32, cap: usize) -> usize {
    guarded(0, || {
        let tags = champion(userdata).inner.tags();
        for (i, tag) in tags.iter().take(cap).enumerate() {
            *out.add(i) = tag.code();
        }
        tags.len()
    })
}

unsafe extern "C" fn champion_stat(userdata: *const c_void, out: *mut StatV1) {
    guarded((), || *out = champion(userdata).inner.stat());
}

unsafe extern "C" fn champion_growth(userdata: *const c_void, out: *mut StatV1) {
    guarded((), || *out = champion(userdata).inner.growth());
}

unsafe extern "C" fn champion_attack(userdata: *const c_void, out: *mut ActionRegV1) -> bool {
    guarded(false, || {
        *out = action_to_reg(champion(userdata).inner.attack());
        true
    })
}

unsafe extern "C" fn champion_skill(userdata: *const c_void, out: *mut ActionRegV1) -> bool {
    guarded(false, || {
        *out = action_to_reg(champion(userdata).inner.skill());
        true
    })
}

unsafe extern "C" fn champion_skill2(userdata: *const c_void, out: *mut ActionRegV1) -> bool {
    guarded(false, || {
        *out = action_to_reg(champion(userdata).inner.skill2());
        true
    })
}

unsafe extern "C" fn champion_ult(userdata: *const c_void, out: *mut ActionRegV1) -> bool {
    guarded(false, || match champion(userdata).inner.ult() {
        Some(action) => {
            *out = action_to_reg(action);
            true
        }
        None => false,
    })
}

unsafe extern "C" fn champion_passive(userdata: *const c_void, out: *mut PassiveRegV1) -> bool {
    guarded(false, || match champion(userdata).inner.passive() {
        Some(passive) => {
            *out = passive_to_reg(passive);
            true
        }
        None => false,
    })
}

unsafe extern "C" fn champion_lane_prior(
    userdata: *const c_void,
    out: *mut usize,
    cap: usize,
) -> bool {
    guarded(false, || {
        let Some(prior) = champion(userdata).inner.lane_prior() else {
            return false;
        };
        if out.is_null() || cap == 0 {
            return false;
        }
        for i in 0..cap {
            unsafe { *out.add(i) = 0 };
        }
        for (lane, weight) in prior {
            let index = lane.code() as usize;
            if index < cap {
                unsafe { *out.add(index) = weight.min(100) };
            }
        }
        true
    })
}

static CHAMPION_VTABLE: ChampionVtableV1 = ChampionVtableV1 {
    size: size_of::<ChampionVtableV1>(),
    destroy: Some(champion_destroy),
    id: Some(champion_id),
    name: Some(champion_name),
    skill_icon: Some(champion_skill_icon),
    category: Some(champion_category),
    tags: Some(champion_tags),
    stat: Some(champion_stat),
    growth: Some(champion_growth),
    attack: Some(champion_attack),
    skill: Some(champion_skill),
    skill2: Some(champion_skill2),
    ult: Some(champion_ult),
    passive: Some(champion_passive),
    lane_prior: Some(champion_lane_prior),
};

pub(crate) fn champion_to_reg(inner: Box<dyn StableChampion>) -> ChampionRegV1 {
    let id = inner.id();
    let name = inner.name();
    let icons = (0..SKILL_ICON_SLOTS).map(|i| inner.skill_icon(i)).collect();
    let shim = Box::new(ChampionShim {
        inner,
        id,
        name,
        icons,
    });
    ChampionRegV1 {
        userdata: Box::into_raw(shim) as *mut c_void,
        vtable: &CHAMPION_VTABLE,
    }
}

// ---------------------------------------------------------------------------
// Action
// ---------------------------------------------------------------------------

struct ActionShim {
    inner: Box<dyn StableAction>,
    action_name: String,
    description: String,
}

unsafe fn action<'a>(userdata: *const c_void) -> &'a ActionShim {
    &*(userdata as *const ActionShim)
}

unsafe extern "C" fn action_destroy(userdata: *mut c_void) {
    guarded((), || {
        drop(unsafe { Box::from_raw(userdata as *mut ActionShim) })
    });
}

unsafe extern "C" fn action_clone(userdata: *const c_void) -> *mut c_void {
    guarded(std::ptr::null_mut(), || {
        let shim = action(userdata);
        let cloned = Box::new(ActionShim {
            inner: shim.inner.clone_box(),
            action_name: shim.action_name.clone(),
            description: shim.description.clone(),
        });
        Box::into_raw(cloned) as *mut c_void
    })
}

unsafe extern "C" fn action_name(userdata: *const c_void) -> StrV1 {
    guarded(StrV1::EMPTY, || {
        StrV1::from_str(&action(userdata).action_name)
    })
}

unsafe extern "C" fn action_duration(userdata: *const c_void) -> usize {
    guarded(0, || action(userdata).inner.duration())
}

unsafe extern "C" fn action_cancelable(userdata: *const c_void) -> bool {
    guarded(false, || action(userdata).inner.cancelable())
}

unsafe extern "C" fn action_cooltime(
    userdata: *const c_void,
    caster_stat: *const StatV1,
    caster_level: usize,
) -> usize {
    guarded(0, || {
        action(userdata).inner.cooltime(&*caster_stat, caster_level)
    })
}

unsafe extern "C" fn action_casting_target(userdata: *const c_void) -> u32 {
    guarded(0, || action(userdata).inner.casting_target().code())
}

unsafe extern "C" fn action_effect(userdata: *const c_void, out: *mut EffectSpecV1) -> bool {
    guarded(false, || match action(userdata).inner.effect() {
        Some(spec) => {
            *out = EffectSpecV1 {
                range: spec.range,
                growth_range: spec.growth_range,
                start_timing: spec.start_timing,
                casting: spec.casting.code(),
                target: spec.target.code(),
                attack_type: spec.attack_type.code(),
                effect: effect_type_to_reg(spec.effect),
            };
            true
        }
        None => false,
    })
}

unsafe extern "C" fn action_cooltime_use_count(
    userdata: *const c_void,
    caster_stat: *const StatV1,
) -> usize {
    guarded(1, || {
        action(userdata).inner.cooltime_use_count(&*caster_stat)
    })
}

unsafe extern "C" fn action_can_use_with_move(userdata: *const c_void) -> bool {
    guarded(false, || action(userdata).inner.can_use_with_move())
}

unsafe extern "C" fn action_description(userdata: *const c_void) -> StrV1 {
    guarded(StrV1::EMPTY, || {
        StrV1::from_str(&action(userdata).description)
    })
}

static ACTION_VTABLE: ActionVtableV1 = ActionVtableV1 {
    size: size_of::<ActionVtableV1>(),
    destroy: Some(action_destroy),
    clone: Some(action_clone),
    action_name: Some(action_name),
    duration: Some(action_duration),
    cancelable: Some(action_cancelable),
    cooltime: Some(action_cooltime),
    casting_target: Some(action_casting_target),
    effect: Some(action_effect),
    cooltime_use_count: Some(action_cooltime_use_count),
    can_use_with_move: Some(action_can_use_with_move),
    description: Some(action_description),
};

fn action_to_reg(inner: Box<dyn StableAction>) -> ActionRegV1 {
    let action_name = inner.action_name();
    let description = inner.description();
    let shim = Box::new(ActionShim {
        inner,
        action_name,
        description,
    });
    ActionRegV1 {
        userdata: Box::into_raw(shim) as *mut c_void,
        vtable: &ACTION_VTABLE,
    }
}

// ---------------------------------------------------------------------------
// EffectType
// ---------------------------------------------------------------------------

struct EffectTypeShim {
    inner: Box<dyn StableEffectType>,
}

unsafe fn effect_type<'a>(userdata: *const c_void) -> &'a EffectTypeShim {
    &*(userdata as *const EffectTypeShim)
}

unsafe extern "C" fn effect_destroy(userdata: *mut c_void) {
    guarded((), || {
        drop(unsafe { Box::from_raw(userdata as *mut EffectTypeShim) })
    });
}

unsafe extern "C" fn effect_apply(
    userdata: *mut c_void,
    sim: *mut SimCtxV1,
    rng_seed: u64,
    caster_id: usize,
    input: *const InputTargetV1,
) {
    guarded((), || {
        let Some(mut sim) = (unsafe { StableSim::from_raw(sim) }) else {
            return;
        };
        let input = if input.is_null() {
            InputTargetV1::NONE
        } else {
            *input
        };
        effect_type(userdata)
            .inner
            .apply(&mut sim, rng_seed, caster_id, input);
    });
}

unsafe extern "C" fn effect_expected_damage(
    userdata: *const c_void,
    caster_stat: *const StatV1,
    out_ad: *mut usize,
    out_ap: *mut usize,
) {
    guarded((), || {
        let (ad, ap) = effect_type(userdata).inner.expected_damage(&*caster_stat);
        *out_ad = ad;
        *out_ap = ap;
    });
}

unsafe extern "C" fn effect_expected_heal(
    userdata: *const c_void,
    caster_stat: *const StatV1,
) -> usize {
    guarded(0, || {
        effect_type(userdata).inner.expected_heal(&*caster_stat)
    })
}

unsafe extern "C" fn effect_expected_shield(
    userdata: *const c_void,
    caster_stat: *const StatV1,
) -> usize {
    guarded(0, || {
        effect_type(userdata).inner.expected_shield(&*caster_stat)
    })
}

unsafe extern "C" fn effect_expected_cc_time(userdata: *const c_void, out: *mut usize) -> bool {
    guarded(false, || {
        match effect_type(userdata).inner.expected_cc_time() {
            Some(ticks) => {
                *out = ticks;
                true
            }
            None => false,
        }
    })
}

unsafe extern "C" fn effect_expected_buff(
    userdata: *const c_void,
    caster_stat: *const StatV1,
    out: *mut BuffV1,
) -> bool {
    guarded(false, || {
        match effect_type(userdata).inner.expected_buff(&*caster_stat) {
            Some(buff) => {
                *out = buff;
                true
            }
            None => false,
        }
    })
}

unsafe extern "C" fn effect_expected_move_distance(
    userdata: *const c_void,
    out_ticks: *mut usize,
    out_distance: *mut u64,
) -> bool {
    guarded(false, || {
        match effect_type(userdata).inner.expected_move_distance() {
            Some((ticks, distance)) => {
                *out_ticks = ticks;
                *out_distance = distance;
                true
            }
            None => false,
        }
    })
}

unsafe extern "C" fn effect_expected_rush_effect(userdata: *const c_void) -> bool {
    guarded(false, || effect_type(userdata).inner.expected_rush_effect())
}

unsafe extern "C" fn effect_auto_target(userdata: *const c_void) -> bool {
    guarded(false, || effect_type(userdata).inner.auto_target())
}

unsafe extern "C" fn effect_on_caster(userdata: *const c_void) -> bool {
    guarded(false, || effect_type(userdata).inner.on_caster())
}

unsafe extern "C" fn effect_can_move(userdata: *const c_void) -> bool {
    guarded(false, || effect_type(userdata).inner.can_move())
}

unsafe extern "C" fn effect_linear_move_speed(userdata: *const c_void, out: *mut usize) -> bool {
    guarded(false, || {
        match effect_type(userdata).inner.linear_move_speed() {
            Some(speed) => {
                *out = speed;
                true
            }
            None => false,
        }
    })
}

static EFFECT_TYPE_VTABLE: EffectTypeVtableV1 = EffectTypeVtableV1 {
    size: size_of::<EffectTypeVtableV1>(),
    destroy: Some(effect_destroy),
    apply: Some(effect_apply),
    expected_damage: Some(effect_expected_damage),
    expected_heal: Some(effect_expected_heal),
    expected_shield: Some(effect_expected_shield),
    expected_cc_time: Some(effect_expected_cc_time),
    expected_buff: Some(effect_expected_buff),
    expected_move_distance: Some(effect_expected_move_distance),
    expected_rush_effect: Some(effect_expected_rush_effect),
    auto_target: Some(effect_auto_target),
    on_caster: Some(effect_on_caster),
    can_move: Some(effect_can_move),
    linear_move_speed: Some(effect_linear_move_speed),
};

pub(crate) fn effect_type_to_reg(inner: Box<dyn StableEffectType>) -> EffectTypeRegV1 {
    let shim = Box::new(EffectTypeShim { inner });
    EffectTypeRegV1 {
        userdata: Box::into_raw(shim) as *mut c_void,
        vtable: &EFFECT_TYPE_VTABLE,
    }
}

// ---------------------------------------------------------------------------
// Passive
// ---------------------------------------------------------------------------

struct PassiveShim {
    inner: Box<dyn StablePassive>,
}

unsafe fn passive_mut<'a>(userdata: *mut c_void) -> &'a mut PassiveShim {
    &mut *(userdata as *mut PassiveShim)
}

unsafe extern "C" fn passive_destroy(userdata: *mut c_void) {
    guarded((), || {
        drop(unsafe { Box::from_raw(userdata as *mut PassiveShim) })
    });
}

unsafe extern "C" fn passive_clone(userdata: *const c_void) -> *mut c_void {
    guarded(std::ptr::null_mut(), || {
        let shim = &*(userdata as *const PassiveShim);
        Box::into_raw(Box::new(PassiveShim {
            inner: shim.inner.clone_box(),
        })) as *mut c_void
    })
}

/// Wraps a passive/item callback body: recover the sim wrapper, run guarded.
macro_rules! with_sim {
    ($sim:ident, $body:expr) => {
        guarded((), || {
            let Some(mut sim_wrapper) = (unsafe { StableSim::from_raw($sim) }) else {
                return;
            };
            #[allow(clippy::redundant_closure_call)]
            ($body)(&mut sim_wrapper)
        })
    };
}

unsafe extern "C" fn passive_on_spawn(
    userdata: *mut c_void,
    sim: *mut SimCtxV1,
    player: usize,
    entity: usize,
) {
    with_sim!(sim, |s: &mut StableSim<'_>| passive_mut(userdata)
        .inner
        .on_spawn(s, player, entity));
}

unsafe extern "C" fn passive_on_attack(
    userdata: *mut c_void,
    sim: *mut SimCtxV1,
    player: usize,
    entity: usize,
    target: usize,
    damage: *mut usize,
) {
    with_sim!(sim, |s: &mut StableSim<'_>| {
        passive_mut(userdata)
            .inner
            .on_attack(s, player, entity, target, &mut *damage)
    });
}

unsafe extern "C" fn passive_on_damaged(
    userdata: *mut c_void,
    sim: *mut SimCtxV1,
    player: usize,
    entity: usize,
    attacker: usize,
    damage: usize,
) {
    with_sim!(sim, |s: &mut StableSim<'_>| {
        passive_mut(userdata)
            .inner
            .on_damaged(s, player, entity, attacker, damage)
    });
}

unsafe extern "C" fn passive_on_kill(
    userdata: *mut c_void,
    sim: *mut SimCtxV1,
    player: usize,
    entity: usize,
) {
    // Level-1 hosts have no victim to report; `usize::MAX` marks it unknown.
    with_sim!(sim, |s: &mut StableSim<'_>| {
        passive_mut(userdata)
            .inner
            .on_kill(s, player, entity, usize::MAX)
    });
}

unsafe extern "C" fn passive_on_kill_ex(
    userdata: *mut c_void,
    sim: *mut SimCtxV1,
    player: usize,
    entity: usize,
    victim: usize,
) {
    with_sim!(sim, |s: &mut StableSim<'_>| {
        passive_mut(userdata)
            .inner
            .on_kill(s, player, entity, victim)
    });
}

unsafe extern "C" fn passive_on_update(
    userdata: *mut c_void,
    sim: *mut SimCtxV1,
    rng_seed: u64,
    player: usize,
    entity: usize,
) {
    with_sim!(sim, |s: &mut StableSim<'_>| {
        passive_mut(userdata)
            .inner
            .on_update(s, rng_seed, player, entity)
    });
}

unsafe extern "C" fn passive_on_base_attack(
    userdata: *mut c_void,
    sim: *mut SimCtxV1,
    rng_seed: u64,
    player: usize,
    entity: usize,
) {
    with_sim!(sim, |s: &mut StableSim<'_>| {
        passive_mut(userdata)
            .inner
            .on_base_attack(s, rng_seed, player, entity)
    });
}

unsafe extern "C" fn passive_on_assist(
    userdata: *mut c_void,
    sim: *mut SimCtxV1,
    player: usize,
    entity: usize,
) {
    with_sim!(sim, |s: &mut StableSim<'_>| passive_mut(userdata)
        .inner
        .on_assist(s, player, entity));
}

unsafe extern "C" fn passive_on_dead(userdata: *mut c_void, sim: *mut SimCtxV1, player: usize) {
    with_sim!(sim, |s: &mut StableSim<'_>| passive_mut(userdata)
        .inner
        .on_dead(s, player));
}

static PASSIVE_VTABLE: PassiveVtableV1 = PassiveVtableV1 {
    size: size_of::<PassiveVtableV1>(),
    destroy: Some(passive_destroy),
    clone: Some(passive_clone),
    on_spawn: Some(passive_on_spawn),
    on_attack: Some(passive_on_attack),
    on_damaged: Some(passive_on_damaged),
    on_kill: Some(passive_on_kill),
    on_update: Some(passive_on_update),
    on_base_attack: Some(passive_on_base_attack),
    on_assist: Some(passive_on_assist),
    on_dead: Some(passive_on_dead),
    on_kill_ex: Some(passive_on_kill_ex),
};

fn passive_to_reg(inner: Box<dyn StablePassive>) -> PassiveRegV1 {
    PassiveRegV1 {
        userdata: Box::into_raw(Box::new(PassiveShim { inner })) as *mut c_void,
        vtable: &PASSIVE_VTABLE,
    }
}

// ---------------------------------------------------------------------------
// Item
// ---------------------------------------------------------------------------

struct ItemShim {
    inner: Box<dyn StableItem>,
    key: String,
    icon: String,
    next_tier: Vec<String>,
    previous_tier: Vec<String>,
}

unsafe fn item<'a>(userdata: *const c_void) -> &'a ItemShim {
    &*(userdata as *const ItemShim)
}

unsafe fn item_mut<'a>(userdata: *mut c_void) -> &'a mut ItemShim {
    &mut *(userdata as *mut ItemShim)
}

unsafe extern "C" fn item_destroy(userdata: *mut c_void) {
    guarded((), || {
        drop(unsafe { Box::from_raw(userdata as *mut ItemShim) })
    });
}

unsafe extern "C" fn item_clone(userdata: *const c_void) -> *mut c_void {
    guarded(std::ptr::null_mut(), || {
        let shim = item(userdata);
        let cloned = Box::new(ItemShim {
            inner: shim.inner.clone_box(),
            key: shim.key.clone(),
            icon: shim.icon.clone(),
            next_tier: shim.next_tier.clone(),
            previous_tier: shim.previous_tier.clone(),
        });
        Box::into_raw(cloned) as *mut c_void
    })
}

unsafe extern "C" fn item_key(userdata: *const c_void) -> StrV1 {
    guarded(StrV1::EMPTY, || StrV1::from_str(&item(userdata).key))
}

unsafe extern "C" fn item_icon(userdata: *const c_void) -> StrV1 {
    guarded(StrV1::EMPTY, || StrV1::from_str(&item(userdata).icon))
}

unsafe extern "C" fn item_price(userdata: *const c_void) -> usize {
    guarded(0, || item(userdata).inner.price())
}

unsafe extern "C" fn item_tier(userdata: *const c_void) -> usize {
    guarded(0, || item(userdata).inner.tier())
}

unsafe extern "C" fn item_stat(userdata: *const c_void, out: *mut BuffV1) {
    guarded((), || *out = item(userdata).inner.stat());
}

unsafe fn write_str_list(list: &[String], out: *mut StrV1, cap: usize) -> usize {
    for (i, s) in list.iter().take(cap).enumerate() {
        *out.add(i) = StrV1::from_str(s);
    }
    list.len()
}

unsafe extern "C" fn item_next_tier(userdata: *const c_void, out: *mut StrV1, cap: usize) -> usize {
    guarded(0, || write_str_list(&item(userdata).next_tier, out, cap))
}

unsafe extern "C" fn item_previous_tier(
    userdata: *const c_void,
    out: *mut StrV1,
    cap: usize,
) -> usize {
    guarded(0, || {
        write_str_list(&item(userdata).previous_tier, out, cap)
    })
}

unsafe extern "C" fn item_tags(userdata: *const c_void, out: *mut u32, cap: usize) -> usize {
    guarded(0, || {
        let tags = item(userdata).inner.tags();
        for (i, tag) in tags.iter().take(cap).enumerate() {
            *out.add(i) = tag.code();
        }
        tags.len()
    })
}

unsafe extern "C" fn item_category(userdata: *const c_void) -> u32 {
    guarded(0, || item(userdata).inner.category().code())
}

unsafe extern "C" fn item_on_attack(
    userdata: *mut c_void,
    sim: *mut SimCtxV1,
    caster: usize,
    target: usize,
    damage: *mut usize,
    damage_type: u32,
) {
    // Level-1 hosts carry no attack type or crit flag.
    with_sim!(sim, |s: &mut StableSim<'_>| {
        let damage_type =
            crate::DamageTypeV1::from_code(damage_type).unwrap_or(crate::DamageTypeV1::Ad);
        item_mut(userdata).inner.on_attack(
            s,
            caster,
            target,
            &mut *damage,
            damage_type,
            crate::AttackTypeV1::Skill,
            false,
        )
    });
}

#[allow(clippy::too_many_arguments)]
unsafe extern "C" fn item_on_attack_ex(
    userdata: *mut c_void,
    sim: *mut SimCtxV1,
    caster: usize,
    target: usize,
    damage: *mut usize,
    damage_type: u32,
    attack_type: u32,
    is_crit: bool,
) {
    with_sim!(sim, |s: &mut StableSim<'_>| {
        let damage_type =
            crate::DamageTypeV1::from_code(damage_type).unwrap_or(crate::DamageTypeV1::Ad);
        let attack_type =
            crate::AttackTypeV1::from_code(attack_type).unwrap_or(crate::AttackTypeV1::Skill);
        item_mut(userdata).inner.on_attack(
            s,
            caster,
            target,
            &mut *damage,
            damage_type,
            attack_type,
            is_crit,
        )
    });
}

unsafe extern "C" fn item_on_base_attack(
    userdata: *mut c_void,
    sim: *mut SimCtxV1,
    rng_seed: u64,
    player: usize,
    entity: usize,
) {
    with_sim!(sim, |s: &mut StableSim<'_>| {
        item_mut(userdata)
            .inner
            .on_base_attack(s, rng_seed, player, entity)
    });
}

unsafe extern "C" fn item_on_assist(
    userdata: *mut c_void,
    sim: *mut SimCtxV1,
    player: usize,
    entity: usize,
) {
    with_sim!(sim, |s: &mut StableSim<'_>| item_mut(userdata)
        .inner
        .on_assist(s, player, entity));
}

unsafe extern "C" fn item_on_dead(userdata: *mut c_void, sim: *mut SimCtxV1, player: usize) {
    with_sim!(sim, |s: &mut StableSim<'_>| item_mut(userdata)
        .inner
        .on_dead(s, player));
}

unsafe extern "C" fn item_on_cc(
    userdata: *mut c_void,
    sim: *mut SimCtxV1,
    rng_seed: u64,
    player: usize,
    caster: usize,
) {
    with_sim!(sim, |s: &mut StableSim<'_>| {
        item_mut(userdata).inner.on_cc(s, rng_seed, player, caster)
    });
}

unsafe extern "C" fn item_on_upgrade(userdata: *mut c_void, next_key: crate::StrV1) -> u64 {
    guarded(0, || item_mut(userdata).inner.on_upgrade(next_key.as_str()))
}

unsafe extern "C" fn item_on_upgraded_from(
    userdata: *mut c_void,
    prev_key: crate::StrV1,
    carry: u64,
) {
    guarded((), || {
        item_mut(userdata)
            .inner
            .on_upgraded_from(prev_key.as_str(), carry)
    })
}

unsafe extern "C" fn item_update(
    userdata: *mut c_void,
    sim: *mut SimCtxV1,
    rng_seed: u64,
    player: usize,
) {
    with_sim!(sim, |s: &mut StableSim<'_>| item_mut(userdata)
        .inner
        .update(s, rng_seed, player));
}

unsafe extern "C" fn item_on_spawn(userdata: *mut c_void, sim: *mut SimCtxV1, player: usize) {
    with_sim!(sim, |s: &mut StableSim<'_>| item_mut(userdata)
        .inner
        .on_spawn(s, player));
}

unsafe extern "C" fn item_on_healed(
    userdata: *mut c_void,
    sim: *mut SimCtxV1,
    caster: usize,
    has_caster: bool,
    entity: usize,
    heal: usize,
) {
    with_sim!(sim, |s: &mut StableSim<'_>| {
        let caster = has_caster.then_some(caster);
        item_mut(userdata).inner.on_healed(s, caster, entity, heal)
    });
}

unsafe extern "C" fn item_on_damaged(
    userdata: *mut c_void,
    sim: *mut SimCtxV1,
    player: usize,
    entity: usize,
    attacker: usize,
    damage: usize,
) {
    with_sim!(sim, |s: &mut StableSim<'_>| {
        item_mut(userdata).inner.on_damaged(
            s,
            player,
            entity,
            attacker,
            damage,
            crate::DamageTypeV1::Ad,
            crate::AttackTypeV1::Skill,
            false,
        )
    });
}

#[allow(clippy::too_many_arguments)]
unsafe extern "C" fn item_on_damaged_ex(
    userdata: *mut c_void,
    sim: *mut SimCtxV1,
    player: usize,
    entity: usize,
    attacker: usize,
    damage: usize,
    damage_type: u32,
    attack_type: u32,
    is_crit: bool,
) {
    with_sim!(sim, |s: &mut StableSim<'_>| {
        let damage_type =
            crate::DamageTypeV1::from_code(damage_type).unwrap_or(crate::DamageTypeV1::Ad);
        let attack_type =
            crate::AttackTypeV1::from_code(attack_type).unwrap_or(crate::AttackTypeV1::Skill);
        item_mut(userdata).inner.on_damaged(
            s,
            player,
            entity,
            attacker,
            damage,
            damage_type,
            attack_type,
            is_crit,
        )
    });
}

unsafe extern "C" fn item_on_kill(
    userdata: *mut c_void,
    sim: *mut SimCtxV1,
    rng_seed: u64,
    player: usize,
    entity: usize,
) {
    with_sim!(sim, |s: &mut StableSim<'_>| {
        item_mut(userdata)
            .inner
            .on_kill(s, rng_seed, player, entity, usize::MAX)
    });
}

unsafe extern "C" fn item_on_kill_ex(
    userdata: *mut c_void,
    sim: *mut SimCtxV1,
    rng_seed: u64,
    player: usize,
    entity: usize,
    victim: usize,
) {
    with_sim!(sim, |s: &mut StableSim<'_>| {
        item_mut(userdata)
            .inner
            .on_kill(s, rng_seed, player, entity, victim)
    });
}

unsafe extern "C" fn item_on_skill_hit(
    userdata: *mut c_void,
    sim: *mut SimCtxV1,
    rng_seed: u64,
    caster: usize,
    target: usize,
) {
    with_sim!(sim, |s: &mut StableSim<'_>| {
        item_mut(userdata)
            .inner
            .on_skill_hit(s, rng_seed, caster, target, false)
    });
}

unsafe extern "C" fn item_on_skill_hit_ex(
    userdata: *mut c_void,
    sim: *mut SimCtxV1,
    rng_seed: u64,
    caster: usize,
    target: usize,
    is_ally: bool,
) {
    with_sim!(sim, |s: &mut StableSim<'_>| {
        item_mut(userdata)
            .inner
            .on_skill_hit(s, rng_seed, caster, target, is_ally)
    });
}

static ITEM_VTABLE: ItemVtableV1 = ItemVtableV1 {
    size: size_of::<ItemVtableV1>(),
    destroy: Some(item_destroy),
    clone: Some(item_clone),
    key: Some(item_key),
    icon: Some(item_icon),
    price: Some(item_price),
    tier: Some(item_tier),
    stat: Some(item_stat),
    next_tier: Some(item_next_tier),
    previous_tier: Some(item_previous_tier),
    tags: Some(item_tags),
    category: Some(item_category),
    on_attack: Some(item_on_attack),
    update: Some(item_update),
    on_spawn: Some(item_on_spawn),
    on_healed: Some(item_on_healed),
    on_damaged: Some(item_on_damaged),
    on_kill: Some(item_on_kill),
    on_skill_hit: Some(item_on_skill_hit),
    on_attack_ex: Some(item_on_attack_ex),
    on_damaged_ex: Some(item_on_damaged_ex),
    on_kill_ex: Some(item_on_kill_ex),
    on_skill_hit_ex: Some(item_on_skill_hit_ex),
    on_base_attack: Some(item_on_base_attack),
    on_assist: Some(item_on_assist),
    on_dead: Some(item_on_dead),
    on_cc: Some(item_on_cc),
    on_upgrade: Some(item_on_upgrade),
    on_upgraded_from: Some(item_on_upgraded_from),
};

pub(crate) fn item_to_reg(inner: Box<dyn StableItem>) -> ItemRegV1 {
    let key = inner.key();
    let icon = inner.icon();
    let next_tier = inner.next_tier();
    let previous_tier = inner.previous_tier();
    let shim = Box::new(ItemShim {
        inner,
        key,
        icon,
        next_tier,
        previous_tier,
    });
    ItemRegV1 {
        userdata: Box::into_raw(shim) as *mut c_void,
        vtable: &ITEM_VTABLE,
    }
}

// ---------------------------------------------------------------------------
// Server extension
// ---------------------------------------------------------------------------

struct ServerExtShim {
    inner: Box<dyn StableServerExtension>,
}

unsafe fn server_ext<'a>(userdata: *const c_void) -> &'a ServerExtShim {
    &*(userdata as *const ServerExtShim)
}

unsafe extern "C" fn server_ext_destroy(userdata: *mut c_void) {
    guarded((), || {
        drop(unsafe { Box::from_raw(userdata as *mut ServerExtShim) })
    });
}

macro_rules! server_hook {
    ($name:ident, $method:ident) => {
        unsafe extern "C" fn $name(userdata: *mut c_void, ctx: *mut ServerCtxV1) {
            guarded((), || {
                let Some(mut ctx) = (unsafe { StableServerCtx::from_raw(ctx) }) else {
                    return;
                };
                server_ext(userdata).inner.$method(&mut ctx);
            });
        }
    };
}

server_hook!(server_ext_on_server_start, on_server_start);
server_hook!(server_ext_before_management_tick, before_management_tick);
server_hook!(server_ext_after_management_tick, after_management_tick);

unsafe extern "C" fn server_ext_handle_command(
    userdata: *mut c_void,
    ctx: *mut ServerCtxV1,
    command: *const u8,
    command_len: usize,
    payload: *const u8,
    payload_len: usize,
    sender_player_id: usize,
    has_sender_player: bool,
    sender_team_id: usize,
    has_sender_team: bool,
) -> u32 {
    guarded(CommandResultV1::Pass.code(), || {
        let Some(mut ctx) = (unsafe { StableServerCtx::from_raw(ctx) }) else {
            return CommandResultV1::Pass.code();
        };
        let command_str = if command.is_null() {
            ""
        } else {
            std::str::from_utf8(std::slice::from_raw_parts(command, command_len)).unwrap_or("")
        };
        let payload_slice: &[u8] = if payload.is_null() {
            &[]
        } else {
            std::slice::from_raw_parts(payload, payload_len)
        };
        let command = StableCommand {
            command: command_str,
            payload: payload_slice,
            sender_player_id: has_sender_player.then_some(sender_player_id),
            sender_team_id: has_sender_team.then_some(sender_team_id),
        };
        server_ext(userdata)
            .inner
            .handle_command(&mut ctx, &command)
            .code()
    })
}

static SERVER_EXT_VTABLE: ServerExtVtableV1 = ServerExtVtableV1 {
    size: size_of::<ServerExtVtableV1>(),
    destroy: Some(server_ext_destroy),
    on_server_start: Some(server_ext_on_server_start),
    before_management_tick: Some(server_ext_before_management_tick),
    after_management_tick: Some(server_ext_after_management_tick),
    handle_command: Some(server_ext_handle_command),
};

pub(crate) fn server_ext_to_reg(inner: Box<dyn StableServerExtension>) -> ServerExtRegV1 {
    ServerExtRegV1 {
        userdata: Box::into_raw(Box::new(ServerExtShim { inner })) as *mut c_void,
        vtable: &SERVER_EXT_VTABLE,
    }
}

// ---------------------------------------------------------------------------
// Client extension
// ---------------------------------------------------------------------------

struct ExtensionShim {
    inner: Box<dyn StableExtension>,
}

unsafe fn extension<'a>(userdata: *const c_void) -> &'a ExtensionShim {
    &*(userdata as *const ExtensionShim)
}

unsafe extern "C" fn extension_destroy(userdata: *mut c_void) {
    guarded((), || {
        drop(unsafe { Box::from_raw(userdata as *mut ExtensionShim) })
    });
}

unsafe extern "C" fn extension_on_init(userdata: *mut c_void, ctx: *mut ClientCtxV1) {
    guarded((), || {
        let Some(mut client) = (unsafe { StableClient::from_raw(ctx) }) else {
            return;
        };
        extension(userdata).inner.on_init(&mut client);
    });
}

unsafe extern "C" fn extension_pre_update(
    userdata: *mut c_void,
    ctx: *mut ClientCtxV1,
    dt_micros: u64,
) {
    guarded((), || {
        let Some(mut client) = (unsafe { StableClient::from_raw(ctx) }) else {
            return;
        };
        extension(userdata).inner.pre_update(&mut client, dt_micros);
    });
}

unsafe extern "C" fn extension_post_update(
    userdata: *mut c_void,
    ctx: *mut ClientCtxV1,
    dt_micros: u64,
) {
    guarded((), || {
        let Some(mut client) = (unsafe { StableClient::from_raw(ctx) }) else {
            return;
        };
        extension(userdata)
            .inner
            .post_update(&mut client, dt_micros);
    });
}

unsafe extern "C" fn extension_on_end(userdata: *mut c_void) {
    guarded((), || extension(userdata).inner.on_end());
}

unsafe extern "C" fn extension_pre_render(userdata: *mut c_void, ctx: *mut ClientCtxV1) {
    guarded((), || {
        let Some(mut client) = (unsafe { StableClient::from_raw(ctx) }) else {
            return;
        };
        extension(userdata).inner.pre_render(&mut client);
    });
}

unsafe extern "C" fn extension_post_render(userdata: *mut c_void, ctx: *mut ClientCtxV1) {
    guarded((), || {
        let Some(mut client) = (unsafe { StableClient::from_raw(ctx) }) else {
            return;
        };
        extension(userdata).inner.post_render(&mut client);
    });
}

static EXTENSION_VTABLE: ExtensionVtableV1 = ExtensionVtableV1 {
    size: size_of::<ExtensionVtableV1>(),
    destroy: Some(extension_destroy),
    on_init: Some(extension_on_init),
    pre_update: Some(extension_pre_update),
    post_update: Some(extension_post_update),
    on_end: Some(extension_on_end),
    pre_render: Some(extension_pre_render),
    post_render: Some(extension_post_render),
};

pub(crate) fn extension_to_reg(inner: Box<dyn StableExtension>) -> ExtensionRegV1 {
    ExtensionRegV1 {
        userdata: Box::into_raw(Box::new(ExtensionShim { inner })) as *mut c_void,
        vtable: &EXTENSION_VTABLE,
    }
}

// ---------------------------------------------------------------------------
// Draft score hook
// ---------------------------------------------------------------------------

struct DraftHookShim {
    inner: Box<dyn StableDraftHook>,
    id: String,
}

unsafe fn draft_hook<'a>(userdata: *const c_void) -> &'a DraftHookShim {
    &*(userdata as *const DraftHookShim)
}

unsafe extern "C" fn draft_hook_destroy(userdata: *mut c_void) {
    guarded((), || {
        drop(unsafe { Box::from_raw(userdata as *mut DraftHookShim) })
    });
}

unsafe extern "C" fn draft_hook_id(userdata: *const c_void) -> StrV1 {
    guarded(StrV1::EMPTY, || StrV1::from_str(&draft_hook(userdata).id))
}

unsafe extern "C" fn draft_hook_priority(userdata: *const c_void) -> i32 {
    guarded(0, || draft_hook(userdata).inner.priority())
}

fn write_draft_decision(decision: StableDraftDecision, out_score: *mut f32) -> u32 {
    match decision {
        StableDraftDecision::Pass => DraftDecisionKindV1::Pass.code(),
        StableDraftDecision::Add(score) => {
            if !out_score.is_null() {
                unsafe { *out_score = score };
            }
            DraftDecisionKindV1::Add.code()
        }
        StableDraftDecision::Replace(score) => {
            if !out_score.is_null() {
                unsafe { *out_score = score };
            }
            DraftDecisionKindV1::Replace.code()
        }
    }
}

unsafe extern "C" fn draft_hook_score_ban(
    userdata: *const c_void,
    ctx: *const DraftCtxV1,
    candidate: usize,
    base_score: f32,
    out_score: *mut f32,
) -> u32 {
    guarded(DraftDecisionKindV1::Pass.code(), || {
        let Some(ctx) = (unsafe { StableDraftContext::from_raw(ctx) }) else {
            return DraftDecisionKindV1::Pass.code();
        };
        let decision = draft_hook(userdata)
            .inner
            .score_ban(&ctx, candidate, base_score);
        write_draft_decision(decision, out_score)
    })
}

unsafe extern "C" fn draft_hook_score_pick(
    userdata: *const c_void,
    ctx: *const DraftCtxV1,
    candidate: usize,
    base_score: f32,
    out_score: *mut f32,
) -> u32 {
    guarded(DraftDecisionKindV1::Pass.code(), || {
        let Some(ctx) = (unsafe { StableDraftContext::from_raw(ctx) }) else {
            return DraftDecisionKindV1::Pass.code();
        };
        let decision = draft_hook(userdata)
            .inner
            .score_pick(&ctx, candidate, base_score);
        write_draft_decision(decision, out_score)
    })
}

unsafe extern "C" fn draft_hook_decide_ban(
    userdata: *const c_void,
    ctx: *const DraftCtxV1,
    out_champion: *mut usize,
) -> bool {
    guarded(false, || {
        let Some(ctx) = (unsafe { StableDraftContext::from_raw(ctx) }) else {
            return false;
        };
        match draft_hook(userdata).inner.decide_ban(&ctx) {
            Some(champion) => {
                if !out_champion.is_null() {
                    unsafe { *out_champion = champion };
                }
                true
            }
            None => false,
        }
    })
}

unsafe extern "C" fn draft_hook_decide_pick(
    userdata: *const c_void,
    ctx: *const DraftCtxV1,
    out_champion: *mut usize,
) -> bool {
    guarded(false, || {
        let Some(ctx) = (unsafe { StableDraftContext::from_raw(ctx) }) else {
            return false;
        };
        match draft_hook(userdata).inner.decide_pick(&ctx) {
            Some(champion) => {
                if !out_champion.is_null() {
                    unsafe { *out_champion = champion };
                }
                true
            }
            None => false,
        }
    })
}

static DRAFT_HOOK_VTABLE: DraftHookVtableV1 = DraftHookVtableV1 {
    size: size_of::<DraftHookVtableV1>(),
    destroy: Some(draft_hook_destroy),
    id: Some(draft_hook_id),
    priority: Some(draft_hook_priority),
    score_ban: Some(draft_hook_score_ban),
    score_pick: Some(draft_hook_score_pick),
    decide_ban: Some(draft_hook_decide_ban),
    decide_pick: Some(draft_hook_decide_pick),
};

pub(crate) fn draft_hook_to_reg(inner: Box<dyn StableDraftHook>) -> DraftHookRegV1 {
    let id = inner.id();
    DraftHookRegV1 {
        userdata: Box::into_raw(Box::new(DraftHookShim { inner, id })) as *mut c_void,
        vtable: &DRAFT_HOOK_VTABLE,
    }
}

// ---------------------------------------------------------------------------
// Item build hook
// ---------------------------------------------------------------------------

struct ItemBuildHookShim {
    inner: Box<dyn crate::StableItemBuildHook>,
    id: String,
}

unsafe fn item_build_hook<'a>(userdata: *const c_void) -> &'a ItemBuildHookShim {
    &*(userdata as *const ItemBuildHookShim)
}

unsafe extern "C" fn item_build_hook_destroy(userdata: *mut c_void) {
    guarded((), || {
        drop(unsafe { Box::from_raw(userdata as *mut ItemBuildHookShim) })
    });
}

unsafe extern "C" fn item_build_hook_id(userdata: *const c_void) -> StrV1 {
    guarded(StrV1::EMPTY, || {
        StrV1::from_str(&item_build_hook(userdata).id)
    })
}

unsafe extern "C" fn item_build_hook_priority(userdata: *const c_void) -> i32 {
    guarded(0, || item_build_hook(userdata).inner.priority())
}

unsafe extern "C" fn item_build_hook_score_item(
    userdata: *const c_void,
    ctx: *const crate::ItemBuildCtxV1,
    candidate: usize,
    base_score: f32,
    out_score: *mut f32,
) -> u32 {
    guarded(DraftDecisionKindV1::Pass.code(), || {
        let Some(ctx) = (unsafe { crate::StableItemBuildContext::from_raw(ctx) }) else {
            return DraftDecisionKindV1::Pass.code();
        };
        let decision = item_build_hook(userdata)
            .inner
            .score_item(&ctx, candidate, base_score);
        write_draft_decision(decision, out_score)
    })
}

unsafe extern "C" fn item_build_hook_decide_build(
    userdata: *const c_void,
    ctx: *const crate::ItemBuildCtxV1,
    out_build: *mut usize,
    cap: usize,
) -> usize {
    guarded(0, || {
        let Some(ctx) = (unsafe { crate::StableItemBuildContext::from_raw(ctx) }) else {
            return 0;
        };
        let build = item_build_hook(userdata).inner.decide_build(&ctx);
        if out_build.is_null() {
            return 0;
        }
        for (i, item) in build.iter().take(cap).enumerate() {
            unsafe { *out_build.add(i) = *item };
        }
        build.len().min(cap)
    })
}

static ITEM_BUILD_HOOK_VTABLE: crate::ItemBuildHookVtableV1 = crate::ItemBuildHookVtableV1 {
    size: size_of::<crate::ItemBuildHookVtableV1>(),
    destroy: Some(item_build_hook_destroy),
    id: Some(item_build_hook_id),
    priority: Some(item_build_hook_priority),
    score_item: Some(item_build_hook_score_item),
    decide_build: Some(item_build_hook_decide_build),
};

pub(crate) fn item_build_hook_to_reg(
    inner: Box<dyn crate::StableItemBuildHook>,
) -> crate::ItemBuildHookRegV1 {
    let id = inner.id();
    crate::ItemBuildHookRegV1 {
        userdata: Box::into_raw(Box::new(ItemBuildHookShim { inner, id })) as *mut c_void,
        vtable: &ITEM_BUILD_HOOK_VTABLE,
    }
}

// ---------------------------------------------------------------------------
// Player input AI
// ---------------------------------------------------------------------------

struct PlayerAiShim {
    inner: Box<dyn StablePlayerAi>,
    id: String,
}

unsafe fn player_ai<'a>(userdata: *const c_void) -> &'a PlayerAiShim {
    &*(userdata as *const PlayerAiShim)
}

unsafe fn player_ai_mut<'a>(userdata: *mut c_void) -> &'a mut PlayerAiShim {
    &mut *(userdata as *mut PlayerAiShim)
}

unsafe extern "C" fn player_ai_destroy(userdata: *mut c_void) {
    guarded((), || {
        drop(unsafe { Box::from_raw(userdata as *mut PlayerAiShim) })
    });
}

unsafe extern "C" fn player_ai_clone(userdata: *const c_void) -> *mut c_void {
    guarded(std::ptr::null_mut(), || {
        let shim = player_ai(userdata);
        let cloned = Box::new(PlayerAiShim {
            inner: shim.inner.clone_box(),
            id: shim.id.clone(),
        });
        Box::into_raw(cloned) as *mut c_void
    })
}

unsafe extern "C" fn player_ai_id(userdata: *const c_void) -> StrV1 {
    guarded(StrV1::EMPTY, || StrV1::from_str(&player_ai(userdata).id))
}

unsafe extern "C" fn player_ai_priority(userdata: *const c_void) -> i32 {
    guarded(0, || player_ai(userdata).inner.priority())
}

unsafe extern "C" fn player_ai_matches(userdata: *const c_void, init: *const AiInitV1) -> bool {
    guarded(false, || {
        let Some(init) = (unsafe { StableAiInit::from_raw(init) }) else {
            return false;
        };
        player_ai(userdata).inner.matches(&init)
    })
}

unsafe extern "C" fn player_ai_think(
    userdata: *mut c_void,
    ctx: *mut AiCtxV1,
    base_input: *const InputV1,
    out_input: *mut InputV1,
) -> u32 {
    guarded(AiDecisionKindV1::Pass.code(), || {
        let Some(mut ctx) = (unsafe { StableAiContext::from_raw(ctx) }) else {
            return AiDecisionKindV1::Pass.code();
        };
        let base = if base_input.is_null() {
            None
        } else {
            Some(unsafe { *base_input })
        };
        match player_ai_mut(userdata).inner.think(&mut ctx, base) {
            Some(input) => {
                if !out_input.is_null() {
                    unsafe { *out_input = input };
                }
                AiDecisionKindV1::Replace.code()
            }
            None => AiDecisionKindV1::Pass.code(),
        }
    })
}

static PLAYER_AI_VTABLE: PlayerAiVtableV1 = PlayerAiVtableV1 {
    size: size_of::<PlayerAiVtableV1>(),
    destroy: Some(player_ai_destroy),
    clone: Some(player_ai_clone),
    id: Some(player_ai_id),
    priority: Some(player_ai_priority),
    matches: Some(player_ai_matches),
    think: Some(player_ai_think),
};

pub(crate) fn player_ai_to_reg(inner: Box<dyn StablePlayerAi>) -> PlayerAiRegV1 {
    let id = inner.id();
    PlayerAiRegV1 {
        userdata: Box::into_raw(Box::new(PlayerAiShim { inner, id })) as *mut c_void,
        vtable: &PLAYER_AI_VTABLE,
    }
}

// ---------------------------------------------------------------------------
// Match hook
// ---------------------------------------------------------------------------

struct MatchHookShim {
    inner: Box<dyn crate::StableMatchHook>,
}

unsafe fn match_hook<'a>(userdata: *const c_void) -> &'a MatchHookShim {
    &*(userdata as *const MatchHookShim)
}

unsafe extern "C" fn match_hook_destroy(userdata: *mut c_void) {
    guarded((), || {
        drop(unsafe { Box::from_raw(userdata as *mut MatchHookShim) })
    });
}

unsafe extern "C" fn match_hook_on_start(userdata: *mut c_void, sim: *mut SimCtxV1) {
    guarded((), || {
        let Some(mut sim) = (unsafe { StableSim::from_raw(sim) }) else {
            return;
        };
        match_hook(userdata).inner.on_match_start(&mut sim);
    });
}

unsafe extern "C" fn match_hook_on_tick(userdata: *mut c_void, sim: *mut SimCtxV1, rng_seed: u64) {
    guarded((), || {
        let Some(mut sim) = (unsafe { StableSim::from_raw(sim) }) else {
            return;
        };
        match_hook(userdata).inner.on_match_tick(&mut sim, rng_seed);
    });
}

unsafe extern "C" fn match_hook_check_end(
    userdata: *mut c_void,
    sim: *mut SimCtxV1,
    out_blue_win: *mut bool,
) -> bool {
    guarded(false, || {
        let Some(mut sim) = (unsafe { StableSim::from_raw(sim) }) else {
            return false;
        };
        match match_hook(userdata).inner.check_match_end(&mut sim) {
            Some(blue_win) => {
                if !out_blue_win.is_null() {
                    *out_blue_win = blue_win;
                }
                true
            }
            None => false,
        }
    })
}

static MATCH_HOOK_VTABLE: crate::MatchHookVtableV1 = crate::MatchHookVtableV1 {
    size: size_of::<crate::MatchHookVtableV1>(),
    destroy: Some(match_hook_destroy),
    on_match_start: Some(match_hook_on_start),
    on_match_tick: Some(match_hook_on_tick),
    check_match_end: Some(match_hook_check_end),
};

pub(crate) fn match_hook_to_reg(inner: Box<dyn crate::StableMatchHook>) -> crate::MatchHookRegV1 {
    crate::MatchHookRegV1 {
        userdata: Box::into_raw(Box::new(MatchHookShim { inner })) as *mut c_void,
        vtable: &MATCH_HOOK_VTABLE,
    }
}

// ---------------------------------------------------------------------------
// Map customizer
// ---------------------------------------------------------------------------

struct MapCustomizerShim {
    inner: Box<dyn crate::StableMapCustomizer>,
}

unsafe extern "C" fn map_customizer_destroy(userdata: *mut c_void) {
    guarded((), || {
        drop(unsafe { Box::from_raw(userdata as *mut MapCustomizerShim) })
    });
}

unsafe extern "C" fn map_customizer_customize(
    userdata: *mut c_void,
    mode_kind: u32,
    doc: *mut crate::JsonDocCtxV1,
) {
    guarded((), || {
        let Some(mut doc) = (unsafe { crate::StableJsonDoc::from_raw(doc) }) else {
            return;
        };
        let shim = unsafe { &*(userdata as *const MapCustomizerShim) };
        shim.inner
            .customize(crate::GameModeKindV1::from_code(mode_kind), &mut doc);
    });
}

static MAP_CUSTOMIZER_VTABLE: crate::MapCustomizerVtableV1 = crate::MapCustomizerVtableV1 {
    size: size_of::<crate::MapCustomizerVtableV1>(),
    destroy: Some(map_customizer_destroy),
    customize: Some(map_customizer_customize),
};

pub(crate) fn map_customizer_to_reg(
    inner: Box<dyn crate::StableMapCustomizer>,
) -> crate::MapCustomizerRegV1 {
    crate::MapCustomizerRegV1 {
        userdata: Box::into_raw(Box::new(MapCustomizerShim { inner })) as *mut c_void,
        vtable: &MAP_CUSTOMIZER_VTABLE,
    }
}
