//! What a mod hands back to the game from `tfm2_mod_entry_stable`.
//!
//! The export struct is allocated by the mod. The host reads only fields
//! that fit inside `size` (an older mod's struct is a valid prefix of a
//! newer host's definition), copies what it needs, then calls `destroy`
//! exactly once so the mod's own allocator frees the container.
//!
//! Content registrations (champions, items, ...) are appended in M1 as
//! `(ptr, len)` arrays of `{ userdata, vtable }` entries; ownership of those
//! entries transfers to the host and follows the process-lifetime DLL policy.

use crate::{
    ChampionRegV1, DraftHookRegV1, ExtensionRegV1, ItemRegV1, MapCustomizerRegV1, MatchHookRegV1,
    NativeEffectRegV1, PlayerAiRegV1, ServerExtRegV1,
};

/// Mod export container, level 1 layout.
#[repr(C)]
pub struct ModExportV1 {
    pub size: usize,
    /// Level this mod requires. Must match `tfm2_mod_required_abi_level()`;
    /// the loader gates on the symbol before calling entry.
    pub required_abi_level: u32,
    /// Mod id, UTF-8, owned by the mod. Valid until `destroy` runs.
    pub mod_id_ptr: *const u8,
    pub mod_id_len: usize,
    /// Frees this export container (not content userdata). Called once by
    /// the host after it copied everything out.
    pub destroy: Option<unsafe extern "C" fn(export: *mut ModExportV1)>,
    /// Registered gameplay content. The arrays themselves are freed by
    /// `destroy`; ownership of each entry's userdata transfers to the host,
    /// which releases it through the entry vtable's own `destroy`.
    pub champions_ptr: *const ChampionRegV1,
    pub champions_len: usize,
    pub items_ptr: *const ItemRegV1,
    pub items_len: usize,
    pub native_effects_ptr: *const NativeEffectRegV1,
    pub native_effects_len: usize,
    /// Server extension; `ServerExtRegV1::NULL` when the mod has none.
    pub server_ext: ServerExtRegV1,
    /// Client extension; `ExtensionRegV1::NULL` when the mod has none.
    pub extension: ExtensionRegV1,
    pub draft_hooks_ptr: *const DraftHookRegV1,
    pub draft_hooks_len: usize,
    pub player_ai_ptr: *const PlayerAiRegV1,
    pub player_ai_len: usize,
    /// Map customizer; `MapCustomizerRegV1::NULL` when the mod has none.
    pub map_customizer: MapCustomizerRegV1,
    /// Match hook; `MatchHookRegV1::NULL` when the mod has none.
    pub match_hook: MatchHookRegV1,
    // -- level 2 --
    pub item_build_hooks_ptr: *const crate::ItemBuildHookRegV1,
    pub item_build_hooks_len: usize,
}
