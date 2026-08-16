//! Mod-side safe wrappers for the AI-hook contexts. Compiled into the mod.

use std::mem::size_of;

use crate::{
    AiCtxV1, AiInitV1, AiVtableV1, DifficultyV1, DraftCtxV1, DraftPhaseV1, InputV1, LaneV1,
};

/// Read-only view of one player's item-build decision.
pub struct StableItemBuildContext<'a> {
    raw: &'a crate::ItemBuildCtxV1,
}

impl<'a> StableItemBuildContext<'a> {
    /// # Safety
    /// `raw` must point at a live `ItemBuildCtxV1` for the call duration.
    pub unsafe fn from_raw(raw: *const crate::ItemBuildCtxV1) -> Option<Self> {
        if raw.is_null() || (*raw).size < size_of::<crate::ItemBuildCtxV1>() {
            return None;
        }
        Some(Self { raw: &*raw })
    }

    pub fn team(&self) -> usize {
        self.raw.team
    }

    pub fn lane(&self) -> Option<LaneV1> {
        LaneV1::from_code(self.raw.lane)
    }

    pub fn champion_key(&self) -> &'a str {
        unsafe { self.raw.champion_key.as_str() }
    }

    fn strs(&self, ptr: *const crate::StrV1, len: usize) -> &'a [crate::StrV1] {
        if ptr.is_null() || len == 0 {
            &[]
        } else {
            unsafe { std::slice::from_raw_parts(ptr, len) }
        }
    }

    fn nums(&self, ptr: *const usize, len: usize) -> &'a [usize] {
        if ptr.is_null() || len == 0 {
            &[]
        } else {
            unsafe { std::slice::from_raw_parts(ptr, len) }
        }
    }

    /// Every selectable item key, in engine order — candidate indices and the
    /// ids returned from `decide_build` index into this.
    pub fn item_keys(&self) -> Vec<&'a str> {
        self.strs(self.raw.item_keys_ptr, self.raw.item_keys_len)
            .iter()
            .map(|s| unsafe { s.as_str() })
            .collect()
    }

    pub fn item_count(&self) -> usize {
        self.raw.item_keys_len
    }

    pub fn item_key(&self, index: usize) -> Option<&'a str> {
        self.strs(self.raw.item_keys_ptr, self.raw.item_keys_len)
            .get(index)
            .map(|s| unsafe { s.as_str() })
    }

    /// Index of an item by key, or `None` when the item is not selectable.
    pub fn item_index(&self, key: &str) -> Option<usize> {
        self.strs(self.raw.item_keys_ptr, self.raw.item_keys_len)
            .iter()
            .position(|s| unsafe { s.as_str() } == key)
    }

    pub fn item_category(&self, index: usize) -> Option<crate::ItemCategoryV1> {
        if self.raw.item_categories_ptr.is_null() || index >= self.raw.item_keys_len {
            return None;
        }
        crate::ItemCategoryV1::from_code(unsafe { *self.raw.item_categories_ptr.add(index) })
    }

    pub fn item_tier(&self, index: usize) -> Option<usize> {
        if self.raw.item_tiers_ptr.is_null() || index >= self.raw.item_keys_len {
            return None;
        }
        Some(unsafe { *self.raw.item_tiers_ptr.add(index) })
    }

    /// The engine's current pick, as indices into `item_keys`.
    pub fn base_build(&self) -> &'a [usize] {
        self.nums(self.raw.base_build_ptr, self.raw.base_build_len)
    }

    pub fn ally_champions(&self) -> Vec<&'a str> {
        self.strs(self.raw.ally_champions_ptr, self.raw.ally_champions_len)
            .iter()
            .map(|s| unsafe { s.as_str() })
            .collect()
    }

    pub fn enemy_champions(&self) -> Vec<&'a str> {
        self.strs(self.raw.enemy_champions_ptr, self.raw.enemy_champions_len)
            .iter()
            .map(|s| unsafe { s.as_str() })
            .collect()
    }
}

/// Read-only view of the draft state during a score hook call.
pub struct StableDraftContext<'a> {
    raw: &'a DraftCtxV1,
}

impl<'a> StableDraftContext<'a> {
    /// # Safety
    /// `raw` must point at a live `DraftCtxV1` for the call duration.
    pub unsafe fn from_raw(raw: *const DraftCtxV1) -> Option<Self> {
        // Require only the level-1 M4 prefix (through `enemy_pick_len`);
        // later fields are size-guarded individually.
        let min = std::mem::offset_of!(DraftCtxV1, enemy_pick_len) + size_of::<usize>();
        if raw.is_null() || (*raw).size < min {
            return None;
        }
        Some(Self { raw: &*raw })
    }

    pub fn phase(&self) -> Option<DraftPhaseV1> {
        DraftPhaseV1::from_code(self.raw.phase)
    }

    pub fn difficulty(&self) -> Option<DifficultyV1> {
        DifficultyV1::from_code(self.raw.difficulty)
    }

