//! Mod-side safe wrapper over [`SimCtxV1`] — how gameplay callbacks read and
//! act on the running simulation. Compiled into the mod; never crosses the
//! boundary itself.

use std::mem::size_of;

use crate::{
    AttackTypeV1, BuffV1, CcV1, EntityHandleV1, FrameVtableV1, KillLogV1, LaneV1, PlayerHandleV1,
    ProjectileHandleV1, ProjectileInfoV1, SimCtxV1, SimVtableV1, StatV1,
};

/// Size-guarded view over the per-callback simulation context. Valid only
/// inside the callback that received it.
pub struct StableSim<'a> {
    raw: *mut SimCtxV1,
    _marker: std::marker::PhantomData<&'a mut SimCtxV1>,
}

impl<'a> StableSim<'a> {
    /// # Safety
    /// `raw` must point at a live `SimCtxV1` for the duration of the callback.
    pub unsafe fn from_raw(raw: *mut SimCtxV1) -> Option<Self> {
        if raw.is_null() {
            return None;
        }
        if (*raw).size < size_of::<SimCtxV1>() {
            return None;
        }
        Some(Self {
            raw,
            _marker: std::marker::PhantomData,
        })
    }

    fn sim(&self) -> *const SimVtableV1 {
        unsafe { (*self.raw).sim }
    }

    fn state(&self) -> *mut std::ffi::c_void {
        unsafe { (*self.raw).state }
    }

    // -- game state --

    pub fn tick(&self) -> usize {
        slot!(self.sim(), SimVtableV1, tick).map_or(0, |f| unsafe { f(self.state()) })
    }

    pub fn seed(&self) -> u64 {
        slot!(self.sim(), SimVtableV1, seed).map_or(0, |f| unsafe { f(self.state()) })
    }

    pub fn score_diff(&self, team: usize) -> i32 {
        slot!(self.sim(), SimVtableV1, score_diff).map_or(0, |f| unsafe { f(self.state(), team) })
    }

    pub fn is_end(&self) -> bool {
        slot!(self.sim(), SimVtableV1, is_end).map_or(false, |f| unsafe { f(self.state()) })
    }

    // -- entities --

    pub fn entity_count(&self) -> usize {
        slot!(self.sim(), SimVtableV1, entity_count).map_or(0, |f| unsafe { f(self.state()) })
    }

    pub fn entity_at(&self, index: usize) -> Option<StableEntity<'_, 'a>> {
        let handle = slot!(self.sim(), SimVtableV1, entity_at)
            .map_or(EntityHandleV1::NULL, |f| unsafe { f(self.state(), index) });
        self.entity(handle)
    }

    pub fn get_entity(&self, id: usize) -> Option<StableEntity<'_, 'a>> {
        self.entity(EntityHandleV1::from_id(id))
    }

    pub fn entity(&self, handle: EntityHandleV1) -> Option<StableEntity<'_, 'a>> {
        let valid = slot!(self.sim(), SimVtableV1, entity_is_valid)
            .map_or(false, |f| unsafe { f(self.state(), handle) });
        valid.then_some(StableEntity { sim: self, handle })
    }

    // -- players --

    pub fn player_count(&self) -> usize {
        slot!(self.sim(), SimVtableV1, player_count).map_or(0, |f| unsafe { f(self.state()) })
    }

    pub fn player_at(&self, index: usize) -> Option<StablePlayer<'_, 'a>> {
        let handle = slot!(self.sim(), SimVtableV1, player_at)
            .map_or(PlayerHandleV1::NULL, |f| unsafe { f(self.state(), index) });
        self.player(handle)
    }

    pub fn get_player(&self, id: usize) -> Option<StablePlayer<'_, 'a>> {
        self.player(PlayerHandleV1::from_id(id))
    }

    pub fn player(&self, handle: PlayerHandleV1) -> Option<StablePlayer<'_, 'a>> {
        let valid = slot!(self.sim(), SimVtableV1, player_is_valid)
            .map_or(false, |f| unsafe { f(self.state(), handle) });
        valid.then_some(StablePlayer { sim: self, handle })
    }

    // -- combat actions --

