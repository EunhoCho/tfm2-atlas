//! AI-hook boundary: draft (ban/pick) scoring and per-tick player input.
//!
//! `f32` is part of this contract surface (draft scoring is engine-side
//! f32 already and lives outside the deterministic simulation zone).

use std::ffi::c_void;

use crate::{InputV1, StrV1};

/// Read-only draft context passed to score hooks. Slices borrow host memory
/// and are valid only for the duration of the call.
#[repr(C)]
pub struct DraftCtxV1 {
    pub size: usize,
    /// [`crate::DraftPhaseV1`] code.
    pub phase: u32,
    /// [`crate::DifficultyV1`] code.
    pub difficulty: u32,
    pub is_explore: bool,
    pub available_ptr: *const usize,
    pub available_len: usize,
    pub ally_ban_ptr: *const usize,
    pub ally_ban_len: usize,
    pub enemy_ban_ptr: *const usize,
    pub enemy_ban_len: usize,
    pub ally_pick_ptr: *const usize,
    pub ally_pick_len: usize,
    pub enemy_pick_ptr: *const usize,
    pub enemy_pick_len: usize,
    /// Champion identities the draft usize ids index into (name/category/
    /// tags/stats). Empty on hosts older than this field.
    pub champion_briefs_ptr: *const crate::ChampionBriefV1,
    pub champion_briefs_len: usize,
}

/// Mod-implemented draft score hook. Score slots return an
/// [`crate::DraftDecisionKindV1`] code and write `out_score` for Add/Replace.
#[repr(C)]
pub struct DraftHookVtableV1 {
    pub size: usize,
    pub destroy: Option<unsafe extern "C" fn(userdata: *mut c_void)>,
    pub id: Option<unsafe extern "C" fn(userdata: *const c_void) -> StrV1>,
    pub priority: Option<unsafe extern "C" fn(userdata: *const c_void) -> i32>,
    pub score_ban: Option<
        unsafe extern "C" fn(
            userdata: *const c_void,
            ctx: *const DraftCtxV1,
            candidate: usize,
            base_score: f32,
            out_score: *mut f32,
        ) -> u32,
    >,
    pub score_pick: Option<
        unsafe extern "C" fn(
            userdata: *const c_void,
            ctx: *const DraftCtxV1,
            candidate: usize,
            base_score: f32,
            out_score: *mut f32,
        ) -> u32,
    >,
    /// Full override: write the exact champion id to ban and return true.
    /// Ignored when the id is not among the current candidates.
    pub decide_ban: Option<
        unsafe extern "C" fn(
            userdata: *const c_void,
            ctx: *const DraftCtxV1,
            out_champion: *mut usize,
        ) -> bool,
    >,
    /// Full override for picks, same rules as `decide_ban`.
    pub decide_pick: Option<
        unsafe extern "C" fn(
            userdata: *const c_void,
            ctx: *const DraftCtxV1,
            out_champion: *mut usize,
        ) -> bool,
    >,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct DraftHookRegV1 {
    pub userdata: *mut c_void,
    pub vtable: *const DraftHookVtableV1,
}

impl DraftHookRegV1 {
    pub const NULL: Self = Self {
        userdata: std::ptr::null_mut(),
        vtable: std::ptr::null(),
    };

    pub fn is_null(&self) -> bool {
        self.vtable.is_null()
    }
}

/// Read-only context for item-build hooks, handed out once per player right
/// after the engine picked that player's final item targets for a match.
/// Slices and string views borrow host memory for the duration of the call.
#[repr(C)]
pub struct ItemBuildCtxV1 {
    pub size: usize,
    pub team: usize,
    /// [`crate::LaneV1`] code for the player this build belongs to.
    pub lane: u32,
    pub champion_key: StrV1,
    /// Every selectable item key, in engine order. `candidate` indices in the
    /// score slot and the ids written by `decide_build` index into this.
    pub item_keys_ptr: *const StrV1,
    pub item_keys_len: usize,
    /// [`crate::ItemCategoryV1`] code per entry of `item_keys`.
    pub item_categories_ptr: *const u32,
    pub item_tiers_ptr: *const usize,
    /// The engine's current pick, as indices into `item_keys`.
    pub base_build_ptr: *const usize,
    pub base_build_len: usize,
    /// Ally/enemy champion keys of the match this build is for.
    pub ally_champions_ptr: *const StrV1,
    pub ally_champions_len: usize,
    pub enemy_champions_ptr: *const StrV1,
    pub enemy_champions_len: usize,
}

/// Mod-implemented item-build hook — the typed replacement for reaching into
/// the scene to rewrite builds. `score_item` returns an
/// [`crate::DraftDecisionKindV1`] code and writes `out_score` for Add/Replace.
#[repr(C)]
pub struct ItemBuildHookVtableV1 {
    pub size: usize,
    pub destroy: Option<unsafe extern "C" fn(userdata: *mut c_void)>,
    pub id: Option<unsafe extern "C" fn(userdata: *const c_void) -> StrV1>,
    pub priority: Option<unsafe extern "C" fn(userdata: *const c_void) -> i32>,
    /// Adjusts the engine's score for one candidate final item.
    pub score_item: Option<
        unsafe extern "C" fn(
            userdata: *const c_void,
            ctx: *const ItemBuildCtxV1,
            candidate: usize,
            base_score: f32,
            out_score: *mut f32,
        ) -> u32,
    >,
    /// Full override: write up to `cap` item indices into `out_build` and
    /// return how many were written (0 = keep the engine's build). Indices
    /// outside `item_keys` are dropped. Of multiple hooks the highest
    /// priority that decides wins.
    pub decide_build: Option<
        unsafe extern "C" fn(
            userdata: *const c_void,
            ctx: *const ItemBuildCtxV1,
            out_build: *mut usize,
            cap: usize,
        ) -> usize,
    >,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct ItemBuildHookRegV1 {
    pub userdata: *mut c_void,
    pub vtable: *const ItemBuildHookVtableV1,
}

impl ItemBuildHookRegV1 {
    pub const NULL: Self = Self {
        userdata: std::ptr::null_mut(),
        vtable: std::ptr::null(),
    };

