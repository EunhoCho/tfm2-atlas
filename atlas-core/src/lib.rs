//! Shared Teamfight Manager 2 Stable ABI runtime for Atlas Core and Editor.

mod api;
mod editor;
mod index;
mod mock_draft;
mod runtime;

pub use api::{
    ApiCommand, ApiError, ApiRequest, ApiResponse, ApiSurface, DraftComposition, DraftEvaluation,
    DraftRecommendation, DraftRule, DraftSide, DraftTeamOption, MasteryEdit, MockDraftAction,
    MockDraftContextUpdate, MockDraftSet, MockDraftState, MovePlayerReadback, PlayerContractEdit,
    PlayerEditReadback, PlayerEditRequest, StaffContractEdit, StaffEditRequest,
};
pub use index::{
    CachedMatchId, ChampionAnalyticsRow, Division, ItemUsageRow, MatchKind, MatchIndex, MatchIndexCache,
    MatchIndexCacheManifest, AnalyticsState, RelationRow, ReplayError, ReplaySummary,
    TeamChampionPreference, MATCH_INDEX_CACHE_CHUNK_SIZE, MATCH_INDEX_CACHE_VERSION,
    MATCH_INDEX_FORMULA_VERSION,
};
pub use mock_draft::{
    carryover_excluded as mock_carryover_excluded, MockDraftError, MockDraftSession,
};
pub use runtime::{
    validate_community_shapes, CatalogStatus, ChampionBriefState, ChampionCatalog,
    ChampionMasteryState, ChampionMasteryValue, ChampionStatState, Diagnostic, EngineStatus,
    IndexCacheState, IndexProgress, RuntimePublicState, TierApplicationState,
};

#[cfg(feature = "core-entry")]
use mod_api_stable::declare_stable_mod;
use mod_api_stable::{LogLevel, StableHost, StableMod};

#[cfg(feature = "core-entry")]
const MOD_ID: &str = "tfm2_atlas_core";

#[cfg(feature = "core-entry")]
fn init(host: &StableHost) -> StableMod {
    let game = host.game_version();
    host.log(
        LogLevel::Info,
        &format!(
            "TFM2 Atlas Core 1.0.33 loaded for game {}.{}.{} (ABI {})",
            game.major,
            game.minor,
            game.patch,
            host.abi_level()
        ),
    );
    runtime::start_bridge_server(ApiSurface::Core, runtime::CORE_BRIDGE_ADDR);
    let mut stable_mod = StableMod::new(MOD_ID);
    stable_mod.set_extension(runtime::ClientExtension::core());
    stable_mod.set_server_extension(runtime::ServerExtension::core());
    stable_mod
}

#[cfg(feature = "core-entry")]
declare_stable_mod!(init, requires = 3);

pub fn create_editor_mod(host: &StableHost) -> StableMod {
    let game = host.game_version();
    host.log(
        LogLevel::Info,
        &format!(
            "TFM2 Atlas Editor 1.0.33 loaded for game {}.{}.{} (ABI {})",
            game.major,
            game.minor,
            game.patch,
            host.abi_level()
        ),
    );
    runtime::start_bridge_server(ApiSurface::Editor, runtime::EDITOR_BRIDGE_ADDR);
    let mut stable_mod = StableMod::new("tfm2_atlas_editor");
    stable_mod.set_extension(runtime::ClientExtension::editor());
    stable_mod.set_server_extension(runtime::ServerExtension::editor());
    stable_mod
}