    /// Deals damage through the engine's own attack pipeline: armor/magic
    /// resist, crit, damage/tank statistics, kill and assist credit, item and
    /// buff procs, lifesteal and reflect, and the damage number in the client.
    /// `ad`/`ap` are the raw pre-mitigation amounts.
    pub fn deal_damage(
        &mut self,
        attacker: usize,
        target: usize,
        ad: usize,
        ap: usize,
        attack_type: AttackTypeV1,
    ) {
        if let Some(f) = slot!(self.sim(), SimVtableV1, deal_damage) {
            unsafe { f(self.state(), attacker, target, ad, ap, attack_type.code()) }
        }
    }

    /// Subtracts HP with no mitigation, statistics or kill credit — for mods
    /// that model their own damage math. Prefer [`StableSim::deal_damage`].
    pub fn deal_damage_raw(
        &mut self,
        attacker: usize,
        target: usize,
        ad: usize,
        ap: usize,
        attack_type: AttackTypeV1,
    ) {
        if let Some(f) = slot!(self.sim(), SimVtableV1, deal_damage_raw) {
            unsafe { f(self.state(), attacker, target, ad, ap, attack_type.code()) }
        }
    }

    pub fn heal(&mut self, caster: usize, target: usize, amount: usize) {
        if let Some(f) = slot!(self.sim(), SimVtableV1, heal) {
            unsafe { f(self.state(), caster, target, amount) }
        }
    }

    pub fn add_buff(&mut self, target: usize, buff: &BuffV1) {
        if let Some(f) = slot!(self.sim(), SimVtableV1, add_buff) {
            unsafe { f(self.state(), target, buff) }
        }
    }

    pub fn apply_cc(&mut self, target: usize, cc: &CcV1) {
        if let Some(f) = slot!(self.sim(), SimVtableV1, apply_cc) {
            unsafe { f(self.state(), target, cc) }
        }
    }

    // -- direct mutation (custom-mode building blocks) --

    /// Sets current HP directly (no damage attribution — use `deal_damage`
    /// when the change should show up in kill logs/stats).
    pub fn entity_set_hp(&mut self, entity_id: usize, hp: usize) -> bool {
        slot!(self.sim(), SimVtableV1, entity_set_hp).map_or(false, |f| unsafe {
            f(self.state(), EntityHandleV1::from_id(entity_id), hp)
        })
    }

    /// Teleports the entity.
    pub fn entity_set_pos(&mut self, entity_id: usize, x: u64, y: u64) -> bool {
        slot!(self.sim(), SimVtableV1, entity_set_pos).map_or(false, |f| unsafe {
            f(self.state(), EntityHandleV1::from_id(entity_id), x, y)
        })
    }

    /// Replaces the entity's base stat block (buffs still apply on top).
    pub fn entity_set_base_stat(&mut self, entity_id: usize, stat: &StatV1) -> bool {
        slot!(self.sim(), SimVtableV1, entity_set_base_stat).map_or(false, |f| unsafe {
            f(self.state(), EntityHandleV1::from_id(entity_id), stat)
        })
    }

    pub fn player_set_gold(&mut self, player_id: usize, gold: usize) -> bool {
        slot!(self.sim(), SimVtableV1, player_set_gold).map_or(false, |f| unsafe {
            f(self.state(), PlayerHandleV1::from_id(player_id), gold)
        })
    }

    /// Saturating gold delta (negative floors at zero).
    pub fn player_add_gold(&mut self, player_id: usize, delta: i64) -> bool {
        slot!(self.sim(), SimVtableV1, player_add_gold).map_or(false, |f| unsafe {
            f(self.state(), PlayerHandleV1::from_id(player_id), delta)
        })
    }

    /// Ends the match immediately with the given winner.
    pub fn force_end(&mut self, blue_win: bool) {
        if let Some(f) = slot!(self.sim(), SimVtableV1, force_end) {
            unsafe { f(self.state(), blue_win) }
        }
    }