    pub fn is_explore(&self) -> bool {
        self.raw.is_explore
    }

    fn read_slice(&self, ptr: *const usize, len: usize) -> &'a [usize] {
        if ptr.is_null() || len == 0 {
            &[]
        } else {
            unsafe { std::slice::from_raw_parts(ptr, len) }
        }
    }

    pub fn available_champions(&self) -> &'a [usize] {
        self.read_slice(self.raw.available_ptr, self.raw.available_len)
    }

    pub fn ally_bans(&self) -> &'a [usize] {
        self.read_slice(self.raw.ally_ban_ptr, self.raw.ally_ban_len)
    }

    pub fn enemy_bans(&self) -> &'a [usize] {
        self.read_slice(self.raw.enemy_ban_ptr, self.raw.enemy_ban_len)
    }

    pub fn ally_picks(&self) -> &'a [usize] {
        self.read_slice(self.raw.ally_pick_ptr, self.raw.ally_pick_len)
    }

    pub fn enemy_picks(&self) -> &'a [usize] {
        self.read_slice(self.raw.enemy_pick_ptr, self.raw.enemy_pick_len)
    }

    // -- champion identities (the draft usize ids index into these) --

    /// All champion identities, or empty on hosts older than this surface.
    pub fn champion_briefs(&self) -> &'a [crate::ChampionBriefV1] {
        let end = std::mem::offset_of!(DraftCtxV1, champion_briefs_len) + size_of::<usize>();
        if self.raw.size < end
            || self.raw.champion_briefs_ptr.is_null()
            || self.raw.champion_briefs_len == 0
        {
            return &[];
        }
        unsafe {
            std::slice::from_raw_parts(self.raw.champion_briefs_ptr, self.raw.champion_briefs_len)
        }
    }

    pub fn champion_brief(&self, champion_id: usize) -> Option<&'a crate::ChampionBriefV1> {
        self.champion_briefs().get(champion_id)
    }

    pub fn champion_name(&self, champion_id: usize) -> Option<&'a str> {
        Some(unsafe { self.champion_brief(champion_id)?.name.as_str() })
    }

    pub fn champion_category(&self, champion_id: usize) -> Option<crate::ChampionCategoryV1> {
        crate::ChampionCategoryV1::from_code(self.champion_brief(champion_id)?.category)
    }

    pub fn champion_tags(&self, champion_id: usize) -> Vec<crate::ChampionTagV1> {
        let Some(brief) = self.champion_brief(champion_id) else {
            return Vec::new();
        };
        brief.tags[..(brief.tag_count as usize).min(brief.tags.len())]
            .iter()
            .filter_map(|code| crate::ChampionTagV1::from_code(*code))
            .collect()
    }

    pub fn champion_stat(&self, champion_id: usize) -> Option<crate::StatV1> {
        Some(self.champion_brief(champion_id)?.stat)
    }

    pub fn champion_growth(&self, champion_id: usize) -> Option<crate::StatV1> {
        Some(self.champion_brief(champion_id)?.growth)
    }
}

/// Draft score decision returned by a stable hook.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum StableDraftDecision {
    Pass,
    Add(f32),
    Replace(f32),
}

/// Static player identity for `matches`.
#[derive(Clone, Debug)]
pub struct StableAiInit {
    pub player_id: usize,
    pub athlete_id: usize,
    pub team: usize,
    pub lane: Option<LaneV1>,
    pub champion_name: String,
}

impl StableAiInit {
    /// # Safety
    /// `raw` must point at a live `AiInitV1` for the call duration.
    pub unsafe fn from_raw(raw: *const AiInitV1) -> Option<Self> {
        if raw.is_null() || (*raw).size < size_of::<AiInitV1>() {
            return None;
        }
        let init = &*raw;
        Some(Self {
            player_id: init.player_id,
            athlete_id: init.athlete_id,
            team: init.team,
            lane: LaneV1::from_code(init.lane),
            champion_name: init.champion_name.as_str().to_string(),
        })
    }
}

/// Size-guarded view over the per-tick AI context. Valid only inside `think`.
pub struct StableAiContext<'a> {
    raw: *mut AiCtxV1,
    /// Backing storage for [`Self::sim`] (the sim wrapper borrows a
    /// `SimCtxV1`, which the AI boundary does not carry directly).
    sim_ctx: Option<Box<crate::SimCtxV1>>,
    _marker: std::marker::PhantomData<&'a mut AiCtxV1>,
}