    pub fn is_null(&self) -> bool {
        self.vtable.is_null()
    }
}

/// Static identity of the player a per-tick AI may attach to. String views
/// are valid only for the duration of the `matches` call.
#[repr(C)]
pub struct AiInitV1 {
    pub size: usize,
    pub player_id: usize,
    pub athlete_id: usize,
    pub team: usize,
    /// [`crate::LaneV1`] code.
    pub lane: u32,
    pub champion_name: StrV1,
}

/// Per-call player AI context. Valid only for the duration of `think`.
#[repr(C)]
pub struct AiCtxV1 {
    pub size: usize,
    pub vtable: *const AiVtableV1,
    pub state: *mut c_void,
    /// Read-only view of the running simulation (the full sim query table;
    /// its mutation slots are inert inside `think` — AI must stay a pure
    /// input decision). Null on hosts older than this field.
    pub sim: *const crate::SimVtableV1,
    pub sim_state: *mut c_void,
}

/// Host helpers available inside `think`, mirroring the legacy
/// `PlayerAiContext` helper methods.
#[repr(C)]
pub struct AiVtableV1 {
    pub size: usize,
    pub player_id: Option<unsafe extern "C" fn(state: *const c_void) -> usize>,
    pub athlete_id: Option<unsafe extern "C" fn(state: *const c_void) -> usize>,
    pub team: Option<unsafe extern "C" fn(state: *const c_void) -> usize>,
    /// [`crate::LaneV1`] code.
    pub lane: Option<unsafe extern "C" fn(state: *const c_void) -> u32>,
    pub champion_name: Option<
        unsafe extern "C" fn(
            state: *const c_void,
            buf: *mut u8,
            buf_cap: usize,
            out_len: *mut usize,
        ) -> bool,
    >,
    pub tick: Option<unsafe extern "C" fn(state: *const c_void) -> usize>,
    pub hp: Option<unsafe extern "C" fn(state: *const c_void, out: *mut usize) -> bool>,
    pub max_hp: Option<unsafe extern "C" fn(state: *const c_void, out: *mut usize) -> bool>,
    pub hp_ratio_percent:
        Option<unsafe extern "C" fn(state: *const c_void, out: *mut usize) -> bool>,
    pub is_valid_input:
        Option<unsafe extern "C" fn(state: *const c_void, input: *const InputV1) -> bool>,
    pub run_away_input: Option<unsafe extern "C" fn(state: *mut c_void, out: *mut InputV1) -> bool>,
    pub run_away_without_skill_input:
        Option<unsafe extern "C" fn(state: *mut c_void, out: *mut InputV1) -> bool>,
    pub recall_input: Option<unsafe extern "C" fn(state: *mut c_void, out: *mut InputV1) -> bool>,
    pub is_safe_to_recall: Option<unsafe extern "C" fn(state: *mut c_void) -> bool>,
}

/// Mod-implemented per-tick player AI. `think` returns an
/// [`crate::AiDecisionKindV1`] code and writes `out_input` on Replace.
#[repr(C)]
pub struct PlayerAiVtableV1 {
    pub size: usize,
    pub destroy: Option<unsafe extern "C" fn(userdata: *mut c_void)>,
    pub clone: Option<unsafe extern "C" fn(userdata: *const c_void) -> *mut c_void>,
    pub id: Option<unsafe extern "C" fn(userdata: *const c_void) -> StrV1>,
    pub priority: Option<unsafe extern "C" fn(userdata: *const c_void) -> i32>,
    pub matches:
        Option<unsafe extern "C" fn(userdata: *const c_void, init: *const AiInitV1) -> bool>,
    pub think: Option<
        unsafe extern "C" fn(
            userdata: *mut c_void,
            ctx: *mut AiCtxV1,
            base_input: *const InputV1,
            out_input: *mut InputV1,
        ) -> u32,
    >,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct PlayerAiRegV1 {
    pub userdata: *mut c_void,
    pub vtable: *const PlayerAiVtableV1,
}

impl PlayerAiRegV1 {
    pub const NULL: Self = Self {
        userdata: std::ptr::null_mut(),
        vtable: std::ptr::null(),
    };

    pub fn is_null(&self) -> bool {
        self.vtable.is_null()
    }
}