    /// Queues a registered native effect (by name) to fire `delay_ticks`
    /// from now with the given caster and input.
    pub fn queue_effect(
        &mut self,
        effect_name: &str,
        attack_type: AttackTypeV1,
        caster_id: usize,
        input: &crate::InputTargetV1,
        delay_ticks: usize,
    ) -> bool {
        slot!(self.sim(), SimVtableV1, queue_effect).map_or(false, |f| unsafe {
            f(
                self.state(),
                effect_name.as_ptr(),
                effect_name.len(),
                attack_type.code(),
                caster_id,
                input,
                delay_ticks,
            )
        })
    }

    /// Spawns a duration-limited combat unit that chases and attacks nearby
    /// enemies. Returns the new entity id.
    #[allow(clippy::too_many_arguments)]
    pub fn spawn_unit(
        &mut self,
        name: &str,
        summoner_id: usize,
        team: usize,
        x: u64,
        y: u64,
        duration_ticks: u64,
        stat: &StatV1,
        attack: &crate::UnitAttackV1,
    ) -> Option<usize> {
        let f = slot!(self.sim(), SimVtableV1, spawn_unit)?;
        let id = unsafe {
            f(
                self.state(),
                name.as_ptr(),
                name.len(),
                summoner_id,
                team,
                x,
                y,
                duration_ticks,
                stat,
                attack,
            )
        };
        (id != usize::MAX).then_some(id)
    }

    /// Spawns a projectile that applies a registered native effect on hit.
    pub fn spawn_projectile(
        &mut self,
        name: &str,
        effect_name: &str,
        spec: &crate::ProjectileSpawnV1,
    ) -> bool {
        slot!(self.sim(), SimVtableV1, spawn_projectile).map_or(false, |f| unsafe {
            f(
                self.state(),
                name.as_ptr(),
                name.len(),
                effect_name.as_ptr(),
                effect_name.len(),
                spec,
            )
        })
    }

    /// Removes every stat buff with the given name from the entity; returns
    /// the number removed.
    pub fn entity_remove_buff(&mut self, entity_id: usize, name: &str) -> usize {
        slot!(self.sim(), SimVtableV1, entity_remove_buff).map_or(0, |f| unsafe {
            f(
                self.state(),
                EntityHandleV1::from_id(entity_id),
                name.as_ptr(),
                name.len(),
            )
        })
    }

    /// Clears every crowd-control effect on the entity; returns the number
    /// removed.
    pub fn entity_clear_cc(&mut self, entity_id: usize) -> usize {
        slot!(self.sim(), SimVtableV1, entity_clear_cc).map_or(0, |f| unsafe {
            f(self.state(), EntityHandleV1::from_id(entity_id))
        })
    }

    /// Grants a damage-absorbing shield expiring after `duration_ticks`.
    /// Shields stack as separate layers, like the built-in shield effect.
    /// Read the current total back with `entity.shield()`.
    pub fn entity_add_shield(
        &mut self,
        entity_id: usize,
        amount: usize,
        duration_ticks: usize,
    ) -> bool {
        slot!(self.sim(), SimVtableV1, entity_add_shield).map_or(false, |f| unsafe {
            f(
                self.state(),
                EntityHandleV1::from_id(entity_id),
                amount,
                duration_ticks,
            )
        })
    }

    /// Drops every shield layer on the entity; returns the number removed.
    pub fn entity_clear_shield(&mut self, entity_id: usize) -> usize {
        slot!(self.sim(), SimVtableV1, entity_clear_shield).map_or(0, |f| unsafe {
            f(self.state(), EntityHandleV1::from_id(entity_id))
        })
    }

    /// Reads a fragment of a team's live macro strategy document as JSON.
    pub fn strategy_get_json(&self, team: usize, path: &str) -> Option<String> {
        let f = slot!(self.sim(), SimVtableV1, strategy_get_json)?;
        let mut len = 0usize;
        if !unsafe {
            f(
                self.state(),
                team,
                path.as_ptr(),
                path.len(),
                std::ptr::null_mut(),
                0,
                &mut len,
            )
        } {
            return None;
        }
        let mut buf = vec![0u8; len];
        let mut full = 0usize;
        if !unsafe {
            f(
                self.state(),
                team,
                path.as_ptr(),
                path.len(),
                buf.as_mut_ptr(),
                buf.len(),
                &mut full,
            )
        } {
            return None;
        }
        buf.truncate(full.min(len));
        String::from_utf8(buf).ok()
    }