impl StableAiContext<'_> {
    /// # Safety
    /// `raw` must point at a live `AiCtxV1` for the call duration.
    pub unsafe fn from_raw(raw: *mut AiCtxV1) -> Option<Self> {
        // Require only the level-1 M4 prefix (through `state`); the sim
        // fields appended later are size-guarded in `sim()`.
        let min = std::mem::offset_of!(AiCtxV1, state) + size_of::<*mut std::ffi::c_void>();
        if raw.is_null() || (*raw).size < min {
            return None;
        }
        Some(Self {
            raw,
            sim_ctx: None,
            _marker: std::marker::PhantomData,
        })
    }

    /// Read-only view of the running simulation, if the host provides one.
    /// Mutation calls on it are inert inside `think` — the AI contract is a
    /// pure input decision.
    pub fn sim(&mut self) -> Option<crate::StableSim<'_>> {
        let end = std::mem::offset_of!(AiCtxV1, sim_state) + size_of::<*mut std::ffi::c_void>();
        if unsafe { (*self.raw).size } < end {
            return None;
        }
        let sim = unsafe { (*self.raw).sim };
        let state = unsafe { (*self.raw).sim_state };
        if sim.is_null() || state.is_null() {
            return None;
        }
        let ctx = self.sim_ctx.get_or_insert_with(|| {
            Box::new(crate::SimCtxV1 {
                size: size_of::<crate::SimCtxV1>(),
                sim: std::ptr::null(),
                frame: std::ptr::null(),
                state: std::ptr::null_mut(),
            })
        });
        ctx.sim = sim;
        ctx.state = state;
        unsafe { crate::StableSim::from_raw(&mut **ctx) }
    }

    fn table(&self) -> *const AiVtableV1 {
        unsafe { (*self.raw).vtable }
    }

    fn state(&self) -> *mut std::ffi::c_void {
        unsafe { (*self.raw).state }
    }

    pub fn player_id(&self) -> usize {
        slot!(self.table(), AiVtableV1, player_id).map_or(0, |f| unsafe { f(self.state()) })
    }

    pub fn athlete_id(&self) -> usize {
        slot!(self.table(), AiVtableV1, athlete_id).map_or(0, |f| unsafe { f(self.state()) })
    }

    pub fn team(&self) -> usize {
        slot!(self.table(), AiVtableV1, team).map_or(0, |f| unsafe { f(self.state()) })
    }

    pub fn lane(&self) -> Option<LaneV1> {
        let code =
            slot!(self.table(), AiVtableV1, lane).map_or(u32::MAX, |f| unsafe { f(self.state()) });
        LaneV1::from_code(code)
    }

    pub fn champion_name(&self) -> Option<String> {
        let f = slot!(self.table(), AiVtableV1, champion_name)?;
        let mut len = 0usize;
        if !unsafe { f(self.state(), std::ptr::null_mut(), 0, &mut len) } {
            return None;
        }
        let mut buf = vec![0u8; len];
        let mut full = 0usize;
        if !unsafe { f(self.state(), buf.as_mut_ptr(), buf.len(), &mut full) } {
            return None;
        }
        buf.truncate(full.min(len));
        String::from_utf8(buf).ok()
    }

    pub fn tick(&self) -> usize {
        slot!(self.table(), AiVtableV1, tick).map_or(0, |f| unsafe { f(self.state()) })
    }

    pub fn hp(&self) -> Option<usize> {
        let f = slot!(self.table(), AiVtableV1, hp)?;
        let mut out = 0usize;
        unsafe { f(self.state(), &mut out) }.then_some(out)
    }

    pub fn max_hp(&self) -> Option<usize> {
        let f = slot!(self.table(), AiVtableV1, max_hp)?;
        let mut out = 0usize;
        unsafe { f(self.state(), &mut out) }.then_some(out)
    }

    pub fn hp_ratio_percent(&self) -> Option<usize> {
        let f = slot!(self.table(), AiVtableV1, hp_ratio_percent)?;
        let mut out = 0usize;
        unsafe { f(self.state(), &mut out) }.then_some(out)
    }

    pub fn is_hp_below_percent(&self, threshold: usize) -> bool {
        self.hp_ratio_percent()
            .is_some_and(|ratio| ratio < threshold)
    }

    pub fn is_valid_input(&self, input: &InputV1) -> bool {
        slot!(self.table(), AiVtableV1, is_valid_input)
            .map_or(false, |f| unsafe { f(self.state(), input) })
    }

    pub fn run_away_input(&mut self) -> Option<InputV1> {
        let f = slot!(self.table(), AiVtableV1, run_away_input)?;
        let mut out = InputV1::default();
        unsafe { f(self.state(), &mut out) }.then_some(out)
    }

    pub fn run_away_without_skill_input(&mut self) -> Option<InputV1> {
        let f = slot!(self.table(), AiVtableV1, run_away_without_skill_input)?;
        let mut out = InputV1::default();
        unsafe { f(self.state(), &mut out) }.then_some(out)
    }

    pub fn recall_input(&mut self) -> Option<InputV1> {
        let f = slot!(self.table(), AiVtableV1, recall_input)?;
        let mut out = InputV1::default();
        unsafe { f(self.state(), &mut out) }.then_some(out)
    }

    pub fn is_safe_to_recall(&mut self) -> bool {
        slot!(self.table(), AiVtableV1, is_safe_to_recall)
            .map_or(false, |f| unsafe { f(self.state()) })
    }
}