    /// Replaces a fragment of a team's macro strategy document (atomic
    /// reject on schema violation).
    pub fn strategy_set_json(&mut self, team: usize, path: &str, json: &str) -> bool {
        slot!(self.sim(), SimVtableV1, strategy_set_json).map_or(false, |f| unsafe {
            f(
                self.state(),
                team,
                path.as_ptr(),
                path.len(),
                json.as_ptr(),
                json.len(),
            )
        })
    }

    // -- utility --

    pub fn distance_sq(&self, id1: usize, id2: usize) -> u64 {
        slot!(self.sim(), SimVtableV1, distance_sq)
            .map_or(u64::MAX, |f| unsafe { f(self.state(), id1, id2) })
    }

    pub fn is_visible(&self, team: usize, id: usize) -> bool {
        slot!(self.sim(), SimVtableV1, is_visible)
            .map_or(false, |f| unsafe { f(self.state(), team, id) })
    }

    // -- game structure --

    pub fn champion_count(&self) -> usize {
        slot!(self.sim(), SimVtableV1, champion_count).map_or(0, |f| unsafe { f(self.state()) })
    }

    pub fn champion_id_at(&self, index: usize) -> usize {
        slot!(self.sim(), SimVtableV1, champion_id_at)
            .map_or(usize::MAX, |f| unsafe { f(self.state(), index) })
    }

    pub fn tower_count(&self) -> usize {
        slot!(self.sim(), SimVtableV1, tower_count).map_or(0, |f| unsafe { f(self.state()) })
    }

    pub fn tower_id_at(&self, index: usize) -> usize {
        slot!(self.sim(), SimVtableV1, tower_id_at)
            .map_or(usize::MAX, |f| unsafe { f(self.state(), index) })
    }

    pub fn kill_log_count(&self) -> usize {
        slot!(self.sim(), SimVtableV1, kill_log_count).map_or(0, |f| unsafe { f(self.state()) })
    }

    pub fn kill_log_at(&self, index: usize) -> Option<KillLogV1> {
        let f = slot!(self.sim(), SimVtableV1, kill_log_at)?;
        let mut out = KillLogV1::default();
        unsafe { f(self.state(), index, &mut out) }.then_some(out)
    }

    // -- projectiles --

    pub fn projectile_count(&self) -> usize {
        slot!(self.sim(), SimVtableV1, projectile_count).map_or(0, |f| unsafe { f(self.state()) })
    }

    pub fn projectile_at(&self, index: usize) -> Option<ProjectileInfoV1> {
        let handle = slot!(self.sim(), SimVtableV1, projectile_at)
            .map_or(ProjectileHandleV1::NULL, |f| unsafe {
                f(self.state(), index)
            });
        let valid = slot!(self.sim(), SimVtableV1, projectile_is_valid)
            .map_or(false, |f| unsafe { f(self.state(), handle) });
        if !valid {
            return None;
        }
        let f = slot!(self.sim(), SimVtableV1, projectile_info)?;
        let mut out = ProjectileInfoV1::default();
        unsafe { f(self.state(), handle, &mut out) }.then_some(out)
    }

    // -- debug drawing --

    pub fn debug_draw_line(&mut self, x1: u64, y1: u64, x2: u64, y2: u64, color: u32) {
        let frame = unsafe { (*self.raw).frame };
        if let Some(f) = slot!(frame, FrameVtableV1, debug_draw_line) {
            unsafe { f(self.state(), x1, y1, x2, y2, color) }
        }
    }

    pub fn debug_draw_circle(&mut self, x: u64, y: u64, r: u64, color: u32) {
        let frame = unsafe { (*self.raw).frame };
        if let Some(f) = slot!(frame, FrameVtableV1, debug_draw_circle) {
            unsafe { f(self.state(), x, y, r, color) }
        }
    }
}

macro_rules! entity_getter {
    ($name:ident, $slot:ident, $ret:ty, $default:expr) => {
        pub fn $name(&self) -> $ret {
            slot!(self.sim.sim(), SimVtableV1, $slot)
                .map_or($default, |f| unsafe { f(self.sim.state(), self.handle) })
        }
    };
}

/// Entity view; valid only within the callback that produced it.
pub struct StableEntity<'s, 'a> {
    sim: &'s StableSim<'a>,
    handle: EntityHandleV1,
}

impl StableEntity<'_, '_> {
    pub fn handle(&self) -> EntityHandleV1 {
        self.handle
    }

    /// Entity id usable with combat/utility calls.
    pub fn id(&self) -> usize {
        self.handle.id().unwrap_or(usize::MAX)
    }

    pub fn stat(&self) -> StatV1 {
        let mut out = StatV1::default();
        if let Some(f) = slot!(self.sim.sim(), SimVtableV1, entity_stat) {
            unsafe { f(self.sim.state(), self.handle, &mut out) };
        }
        out
    }

    pub fn pos(&self) -> (u64, u64) {
        let (mut x, mut y) = (0, 0);
        if let Some(f) = slot!(self.sim.sim(), SimVtableV1, entity_pos) {
            unsafe { f(self.sim.state(), self.handle, &mut x, &mut y) };
        }
        (x, y)
    }

    /// (current, max) hp.
    pub fn hp(&self) -> (usize, usize) {
        let (mut current, mut max) = (0, 0);
        if let Some(f) = slot!(self.sim.sim(), SimVtableV1, entity_hp) {
            unsafe { f(self.sim.state(), self.handle, &mut current, &mut max) };
        }
        (current, max)
    }

    entity_getter!(team, entity_team, usize, 0);
    entity_getter!(level, entity_level, usize, 1);
    entity_getter!(is_alive, entity_is_alive, bool, false);
    entity_getter!(is_champion, entity_is_champion, bool, false);
    entity_getter!(is_tower, entity_is_tower, bool, false);
    entity_getter!(is_minion, entity_is_minion, bool, false);
    entity_getter!(shield, entity_shield, usize, 0);
    // Ticks between auto-attacks, after attack-speed buffs. 0 when the host
    // predates this slot.
    entity_getter!(attack_interval, entity_attack_interval, usize, 0);
    // Attack-speed multiplier in percent (100 = unbuffed).
    entity_getter!(attack_speed_mult, entity_attack_speed_mult, usize, 100);
    entity_getter!(radius, entity_radius, usize, 0);
    entity_getter!(is_targetable, entity_is_targetable, bool, false);
    entity_getter!(buff_count, entity_buff_count, usize, 0);
    entity_getter!(cc_count, entity_cc_count, usize, 0);

    pub fn buff_at(&self, index: usize) -> Option<BuffV1> {
        let f = slot!(self.sim.sim(), SimVtableV1, entity_buff_at)?;
        let mut out = BuffV1::default();
        unsafe { f(self.sim.state(), self.handle, index, &mut out) }.then_some(out)
    }

    pub fn cc_at(&self, index: usize) -> Option<CcV1> {
        let f = slot!(self.sim.sim(), SimVtableV1, entity_cc_at)?;
        let mut out = CcV1::default();
        unsafe { f(self.sim.state(), self.handle, index, &mut out) }.then_some(out)
    }

    /// Entity display/champion name.
    pub fn name(&self) -> Option<String> {
        let f = slot!(self.sim.sim(), SimVtableV1, entity_name)?;
        let mut len = 0usize;
        if !unsafe {
            f(
                self.sim.state(),
                self.handle,
                std::ptr::null_mut(),
                0,
                &mut len,
            )
        } {
            return None;
        }
        let mut buf = vec![0u8; len];
        let mut full = 0usize;
        if !unsafe {
            f(
                self.sim.state(),
                self.handle,
                buf.as_mut_ptr(),
                buf.len(),
                &mut full,
            )
        } {
            return None;
        }
        buf.truncate(full.min(len));
        String::from_utf8(buf).ok()
    }
}

macro_rules! player_getter {
    ($name:ident, $slot:ident, $ret:ty, $default:expr) => {
        pub fn $name(&self) -> $ret {
            slot!(self.sim.sim(), SimVtableV1, $slot)
                .map_or($default, |f| unsafe { f(self.sim.state(), self.handle) })
        }
    };
}

/// Player view; valid only within the callback that produced it.
pub struct StablePlayer<'s, 'a> {
    sim: &'s StableSim<'a>,
    handle: PlayerHandleV1,
}

impl<'s, 'a> StablePlayer<'s, 'a> {
    pub fn handle(&self) -> PlayerHandleV1 {
        self.handle
    }

    pub fn id(&self) -> usize {
        self.handle.id().unwrap_or(usize::MAX)
    }

    pub fn champion(&self) -> Option<StableEntity<'s, 'a>> {
        let handle = slot!(self.sim.sim(), SimVtableV1, player_champion)
            .map_or(EntityHandleV1::NULL, |f| unsafe {
                f(self.sim.state(), self.handle)
            });
        self.sim.entity(handle)
    }

    pub fn lane(&self) -> Option<LaneV1> {
        let code = slot!(self.sim.sim(), SimVtableV1, player_lane)
            .map_or(u32::MAX, |f| unsafe { f(self.sim.state(), self.handle) });
        LaneV1::from_code(code)
    }

    player_getter!(level, player_level, usize, 1);
    player_getter!(gold, player_gold, usize, 0);
    player_getter!(team, player_team, usize, 0);
    player_getter!(is_alive, player_is_alive, bool, false);
    player_getter!(respawn_time, player_respawn_time, usize, 0);
    player_getter!(kills, player_kills, usize, 0);
    player_getter!(deaths, player_deaths, usize, 0);
    player_getter!(assists, player_assists, usize, 0);
    player_getter!(cs, player_cs, usize, 0);
    player_getter!(item_count, player_item_count, usize, 0);

    /// Remaining cooldown ticks as (attack, skill, skill2, ult).
    pub fn cooldowns(&self) -> Option<(usize, usize, usize, usize)> {
        let f = slot!(self.sim.sim(), SimVtableV1, player_cooldowns)?;
        let (mut attack, mut skill, mut skill2, mut ult) = (0usize, 0usize, 0usize, 0usize);
        unsafe {
            f(
                self.sim.state(),
                self.handle,
                &mut attack,
                &mut skill,
                &mut skill2,
                &mut ult,
            )
        }
        .then_some((attack, skill, skill2, ult))
    }

    /// Reads a fragment of the player's cumulative match statistics document
    /// (rating, solo kills, exp breakdown, ...) as JSON.
    pub fn statistics_json(&self, path: &str) -> Option<String> {
        let f = slot!(self.sim.sim(), SimVtableV1, player_statistics_json)?;
        let mut len = 0usize;
        if !unsafe {
            f(
                self.sim.state(),
                self.handle,
                path.as_ptr(),
                path.len(),
                std::ptr::null_mut(),
                0,
                &mut len,
            )
        } {
            return None;
        }
        let mut buf = vec![0u8; len];
        let mut full = 0usize;
        if !unsafe {
            f(
                self.sim.state(),
                self.handle,
                path.as_ptr(),
                path.len(),
                buf.as_mut_ptr(),
                buf.len(),
                &mut full,
            )
        } {
            return None;
        }
        buf.truncate(full.min(len));
        String::from_utf8(buf).ok()
    }

    /// Keys of every item the player currently owns.
    pub fn item_keys(&self) -> Vec<String> {
        let Some(f) = slot!(self.sim.sim(), SimVtableV1, player_item_key_at) else {
            return Vec::new();
        };
        (0..self.item_count())
            .filter_map(|index| {
                let mut len = 0usize;
                if !unsafe {
                    f(
                        self.sim.state(),
                        self.handle,
                        index,
                        std::ptr::null_mut(),
                        0,
                        &mut len,
                    )
                } {
                    return None;
                }
                let mut buf = vec![0u8; len];
                let mut full = 0usize;
                if !unsafe {
                    f(
                        self.sim.state(),
                        self.handle,
                        index,
                        buf.as_mut_ptr(),
                        buf.len(),
                        &mut full,
                    )
                } {
                    return None;
                }
                buf.truncate(full.min(len));
                String::from_utf8(buf).ok()
            })
            .collect()
    }
}
