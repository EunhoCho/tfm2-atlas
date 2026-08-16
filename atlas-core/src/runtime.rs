use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::{
        atomic::{AtomicBool, AtomicU64, AtomicU8, AtomicUsize, Ordering},
        mpsc::{self, Receiver, Sender},
        Arc, Mutex, OnceLock, RwLock,
    },
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use mod_api_stable::{
    CommandResultV1, ManagementEventKindV1, RecordKindV1, StableClient, StableCommand,
    StableExtension, StableServerCtx, StableServerExtension,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tfm2_atlas_engine::{
    analyze_role_gate, apply_lock_to_record, apply_tiers_to_team, candidate_passes_role_gate,
    decode_json_frame, encode_json_frame, evaluate_draft_side, parse_tier_tsv, render_tier_tsv_v2,
    DraftCandidateInput, DraftPhase, DraftRoleConfidence, DraftScoreWeights, LockGroup, LockSet,
    LockStatus, RoleFilter, Tier, TierPolicyRow, TierProfile, MAX_FRAME_BYTES,
};

use crate::{
    editor, mock_carryover_excluded, AnalyticsState, ApiCommand, ApiRequest, ApiResponse,
    ApiSurface, Division, DraftComposition, DraftEvaluation, DraftRecommendation, DraftSide,
    DraftTeamOption, MatchIndex, MatchIndexCache, MatchIndexCacheManifest, MatchKind,
    MockDraftAction, MockDraftContextUpdate, MockDraftSession, MockDraftState, MovePlayerReadback,
    PlayerEditReadback, PlayerEditRequest, StaffEditRequest,
};

#[derive(Debug, Clone, PartialEq, Eq)]
struct ActiveChampionCatalogEntry {
    champion_id: String,
    positions: Vec<RoleFilter>,
}

fn catalog_for_active_ids(
    active_champions: &[ActiveChampionCatalogEntry],
    metadata: &BTreeMap<String, ChampionBriefState>,
) -> Vec<ChampionBriefState> {
    let mut entries = active_champions.to_vec();
    entries.sort_by(|left, right| left.champion_id.cmp(&right.champion_id));
    entries.dedup_by(|left, right| left.champion_id == right.champion_id);
    entries
        .into_iter()
        .map(|entry| {
            let mut champion = metadata
                .get(&entry.champion_id)
                .cloned()
                .unwrap_or_else(|| {
                    let display_name = readable_champion_name(&entry.champion_id);
                    ChampionBriefState {
                        champion_id: entry.champion_id.clone(),
                        display_name,
                        category: None,
                        tags: Vec::new(),
                        positions: Vec::new(),
                        main_position: None,
                        stat: champion_stat(mod_api_stable::StatV1::default()),
                        growth: champion_stat(mod_api_stable::StatV1::default()),
                    }
                });
            champion.positions = entry.positions;
            champion.main_position = champion.positions.first().copied();
            champion
        })
        .collect()
}

fn unix_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}

pub const CORE_BRIDGE_ADDR: &str = "127.0.0.1:28452";
pub const EDITOR_BRIDGE_ADDR: &str = "127.0.0.1:28453";
const NONE_ID: usize = usize::MAX;
const INDEX_BATCH_SIZE: usize = 16;
const PROFILE_SAVE_KEY: &str = "atlas_tier_profile";
const INDEX_CACHE_MANIFEST_KEY: &str = "atlas_match_index_manifest";
const INDEX_CACHE_CHUNK_PREFIX: &str = "atlas_match_index_chunk_";
const REQUEST_PENDING: u8 = 0;
const REQUEST_PROCESSING: u8 = 1;
const REQUEST_CANCELLED: u8 = 2;
const REQUEST_DONE: u8 = 3;
const MAX_INLINE_BRIDGE_RESPONSE_BYTES: usize = 64 * 1024;
const CORE_BRIDGE_COMMAND: &str = "tfm2_atlas_core.bridge";
const CORE_BRIDGE_RESPONSE_EVENT: &str = "tfm2_atlas_core.bridge_response";
const CORE_INDEX_WORK_COMMAND: &str = "tfm2_atlas_core.index_work";
const EDITOR_BRIDGE_COMMAND: &str = "tfm2_atlas_editor.bridge";
const EDITOR_BRIDGE_RESPONSE_EVENT: &str = "tfm2_atlas_editor.bridge_response";
const INDEX_WORK_INTERVAL_MICROS: u64 = 250_000;

fn bridge_command(surface: ApiSurface) -> &'static str {
    match surface {
        ApiSurface::Core => CORE_BRIDGE_COMMAND,
        ApiSurface::Editor => EDITOR_BRIDGE_COMMAND,
    }
}

fn bridge_response_event(surface: ApiSurface) -> &'static str {
    match surface {
        ApiSurface::Core => CORE_BRIDGE_RESPONSE_EVENT,
        ApiSurface::Editor => EDITOR_BRIDGE_RESPONSE_EVENT,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EngineStatus {
    WaitingForCareer,
    ValidatingSchema,
    Indexing,
    Ready,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IndexCacheState {
    Cold,
    Restored,
    Rebuilding,
    Checkpointed,
    Complete,
    Invalid,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexProgress {
    pub total: usize,
    pub processed: usize,
    pub indexed: usize,
    pub failed: usize,
    pub pending: usize,
    pub cache_state: IndexCacheState,
}

impl Default for IndexProgress {
    fn default() -> Self {
        Self {
            total: 0,
            processed: 0,
            indexed: 0,
            failed: 0,
            pending: 0,
            cache_state: IndexCacheState::Cold,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct TierApplicationState {
    pub applied: bool,
    pub team_id: Option<usize>,
    pub profile_revision: u64,
    pub applied_champions: usize,
    pub readback_verified: bool,
    pub client_screen_verified: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChampionCatalog {
    pub revision: u64,
    pub status: CatalogStatus,
    pub champions: Vec<ChampionBriefState>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CatalogStatus {
    Loading,
    Ready,
    Unavailable,
}

impl Default for ChampionCatalog {
    fn default() -> Self {
        Self {
            revision: 0,
            status: CatalogStatus::Loading,
            champions: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChampionMasteryValue {
    pub champion_id: String,
    pub display_name: String,
    pub value: i64,
    pub floor: i64,
    pub active: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChampionMasteryState {
    pub athlete_id: usize,
    pub active: Vec<ChampionMasteryValue>,
    pub inactive: Vec<ChampionMasteryValue>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Diagnostic {
    pub code: String,
    pub message: String,
    pub fatal: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChampionStatState {
    pub attack: usize,
    pub magic_power: usize,
    pub hp: usize,
    pub defence: usize,
    pub magic_resistance: usize,
    pub move_speed: usize,
    pub hp_regen: usize,
    pub stack: usize,
    pub crit_chance: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChampionBriefState {
    pub champion_id: String,
    pub display_name: String,
    pub category: Option<String>,
    pub tags: Vec<String>,
    #[serde(default)]
    pub positions: Vec<RoleFilter>,
    pub main_position: Option<RoleFilter>,
    pub stat: ChampionStatState,
    pub growth: ChampionStatState,
}

impl Diagnostic {
    fn new(code: impl Into<String>, message: impl Into<String>, fatal: bool) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            fatal,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuntimePublicState {
    pub connected: bool,
    pub game_version: String,
    pub engine_status: EngineStatus,
    pub indexed_matches: usize,
    pub pending_records: usize,
    pub data_revision: u64,
    pub player_team_id: Option<usize>,
    pub active_profile: TierProfile,
    pub preview_profile: TierProfile,
    pub analytics: Option<AnalyticsState>,
    pub locks: Vec<LockSet>,
    pub diagnostics: Vec<Diagnostic>,
    pub champion_info: Vec<ChampionBriefState>,
    pub champion_catalog: ChampionCatalog,
    pub index_progress: IndexProgress,
    pub tier_application: TierApplicationState,
}

impl Default for RuntimePublicState {
    fn default() -> Self {
        let profile = TierProfile::default();
        Self {
            connected: false,
            game_version: "0.5.5".to_owned(),
            engine_status: EngineStatus::WaitingForCareer,
            indexed_matches: 0,
            pending_records: 0,
            data_revision: 0,
            player_team_id: None,
            active_profile: profile.clone(),
            preview_profile: profile,
            analytics: None,
            locks: Vec::new(),
            diagnostics: Vec::new(),
            champion_info: Vec::new(),
            champion_catalog: ChampionCatalog::default(),
            index_progress: IndexProgress::default(),
            tier_application: TierApplicationState::default(),
        }
    }
}

impl RuntimePublicState {
    fn mark_disconnected(&mut self) {
        self.connected = false;
        self.engine_status = EngineStatus::WaitingForCareer;
        self.indexed_matches = 0;
        self.pending_records = 0;
        self.data_revision = 0;
        self.player_team_id = None;
        self.analytics = None;
        self.locks.clear();
        self.champion_info.clear();
        self.champion_catalog = ChampionCatalog {
            revision: self.champion_catalog.revision.wrapping_add(1),
            status: CatalogStatus::Unavailable,
            champions: Vec::new(),
        };
        self.index_progress = IndexProgress::default();
        self.tier_application = TierApplicationState::default();
    }
}

#[derive(Clone, Serialize, Deserialize)]
enum BridgeRequest {
    Api(ApiRequest),
    TierSync,
}

#[derive(Clone, Serialize, Deserialize)]
enum BridgeResponse {
    Api(ApiResponse),
    TierSync(String),
    Published,
}

struct RequestEnvelope {
    request: BridgeRequest,
    response: Sender<BridgeResponse>,
    execution: Arc<AtomicU8>,
}

#[derive(Clone)]
struct ClientRecordSync {
    previous_name: Option<String>,
    record: Value,
}

#[derive(Clone, Default)]
struct ClientUiSyncState {
    athletes: BTreeMap<usize, ClientRecordSync>,
    staffs: BTreeMap<usize, ClientRecordSync>,
    champion_tiers: BTreeMap<String, String>,
}

struct BridgeHub {
    request_tx: Sender<RequestEnvelope>,
    request_rx: Mutex<Receiver<RequestEnvelope>>,
    public_state: RwLock<RuntimePublicState>,
    player_team_id: AtomicUsize,
    client_connected: AtomicBool,
    listener_started: AtomicBool,
    champion_info: RwLock<Vec<ChampionBriefState>>,
    champion_catalog: RwLock<ChampionCatalog>,
    client_ui_sync: RwLock<ClientUiSyncState>,
    tier_screen_verified: AtomicBool,
    next_route_id: AtomicU64,
    routed_requests: Mutex<BTreeMap<u64, RequestEnvelope>>,
    published_responses: Mutex<BTreeMap<u64, BridgeResponse>>,
    active_champions: RwLock<Option<Vec<ActiveChampionCatalogEntry>>>,
    subscribers: Mutex<Vec<TcpStream>>,
    catalog_epoch: AtomicU64,
}

impl BridgeHub {
    fn new() -> Self {
        let (request_tx, request_rx) = mpsc::channel();
        Self {
            request_tx,
            request_rx: Mutex::new(request_rx),
            public_state: RwLock::new(RuntimePublicState::default()),
            player_team_id: AtomicUsize::new(NONE_ID),
            client_connected: AtomicBool::new(false),
            listener_started: AtomicBool::new(false),
            champion_info: RwLock::new(Vec::new()),
            champion_catalog: RwLock::new(ChampionCatalog::default()),
            client_ui_sync: RwLock::new(ClientUiSyncState::default()),
            tier_screen_verified: AtomicBool::new(false),
            next_route_id: AtomicU64::new(1),
            routed_requests: Mutex::new(BTreeMap::new()),
            published_responses: Mutex::new(BTreeMap::new()),
            active_champions: RwLock::new(None),
            subscribers: Mutex::new(Vec::new()),
            catalog_epoch: AtomicU64::new(0),
        }
    }
}

#[derive(Serialize, Deserialize)]
struct RoutedBridgeRequest {
    route_id: u64,
    request: BridgeRequest,
}

#[derive(Serialize, Deserialize)]
struct RoutedBridgeResponse {
    route_id: u64,
    response: BridgeResponse,
}

fn hub() -> &'static Arc<BridgeHub> {
    static HUB: OnceLock<Arc<BridgeHub>> = OnceLock::new();
    HUB.get_or_init(|| Arc::new(BridgeHub::new()))
}

pub struct ClientExtension {
    surface: ApiSurface,
    index_work_micros: AtomicU64,
    last_team_id: AtomicUsize,
    seen_catalog_epoch: AtomicU64,
}

impl Default for ClientExtension {
    fn default() -> Self {
        Self::core()
    }
}

impl ClientExtension {
    pub fn core() -> Self {
        Self {
            surface: ApiSurface::Core,
            index_work_micros: AtomicU64::new(0),
            last_team_id: AtomicUsize::new(NONE_ID),
            seen_catalog_epoch: AtomicU64::new(0),
        }
    }

    pub fn editor() -> Self {
        Self {
            surface: ApiSurface::Editor,
            index_work_micros: AtomicU64::new(0),
            last_team_id: AtomicUsize::new(NONE_ID),
            seen_catalog_epoch: AtomicU64::new(0),
        }
    }
}

impl StableExtension for ClientExtension {
    fn post_update(&self, ctx: &mut StableClient<'_>, dt_micros: u64) {
        let team_id = ctx.player_team_id().unwrap_or(NONE_ID);
        hub().player_team_id.store(team_id, Ordering::Release);
        hub()
            .client_connected
            .store(team_id != NONE_ID, Ordering::Release);
        let epoch = hub().catalog_epoch.load(Ordering::Acquire);
        if self.surface == ApiSurface::Core
            && self.seen_catalog_epoch.swap(epoch, Ordering::AcqRel) != epoch
        {
            let active_champions = hub()
                .active_champions
                .read()
                .ok()
                .and_then(|champions| champions.clone());
            let metadata = active_champions
                .as_ref()
                .into_iter()
                .flatten()
                .map(|entry| {
                    let display_name = localized_champion_name(ctx, &entry.champion_id);
                    (
                        entry.champion_id.clone(),
                        champion_state(
                            entry.champion_id.clone(),
                            display_name,
                            ctx.champion_brief(&entry.champion_id),
                        ),
                    )
                })
                .collect::<BTreeMap<_, _>>();
            let champions = active_champions
                .as_ref()
                .map(|entries| catalog_for_active_ids(entries, &metadata))
                .unwrap_or_default();
            let status = if champions.is_empty() {
                CatalogStatus::Loading
            } else {
                CatalogStatus::Ready
            };
            let revision = hub()
                .champion_catalog
                .read()
                .map(|catalog| catalog.revision.wrapping_add(1))
                .unwrap_or(1);
            let catalog = ChampionCatalog {
                revision,
                status,
                champions,
            };
            if let Ok(mut published) = hub().champion_catalog.write() {
                *published = catalog.clone();
            }
            if let Ok(mut published) = hub().champion_info.write() {
                published.clone_from(&catalog.champions);
            }
            emit_state_changed(&["CATALOG_CHANGED"]);
        }
        self.last_team_id.store(team_id, Ordering::Release);
        pump_bridge_events(ctx, self.surface);
        pump_bridge_requests(ctx, self.surface);
        sync_visible_management_ui(ctx);
        if self.surface != ApiSurface::Core {
            self.index_work_micros.store(0, Ordering::Release);
            return;
        }
        let indexing = hub()
            .public_state
            .read()
            .is_ok_and(|state| state.engine_status == EngineStatus::Indexing);
        let elapsed = self
            .index_work_micros
            .fetch_add(dt_micros, Ordering::AcqRel)
            .saturating_add(dt_micros);
        if indexing && team_id != NONE_ID && elapsed >= INDEX_WORK_INTERVAL_MICROS {
            self.index_work_micros.store(0, Ordering::Release);
            let _ = ctx.send_command(CORE_INDEX_WORK_COMMAND, &[]);
        } else if !indexing {
            self.index_work_micros.store(0, Ordering::Release);
        }
    }

    fn on_end(&self) {
        hub().player_team_id.store(NONE_ID, Ordering::Release);
        hub().client_connected.store(false, Ordering::Release);
        if let Ok(mut champions) = hub().champion_info.write() {
            champions.clear();
        }
        if let Ok(mut catalog) = hub().champion_catalog.write() {
            catalog.revision = catalog.revision.wrapping_add(1);
            catalog.status = CatalogStatus::Unavailable;
            catalog.champions.clear();
        }
        hub().catalog_epoch.fetch_add(1, Ordering::AcqRel);
        if let Ok(mut sync) = hub().client_ui_sync.write() {
            *sync = ClientUiSyncState::default();
        }
        if let Ok(mut active) = hub().active_champions.write() {
            *active = None;
        }
        hub().tier_screen_verified.store(false, Ordering::Release);
        if let Ok(mut published) = hub().public_state.write() {
            published.mark_disconnected();
        }
        fail_routed_requests("Career client disconnected");
    }
}

pub struct ServerExtension {
    surface: ApiSurface,
    state: Mutex<RuntimeState>,
}

impl Default for ServerExtension {
    fn default() -> Self {
        Self::core()
    }
}

impl ServerExtension {
    pub fn core() -> Self {
        Self {
            surface: ApiSurface::Core,
            state: Mutex::new(RuntimeState::default()),
        }
    }

    pub fn editor() -> Self {
        Self {
            surface: ApiSurface::Editor,
            state: Mutex::new(RuntimeState::default()),
        }
    }
}

impl StableServerExtension for ServerExtension {
    fn on_server_start(&self, ctx: &mut StableServerCtx<'_>) {
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        match self.surface {
            ApiSurface::Core => state.reset_for_career(ctx),
            ApiSurface::Editor => state.reset_for_editor(ctx),
        }
        state.publish(self.surface);
    }

    fn before_management_tick(&self, ctx: &mut StableServerCtx<'_>) {
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        state.refresh_player_team(ctx);
        state.drain_requests(ctx, self.surface);
        if self.surface == ApiSurface::Editor && state.schema_ready {
            if let Some(team_id) = state.player_team_id(ctx) {
                editor::enforce_recruitment_overrides(ctx, team_id);
            }
        }
        if self.surface == ApiSurface::Editor {
            state.enforce_locks(ctx);
        }
        state.publish(self.surface);
    }

    fn after_management_tick(&self, ctx: &mut StableServerCtx<'_>) {
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        if self.surface == ApiSurface::Core {
            state.process_events(ctx);
            state.process_index_batch(ctx);
        }
        if self.surface == ApiSurface::Editor && state.schema_ready {
            if let Some(team_id) = state.player_team_id(ctx) {
                editor::enforce_recruitment_overrides(ctx, team_id);
            }
        }
        if self.surface == ApiSurface::Editor {
            state.enforce_locks(ctx);
        } else {
            state.enforce_tiers_on_tick(ctx);
        }
        state.publish(self.surface);
    }

    fn handle_command(
        &self,
        ctx: &mut StableServerCtx<'_>,
        command: &StableCommand<'_>,
    ) -> CommandResultV1 {
        if self.surface == ApiSurface::Core && command.command == CORE_INDEX_WORK_COMMAND {
            if let Ok(mut state) = self.state.lock() {
                state.refresh_player_team(ctx);
                state.process_events(ctx);
                state.process_index_batch(ctx);
                state.publish(self.surface);
            }
            return CommandResultV1::Handled;
        }
        if command.command != bridge_command(self.surface) {
            return CommandResultV1::Pass;
        }
        let routed = match serde_json::from_slice::<RoutedBridgeRequest>(command.payload) {
            Ok(routed) => routed,
            Err(_) => return CommandResultV1::Handled,
        };
        let response = match self.state.lock() {
            Ok(mut state) => {
                state.refresh_player_team(ctx);
                let response = state.handle_bridge_request(ctx, routed.request, self.surface);
                state.publish(self.surface);
                response
            }
            Err(_) => bridge_error_response(
                &routed.request,
                "server_state_unavailable",
                "Career server state is unavailable",
            ),
        };
        let payload = hub()
            .published_responses
            .lock()
            .ok()
            .and_then(|mut published| {
                encode_routed_bridge_response(
                    routed.route_id,
                    response,
                    MAX_INLINE_BRIDGE_RESPONSE_BYTES,
                    &mut published,
                )
                .ok()
            });
        match payload {
            Some(payload)
                if ctx.emit_event(
                    command.reply_target(),
                    bridge_response_event(self.surface),
                    &payload,
                ) => {}
            Some(_) => fail_routed_response(
                routed.route_id,
                "response_event_unavailable",
                "Career server could not publish the response completion event",
            ),
            None => fail_routed_response(
                routed.route_id,
                "response_serialization_failed",
                "Career server response could not be serialized",
            ),
        }
        CommandResultV1::Handled
    }
}

fn encode_routed_bridge_response(
    route_id: u64,
    response: BridgeResponse,
    inline_limit: usize,
    published: &mut BTreeMap<u64, BridgeResponse>,
) -> Result<Vec<u8>, serde_json::Error> {
    let routed = RoutedBridgeResponse { route_id, response };
    let payload = serde_json::to_vec(&routed)?;
    if payload.len() <= inline_limit {
        return Ok(payload);
    }
    published.insert(route_id, routed.response);
    serde_json::to_vec(&RoutedBridgeResponse {
        route_id,
        response: BridgeResponse::Published,
    })
}

pub(crate) struct RuntimeState {
    public: RuntimePublicState,
    index: MatchIndex,
    pending: VecDeque<(MatchKind, usize)>,
    locks: BTreeMap<(usize, LockGroup), LockSet>,
    last_event_seq: u64,
    schema_ready: bool,
    processed_since_checkpoint: usize,
    cache_chunk_count: usize,
    draft_settings: DraftScoreWeights,
    mock_draft: MockDraftSession,
}

impl Default for RuntimeState {
    fn default() -> Self {
        Self {
            public: RuntimePublicState::default(),
            index: MatchIndex::new(),
            pending: VecDeque::new(),
            locks: BTreeMap::new(),
            last_event_seq: 0,
            schema_ready: false,
            processed_since_checkpoint: 0,
            cache_chunk_count: 0,
            draft_settings: DraftScoreWeights::default(),
            mock_draft: MockDraftSession::default(),
        }
    }
}

impl RuntimeState {
    pub(crate) fn player_team_id(&self, ctx: &StableServerCtx<'_>) -> Option<usize> {
        self.public.player_team_id.or_else(|| ctx.player_team_id(0))
    }

    pub(crate) fn tier_profile_enabled(&self) -> bool {
        self.schema_ready
            && self.public.engine_status == EngineStatus::Ready
            && self.public.active_profile.enabled
    }

    fn reset_for_career(&mut self, ctx: &mut StableServerCtx<'_>) {
        *self = Self::default();
        self.public.connected = true;
        self.public.engine_status = EngineStatus::ValidatingSchema;
        if let Some(saved) = ctx.save_get_string(PROFILE_SAVE_KEY) {
            match deserialize_saved_profile(&saved) {
                Ok(profile) => {
                    self.public.active_profile = profile.clone();
                    self.public.preview_profile = profile;
                    self.public.diagnostics.push(Diagnostic::new(
                        "saved_profile_restored",
                        "The last applied tier profile was restored",
                        false,
                    ));
                }
                Err(message) => self.public.diagnostics.push(Diagnostic::new(
                    "saved_profile_invalid",
                    format!("The saved tier profile was ignored: {message}"),
                    false,
                )),
            }
        }
        self.refresh_player_team(ctx);
        self.schema_ready = self.validate_schema(ctx);
        if !self.schema_ready {
            self.public.engine_status = EngineStatus::Error;
            return;
        }
        self.restore_index_cache(ctx);
        self.queue_unindexed(ctx);
        self.public.engine_status = EngineStatus::Indexing;
        if self.pending.is_empty() {
            self.finish_indexing();
        }
    }

    fn reset_for_editor(&mut self, ctx: &mut StableServerCtx<'_>) {
        *self = Self::default();
        editor::reset_runtime_overrides();
        self.public.connected = true;
        self.public.engine_status = EngineStatus::ValidatingSchema;
        self.refresh_player_team(ctx);
        self.schema_ready = self.validate_schema(ctx);
        self.public.engine_status = if self.schema_ready {
            EngineStatus::Ready
        } else {
            EngineStatus::Error
        };
    }

    fn refresh_player_team(&mut self, ctx: &StableServerCtx<'_>) {
        let from_client = hub().player_team_id.load(Ordering::Acquire);
        self.public.player_team_id = if from_client != NONE_ID {
            Some(from_client)
        } else {
            ctx.player_team_id(0)
        };
    }

    fn validate_schema(&mut self, ctx: &StableServerCtx<'_>) -> bool {
        let mut failures = Vec::new();
        validate_first_record(
            ctx,
            RecordKindV1::Athlete,
            "Athlete",
            &["id", "stat", "champion_proficiency"],
            &mut failures,
        );
        validate_first_record(
            ctx,
            RecordKindV1::Staff,
            "Staff",
            &["id", "stat"],
            &mut failures,
        );
        validate_first_record(
            ctx,
            RecordKindV1::Team,
            "Team",
            &["id", "champion_tiers"],
            &mut failures,
        );
        let records = [
            (RecordKindV1::Athlete, "Athlete"),
            (RecordKindV1::Staff, "Staff"),
            (RecordKindV1::Team, "Team"),
        ]
        .map(|(kind, label)| {
            ctx.record_ids(kind)
                .first()
                .and_then(|id| ctx.record_get_json(kind, *id, ""))
                .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
                .ok_or(label)
        });
        if let [Ok(athlete), Ok(staff), Ok(team)] = &records {
            if let Err(paths) = validate_community_shapes(athlete, staff, team) {
                failures.extend(paths.into_iter().map(|path| {
                    Diagnostic::new(
                        "community_schema_invalid",
                        format!("Required 0.5.5 Community field is missing or invalid: {path}"),
                        true,
                    )
                }));
            }
        }
        if failures.is_empty() {
            self.public.diagnostics.push(Diagnostic::new(
                "schema_0_5_5_ready",
                "Required Team/Athlete/Staff record paths are available",
                false,
            ));
            true
        } else {
            self.public.diagnostics.extend(failures);
            false
        }
    }

    fn queue_unindexed(&mut self, ctx: &StableServerCtx<'_>) {
        for id in ctx.record_ids(RecordKindV1::MatchReplay) {
            if !self.index.contains(MatchKind::Tournament, id)
                && !self.pending.contains(&(MatchKind::Tournament, id))
            {
                self.pending.push_back((MatchKind::Tournament, id));
            }
        }
        for id in ctx.record_ids(RecordKindV1::SoloRankMatch) {
            if !self.index.contains(MatchKind::Solo, id)
                && !self.pending.contains(&(MatchKind::Solo, id))
            {
                self.pending.push_back((MatchKind::Solo, id));
            }
        }
        self.public.pending_records = self.pending.len();
        self.public.index_progress.total = self.index.match_count() + self.pending.len();
        self.public.index_progress.pending = self.pending.len();
        self.public.index_progress.indexed = self.index.match_count();
    }

    fn process_events(&mut self, ctx: &StableServerCtx<'_>) {
        let events = ctx.management_events_after(self.last_event_seq);
        let mut refresh = false;
        let mut season_rollover = false;
        for event in events {
            self.last_event_seq = self.last_event_seq.max(event.seq);
            if matches!(
                event.kind,
                Some(ManagementEventKindV1::MatchFinished | ManagementEventKindV1::SeasonRollover)
            ) {
                refresh = true;
            }
            season_rollover |= event.kind == Some(ManagementEventKindV1::SeasonRollover);
        }
        if season_rollover {
            hub().catalog_epoch.fetch_add(1, Ordering::AcqRel);
            if let Ok(mut catalog) = hub().champion_catalog.write() {
                catalog.revision = catalog.revision.wrapping_add(1);
                catalog.status = CatalogStatus::Loading;
                catalog.champions.clear();
            }
            if let Ok(mut champions) = hub().champion_info.write() {
                champions.clear();
            }
        }
        if refresh {
            self.queue_unindexed(ctx);
            if !self.pending.is_empty() {
                self.public.engine_status = EngineStatus::Indexing;
            }
        }
    }

    fn process_index_batch(&mut self, ctx: &mut StableServerCtx<'_>) {
        if !self.schema_ready {
            return;
        }
        let mut changed = false;
        for _ in 0..INDEX_BATCH_SIZE {
            let Some((kind, id)) = self.pending.pop_front() else {
                break;
            };
            self.public.index_progress.processed =
                self.public.index_progress.processed.saturating_add(1);
            self.processed_since_checkpoint = self.processed_since_checkpoint.saturating_add(1);
            let record_kind = match kind {
                MatchKind::Tournament => RecordKindV1::MatchReplay,
                MatchKind::Solo => RecordKindV1::SoloRankMatch,
            };
            let Some(json) = ctx.record_get_json(record_kind, id, "") else {
                self.public.diagnostics.push(Diagnostic::new(
                    "record_read_failed",
                    format!("Could not read {kind:?} record {id}"),
                    false,
                ));
                self.public.index_progress.failed =
                    self.public.index_progress.failed.saturating_add(1);
                continue;
            };
            let result = match kind {
                MatchKind::Tournament => {
                    tournament_region_division(ctx, &json).and_then(|(region, division)| {
                        self.index.record_tournament(&json, region, division)
                    })
                }
                MatchKind::Solo => self.index.record_solo(&json),
            };
            match result {
                Ok(()) => changed = true,
                Err(error) => {
                    self.public.index_progress.failed =
                        self.public.index_progress.failed.saturating_add(1);
                    self.public.diagnostics.push(Diagnostic::new(
                        "replay_parse_failed",
                        format!("{kind:?} record {id}: {error}"),
                        false,
                    ));
                }
            }
        }
        self.public.pending_records = self.pending.len();
        self.public.indexed_matches = self.index.match_count();
        self.public.index_progress.pending = self.pending.len();
        self.public.index_progress.indexed = self.index.match_count();
        if self.processed_since_checkpoint >= crate::MATCH_INDEX_CACHE_CHUNK_SIZE
            && !self.pending.is_empty()
        {
            self.persist_index_cache(ctx, IndexCacheState::Checkpointed);
        }
        if self.pending.is_empty() {
            self.persist_index_cache(ctx, IndexCacheState::Complete);
            self.finish_indexing();
        } else if changed {
            self.refresh_analytics();
        }
    }

    fn finish_indexing(&mut self) {
        self.public.engine_status = EngineStatus::Ready;
        self.refresh_analytics();
    }

    fn restore_index_cache(&mut self, ctx: &StableServerCtx<'_>) {
        let Some(raw_manifest) = ctx.save_get_string(INDEX_CACHE_MANIFEST_KEY) else {
            self.public.index_progress.cache_state = IndexCacheState::Cold;
            return;
        };
        let result = (|| -> Result<MatchIndexCache, String> {
            let manifest: MatchIndexCacheManifest = serde_json::from_str(&raw_manifest)
                .map_err(|error| format!("manifest JSON: {error}"))?;
            let chunks = (0..manifest.chunk_count)
                .map(|index| {
                    ctx.save_get_bytes(&format!("{INDEX_CACHE_CHUNK_PREFIX}{index}"))
                        .ok_or_else(|| format!("chunk {index} is missing"))
                })
                .collect::<Result<Vec<_>, _>>()?;
            Ok(MatchIndexCache { manifest, chunks })
        })()
        .and_then(|cache| {
            let current_tournament = ctx
                .record_ids(RecordKindV1::MatchReplay)
                .into_iter()
                .collect::<std::collections::BTreeSet<_>>();
            let current_solo = ctx
                .record_ids(RecordKindV1::SoloRankMatch)
                .into_iter()
                .collect::<std::collections::BTreeSet<_>>();
            if cache
                .manifest
                .match_ids
                .iter()
                .any(|entry| match entry.kind {
                    MatchKind::Tournament => !current_tournament.contains(&entry.id),
                    MatchKind::Solo => !current_solo.contains(&entry.id),
                })
            {
                return Err("cached match ids were deleted from the career".to_owned());
            }
            cache
                .restore(&self.public.game_version)
                .map(|index| (cache, index))
                .map_err(|error| error.to_string())
        });
        match result {
            Ok((cache, index)) => {
                self.cache_chunk_count = cache.manifest.chunk_count;
                self.index = index;
                self.public.index_progress.processed = self.index.match_count();
                self.public.index_progress.indexed = self.index.match_count();
                self.public.index_progress.cache_state = IndexCacheState::Restored;
                self.public.diagnostics.push(Diagnostic::new(
                    "index_cache_restored",
                    format!(
                        "Restored {} indexed matches from career cache",
                        self.index.match_count()
                    ),
                    false,
                ));
            }
            Err(message) => {
                self.index = MatchIndex::new();
                self.public.index_progress.cache_state = IndexCacheState::Invalid;
                self.public.diagnostics.push(Diagnostic::new(
                    "index_cache_invalid",
                    format!("The career index cache will be rebuilt: {message}"),
                    false,
                ));
            }
        }
    }

    fn persist_index_cache(&mut self, ctx: &mut StableServerCtx<'_>, state: IndexCacheState) {
        let cache = match MatchIndexCache::from_index(&self.index, &self.public.game_version) {
            Ok(cache) => cache,
            Err(error) => {
                self.public.diagnostics.push(Diagnostic::new(
                    "index_cache_encode_failed",
                    error.to_string(),
                    false,
                ));
                return;
            }
        };
        for (index, chunk) in cache.chunks.iter().enumerate() {
            if !ctx.save_set_bytes(&format!("{INDEX_CACHE_CHUNK_PREFIX}{index}"), chunk) {
                self.public.diagnostics.push(Diagnostic::new(
                    "index_cache_chunk_write_failed",
                    format!("Could not save index cache chunk {index}"),
                    false,
                ));
                return;
            }
        }
        let old_chunk_count = self.cache_chunk_count;
        let manifest = match serde_json::to_string(&cache.manifest) {
            Ok(manifest) => manifest,
            Err(error) => {
                self.public.diagnostics.push(Diagnostic::new(
                    "index_cache_manifest_encode_failed",
                    error.to_string(),
                    false,
                ));
                return;
            }
        };
        if !ctx.save_set_string(INDEX_CACHE_MANIFEST_KEY, &manifest) {
            self.public.diagnostics.push(Diagnostic::new(
                "index_cache_manifest_write_failed",
                "Could not commit the index cache manifest",
                false,
            ));
            return;
        }
        for index in cache.manifest.chunk_count..old_chunk_count {
            let _ = ctx.save_remove_key(&format!("{INDEX_CACHE_CHUNK_PREFIX}{index}"));
        }
        self.cache_chunk_count = cache.manifest.chunk_count;
        self.processed_since_checkpoint = 0;
        self.public.index_progress.cache_state = state;
    }

    fn refresh_analytics(&mut self) {
        match self.index.preview(&self.public.active_profile) {
            Ok(analytics) => {
                self.public.data_revision = analytics.data_revision;
                self.public.analytics = Some(analytics);
            }
            Err(error) => {
                self.public.engine_status = EngineStatus::Error;
                self.public.diagnostics.push(Diagnostic::new(
                    "active_profile_failed",
                    error.to_string(),
                    true,
                ));
            }
        }
    }

    fn drain_requests(&mut self, ctx: &mut StableServerCtx<'_>, surface: ApiSurface) {
        let Ok(receiver) = hub().request_rx.lock() else {
            return;
        };
        while let Ok(envelope) = receiver.try_recv() {
            if !try_begin_request(&envelope.execution) {
                continue;
            }
            let response = self.handle_bridge_request(ctx, envelope.request, surface);
            envelope.execution.store(REQUEST_DONE, Ordering::Release);
            let _ = envelope.response.send(response);
        }
    }

    fn handle_bridge_request(
        &mut self,
        ctx: &mut StableServerCtx<'_>,
        request: BridgeRequest,
        surface: ApiSurface,
    ) -> BridgeResponse {
        match request {
            BridgeRequest::Api(request) => {
                if surface.allows(request.command) {
                    BridgeResponse::Api(self.handle_api(ctx, request))
                } else {
                    BridgeResponse::Api(ApiResponse::error(
                        request.request_id,
                        "command_not_allowed",
                        "The command is not available on this Atlas service",
                    ))
                }
            }
            BridgeRequest::TierSync if surface == ApiSurface::Core => BridgeResponse::TierSync(
                editor::get_tier_sync(ctx, self).unwrap_or_else(|message| format!("ERR|{message}")),
            ),
            BridgeRequest::TierSync => {
                BridgeResponse::TierSync("ERR|COMMAND_NOT_ALLOWED".to_owned())
            }
        }
    }

    fn handle_api(&mut self, ctx: &mut StableServerCtx<'_>, request: ApiRequest) -> ApiResponse {
        if let Err(error) = request.validate() {
            return ApiResponse::error(request.request_id, error.code, error.message);
        }
        if !self.schema_ready && api_command_mutates(request.command) {
            return ApiResponse::error(
                request.request_id,
                "schema_preflight_failed",
                "Writes are disabled because the 0.5.5 Community schema preflight failed",
            );
        }
        match request.command {
            ApiCommand::SubscribeState => ApiResponse::error(
                request.request_id,
                "subscription_requires_direct_connection",
                "STATE_CHANGED subscriptions must stay on their original connection",
            ),
            ApiCommand::GetDashboard => self.api_ok(request.request_id, &self.public),
            ApiCommand::GetLocks => self.api_ok(request.request_id, &self.lock_values()),
            ApiCommand::GetDiagnostics => self.api_ok(request.request_id, &self.public.diagnostics),
            ApiCommand::GetChampionCatalog => {
                self.api_ok(request.request_id, &registered_champion_catalog_state())
            }
            ApiCommand::GetEditorData => match editor::editor_data(ctx, self, &request.payload) {
                Ok(data) => ApiResponse::ok(request.request_id, data),
                Err(message) => {
                    ApiResponse::error(request.request_id, "editor_data_failed", message)
                }
            },
            ApiCommand::ApplyEditorSettings => {
                match editor::apply_editor_settings(ctx, self, &request.payload) {
                    Ok(data) => ApiResponse::ok(request.request_id, data),
                    Err(message) => {
                        ApiResponse::error(request.request_id, "editor_settings_failed", message)
                    }
                }
            }
            ApiCommand::GetPlayerMastery => {
                #[derive(Deserialize)]
                struct PlayerPayload {
                    athlete_id: usize,
                }
                let payload: PlayerPayload = match serde_json::from_value(request.payload) {
                    Ok(payload) => payload,
                    Err(error) => {
                        return ApiResponse::error(
                            request.request_id,
                            "invalid_mastery_schema",
                            error.to_string(),
                        )
                    }
                };
                match player_mastery_state(ctx, payload.athlete_id) {
                    Ok(state) => self.api_ok(request.request_id, &state),
                    Err(message) => {
                        ApiResponse::error(request.request_id, "mastery_read_failed", message)
                    }
                }
            }
            ApiCommand::SetPlayerMastery => {
                #[derive(Deserialize)]
                struct MasteryValuePayload {
                    champion_id: String,
                    value: i64,
                }
                #[derive(Deserialize)]
                struct MasteryPayload {
                    athlete_id: usize,
                    values: Vec<MasteryValuePayload>,
                }
                let payload: MasteryPayload = match serde_json::from_value(request.payload) {
                    Ok(payload) => payload,
                    Err(error) => {
                        return ApiResponse::error(
                            request.request_id,
                            "invalid_mastery_schema",
                            error.to_string(),
                        )
                    }
                };
                if payload.values.is_empty()
                    || payload.values.len() > 512
                    || payload
                        .values
                        .iter()
                        .any(|value| !(0..=100).contains(&value.value))
                {
                    return ApiResponse::error(
                        request.request_id,
                        "invalid_mastery_values",
                        "Mastery values must contain 1-512 entries in the 0-100 range",
                    );
                }
                let mut unique = std::collections::BTreeSet::new();
                if payload.values.iter().any(|value| {
                    value.champion_id.trim().is_empty() || !unique.insert(value.champion_id.clone())
                }) {
                    return ApiResponse::error(
                        request.request_id,
                        "invalid_mastery_values",
                        "Champion ids must be non-empty and unique",
                    );
                }
                let lock = LockSet::new_mastery(
                    payload.athlete_id,
                    payload.athlete_id.to_string(),
                    payload
                        .values
                        .into_iter()
                        .map(|value| (value.champion_id, value.value as f64))
                        .collect(),
                );
                match apply_lock_once(ctx, &lock)
                    .and_then(|_| player_mastery_state(ctx, payload.athlete_id))
                {
                    Ok(state) => self.api_ok(request.request_id, &state),
                    Err(message) => {
                        ApiResponse::error(request.request_id, "mastery_write_failed", message)
                    }
                }
            }
            ApiCommand::ApplyPlayerEdit => {
                let payload: PlayerEditRequest = match serde_json::from_value(request.payload) {
                    Ok(payload) => payload,
                    Err(error) => {
                        return ApiResponse::error(
                            request.request_id,
                            "invalid_player_edit_schema",
                            error.to_string(),
                        )
                    }
                };
                match editor::apply_player_edit(ctx, &payload).map(|(record, changed_paths)| {
                    PlayerEditReadback {
                        target_id: payload.athlete_id,
                        changed_paths,
                        record,
                        client_screen_verified: false,
                    }
                }) {
                    Ok(readback) => self.api_ok(request.request_id, &readback),
                    Err(message) => {
                        ApiResponse::error(request.request_id, "player_edit_failed", message)
                    }
                }
            }
            ApiCommand::ApplyStaffEdit => {
                let payload: StaffEditRequest = match serde_json::from_value(request.payload) {
                    Ok(payload) => payload,
                    Err(error) => {
                        return ApiResponse::error(
                            request.request_id,
                            "invalid_staff_edit_schema",
                            error.to_string(),
                        )
                    }
                };
                match editor::apply_staff_edit(ctx, &payload).map(|(record, changed_paths)| {
                    PlayerEditReadback {
                        target_id: payload.staff_id,
                        changed_paths,
                        record,
                        client_screen_verified: false,
                    }
                }) {
                    Ok(readback) => self.api_ok(request.request_id, &readback),
                    Err(message) => {
                        ApiResponse::error(request.request_id, "staff_edit_failed", message)
                    }
                }
            }
            ApiCommand::MovePlayer => {
                #[derive(Deserialize)]
                struct MovePayload {
                    athlete_id: usize,
                    team_id: usize,
                }
                let payload: MovePayload = match serde_json::from_value(request.payload) {
                    Ok(payload) => payload,
                    Err(error) => {
                        return ApiResponse::error(
                            request.request_id,
                            "invalid_move_player_schema",
                            error.to_string(),
                        )
                    }
                };
                let result = editor::move_player(ctx, payload.athlete_id, payload.team_id)
                    .and_then(|_| {
                        let raw = ctx
                            .record_get_json(RecordKindV1::Athlete, payload.athlete_id, "")
                            .ok_or_else(|| "TRANSFER_READBACK_MISSING".to_owned())?;
                        let record = serde_json::from_str(&raw)
                            .map_err(|error| format!("TRANSFER_READBACK_INVALID:{error}"))?;
                        Ok(MovePlayerReadback {
                            athlete_id: payload.athlete_id,
                            team_id: payload.team_id,
                            record,
                            client_screen_verified: false,
                            status: "server_applied_client_sync_pending".to_owned(),
                        })
                    });
                match result {
                    Ok(readback) => self.api_ok(request.request_id, &readback),
                    Err(message) => {
                        ApiResponse::error(request.request_id, "move_player_failed", message)
                    }
                }
            }
            ApiCommand::ExportTierTsv => match self.current_tier_rows(ctx) {
                Ok(rows) => self.api_ok(
                    request.request_id,
                    &json!({
                        "version": 2,
                        "row_count": rows.len(),
                        "tsv": render_tier_tsv_v2(&rows),
                    }),
                ),
                Err(message) => {
                    ApiResponse::error(request.request_id, "tier_tsv_export_failed", message)
                }
            },
            ApiCommand::ValidateTierTsv => {
                #[derive(Deserialize)]
                struct TsvPayload {
                    tsv: String,
                }
                let payload: TsvPayload = match serde_json::from_value(request.payload) {
                    Ok(payload) => payload,
                    Err(error) => {
                        return ApiResponse::error(
                            request.request_id,
                            "invalid_tsv_schema",
                            error.to_string(),
                        )
                    }
                };
                if payload.tsv.len() > 4 * 1024 * 1024 {
                    return ApiResponse::error(
                        request.request_id,
                        "tier_tsv_too_large",
                        "Tier TSV input exceeds 4 MiB",
                    );
                }
                let version = if payload
                    .tsv
                    .lines()
                    .next()
                    .is_some_and(|header| header.trim_end_matches('\r').ends_with("\teligible"))
                {
                    2
                } else {
                    1
                };
                match parse_tier_tsv(&payload.tsv) {
                    Ok(rows) if rows.len() <= 10_000 => self.api_ok(
                        request.request_id,
                        &json!({"version": version, "row_count": rows.len(), "rows": rows}),
                    ),
                    Ok(_) => ApiResponse::error(
                        request.request_id,
                        "tier_tsv_too_many_rows",
                        "Tier TSV input exceeds 10,000 rows",
                    ),
                    Err(error) => ApiResponse::error(
                        request.request_id,
                        "invalid_tier_tsv",
                        error.to_string(),
                    ),
                }
            }
            ApiCommand::PreviewTierProfile => {
                let profile: TierProfile = match serde_json::from_value(request.payload) {
                    Ok(profile) => profile,
                    Err(error) => {
                        return ApiResponse::error(
                            request.request_id,
                            "invalid_profile_schema",
                            error.to_string(),
                        )
                    }
                };
                match profile.validate().and_then(|_| {
                    self.index
                        .preview(&profile)
                        .map_err(|_| tfm2_atlas_engine::ValidationError::InvalidPatch)
                }) {
                    Ok(analytics) => {
                        self.public.preview_profile = profile;
                        self.api_ok(request.request_id, &analytics)
                    }
                    Err(error) => {
                        ApiResponse::error(request.request_id, error.code(), error.to_string())
                    }
                }
            }
            ApiCommand::ApplyTierProfile => {
                let profile: TierProfile = match serde_json::from_value(request.payload) {
                    Ok(profile) => profile,
                    Err(error) => {
                        return ApiResponse::error(
                            request.request_id,
                            "invalid_profile_schema",
                            error.to_string(),
                        )
                    }
                };
                if let Err(error) = profile.validate() {
                    return ApiResponse::error(request.request_id, error.code(), error.to_string());
                }
                match self.index.preview(&profile) {
                    Ok(analytics) => {
                        let saved = match serialize_saved_profile(&profile) {
                            Ok(saved) => saved,
                            Err(message) => {
                                return ApiResponse::error(
                                    request.request_id,
                                    "profile_serialization_failed",
                                    message,
                                )
                            }
                        };
                        let applied_champions = if profile.enabled {
                            match self.write_tiers_for_profile(ctx, &analytics) {
                                Ok(count) => count,
                                Err(message) => {
                                    self.public.tier_application = TierApplicationState {
                                        applied: false,
                                        team_id: self.public.player_team_id,
                                        profile_revision: self
                                            .public
                                            .tier_application
                                            .profile_revision,
                                        applied_champions: 0,
                                        readback_verified: false,
                                        client_screen_verified: false,
                                        error: Some(message.clone()),
                                    };
                                    return ApiResponse::error(
                                        request.request_id,
                                        "tier_apply_failed",
                                        message,
                                    );
                                }
                            }
                        } else {
                            0
                        };
                        if !ctx.save_set_string(PROFILE_SAVE_KEY, &saved) {
                            self.public.tier_application.error = Some(
                                "Tier readback succeeded, but the profile could not be persisted"
                                    .to_owned(),
                            );
                            return ApiResponse::error(
                                request.request_id,
                                "profile_persistence_failed",
                                "The active profile was not changed because its setting could not be saved",
                            );
                        }
                        self.public.active_profile = profile.clone();
                        self.public.preview_profile = profile;
                        self.public.analytics = Some(analytics);
                        self.public.tier_application = TierApplicationState {
                            applied: self.public.active_profile.enabled,
                            team_id: self.public.player_team_id,
                            profile_revision: self
                                .public
                                .tier_application
                                .profile_revision
                                .wrapping_add(1),
                            applied_champions,
                            readback_verified: self.public.active_profile.enabled,
                            client_screen_verified: !self.public.active_profile.enabled,
                            error: None,
                        };
                        self.api_ok(request.request_id, &self.public)
                    }
                    Err(error) => ApiResponse::error(
                        request.request_id,
                        "profile_preview_failed",
                        error.to_string(),
                    ),
                }
            }
            ApiCommand::GetMockDraft => match self.mock_draft_state(ctx) {
                Ok(state) => self.api_ok(request.request_id, &state),
                Err(message) => {
                    ApiResponse::error(request.request_id, "mock_draft_state_failed", message)
                }
            },
            ApiCommand::SetMockDraftContext => {
                let update: MockDraftContextUpdate = match serde_json::from_value(request.payload) {
                    Ok(update) => update,
                    Err(error) => {
                        return ApiResponse::error(
                            request.request_id,
                            "invalid_mock_draft_context",
                            error.to_string(),
                        )
                    }
                };
                if let Some(team_id) = update.opponent_team_id {
                    let valid = draft_team_options(ctx, self.public.player_team_id)
                        .into_iter()
                        .any(|team| team.team_id == team_id && !team.player_team);
                    if !valid {
                        return ApiResponse::error(
                            request.request_id,
                            "invalid_draft_opponent",
                            "The selected opponent is not available in this career",
                        );
                    }
                }
                if let Err(error) = self.mock_draft.set_context(update) {
                    return ApiResponse::error(request.request_id, error.code(), error.to_string());
                }
                match self.mock_draft_state(ctx) {
                    Ok(state) => self.api_ok(request.request_id, &state),
                    Err(message) => {
                        ApiResponse::error(request.request_id, "mock_draft_state_failed", message)
                    }
                }
            }
            ApiCommand::ApplyMockDraftAction => {
                let action: MockDraftAction = match serde_json::from_value(request.payload) {
                    Ok(action) => action,
                    Err(error) => {
                        return ApiResponse::error(
                            request.request_id,
                            "invalid_mock_draft_action",
                            error.to_string(),
                        )
                    }
                };
                let active = registered_champion_catalog_state()
                    .champions
                    .into_iter()
                    .map(|champion| champion.champion_id)
                    .collect::<Vec<_>>();
                if let Err(error) = self.mock_draft.apply(action, &active) {
                    return ApiResponse::error(request.request_id, error.code(), error.to_string());
                }
                match self.mock_draft_state(ctx) {
                    Ok(state) => self.api_ok(request.request_id, &state),
                    Err(message) => {
                        ApiResponse::error(request.request_id, "mock_draft_state_failed", message)
                    }
                }
            }
            ApiCommand::RemoveMockDraftAction => {
                let action: MockDraftAction = match serde_json::from_value(request.payload) {
                    Ok(action) => action,
                    Err(error) => {
                        return ApiResponse::error(
                            request.request_id,
                            "invalid_mock_draft_action",
                            error.to_string(),
                        )
                    }
                };
                if let Err(error) = self.mock_draft.remove(&action) {
                    return ApiResponse::error(request.request_id, error.code(), error.to_string());
                }
                match self.mock_draft_state(ctx) {
                    Ok(state) => self.api_ok(request.request_id, &state),
                    Err(message) => {
                        ApiResponse::error(request.request_id, "mock_draft_state_failed", message)
                    }
                }
            }
            ApiCommand::UndoMockDraft => {
                self.mock_draft.undo();
                match self.mock_draft_state(ctx) {
                    Ok(state) => self.api_ok(request.request_id, &state),
                    Err(message) => {
                        ApiResponse::error(request.request_id, "mock_draft_state_failed", message)
                    }
                }
            }
            ApiCommand::ResetMockDraftSet => {
                self.mock_draft.reset_current_set();
                match self.mock_draft_state(ctx) {
                    Ok(state) => self.api_ok(request.request_id, &state),
                    Err(message) => {
                        ApiResponse::error(request.request_id, "mock_draft_state_failed", message)
                    }
                }
            }
            ApiCommand::SetDraftSettings => {
                let settings: DraftScoreWeights = match serde_json::from_value(request.payload) {
                    Ok(settings) => settings,
                    Err(error) => {
                        return ApiResponse::error(
                            request.request_id,
                            "invalid_draft_settings",
                            error.to_string(),
                        )
                    }
                };
                if let Err(error) = settings.validate() {
                    return ApiResponse::error(
                        request.request_id,
                        "invalid_draft_settings",
                        error.to_string(),
                    );
                }
                self.draft_settings = settings;
                match self.mock_draft_state(ctx) {
                    Ok(state) => self.api_ok(request.request_id, &state),
                    Err(message) => {
                        ApiResponse::error(request.request_id, "mock_draft_state_failed", message)
                    }
                }
            }
            ApiCommand::SetLock => {
                let lock: LockSet = match serde_json::from_value(request.payload) {
                    Ok(lock) => lock,
                    Err(error) => {
                        return ApiResponse::error(
                            request.request_id,
                            "invalid_lock_schema",
                            error.to_string(),
                        )
                    }
                };
                match self.apply_lock(ctx, lock) {
                    Ok(lock) => self.api_ok(request.request_id, &lock),
                    Err((code, message)) => ApiResponse::error(request.request_id, code, message),
                }
            }
            ApiCommand::Unlock => {
                #[derive(Deserialize)]
                struct UnlockPayload {
                    target_id: usize,
                    group: LockGroup,
                }
                let payload: UnlockPayload = match serde_json::from_value(request.payload) {
                    Ok(payload) => payload,
                    Err(error) => {
                        return ApiResponse::error(
                            request.request_id,
                            "invalid_unlock_schema",
                            error.to_string(),
                        )
                    }
                };
                if self
                    .locks
                    .remove(&(payload.target_id, payload.group))
                    .is_some()
                {
                    self.api_ok(request.request_id, &self.lock_values())
                } else {
                    ApiResponse::error(
                        request.request_id,
                        "lock_not_found",
                        "The requested lock is not active",
                    )
                }
            }
        }
    }

    fn api_ok<T: Serialize>(&self, request_id: u64, value: &T) -> ApiResponse {
        match serde_json::to_value(value) {
            Ok(value) => ApiResponse::ok(request_id, value),
            Err(error) => ApiResponse::error(
                request_id,
                "response_serialization_failed",
                error.to_string(),
            ),
        }
    }

    fn mock_draft_state(&self, ctx: &StableServerCtx<'_>) -> Result<MockDraftState, String> {
        let mut state = self.mock_draft.state().clone();
        state.team_options = draft_team_options(ctx, self.public.player_team_id);
        let catalog = registered_champion_catalog_state()
            .champions
            .into_iter()
            .map(|champion| (champion.champion_id.clone(), champion))
            .collect::<BTreeMap<_, _>>();
        state.available_champions = catalog.keys().cloned().collect();
        state.settings = self.draft_settings.clone();
        state.excluded_blue = mock_carryover_excluded(&state, DraftSide::Blue)
            .into_iter()
            .collect();
        state.excluded_red = mock_carryover_excluded(&state, DraftSide::Red)
            .into_iter()
            .collect();
        let mut profile = self.public.preview_profile.clone();
        profile.role = RoleFilter::All;
        let analytics = self
            .index
            .preview(&profile)
            .map_err(|error| error.to_string())?;
        state.recommendation = Some(self.build_mock_draft_recommendation(&state, &analytics)?);
        state.evaluation = evaluate_mock_draft(&state, &analytics);
        let current = &state.sets[state.current_set - 1];
        state.blue_gate = Some(analyze_role_gate(&role_sets(&current.blue_picks, &catalog)));
        state.red_gate = Some(analyze_role_gate(&role_sets(&current.red_picks, &catalog)));
        Ok(state)
    }

    fn build_mock_draft_recommendation(
        &self,
        state: &MockDraftState,
        analytics: &AnalyticsState,
    ) -> Result<DraftRecommendation, String> {
        let rows = analytics
            .champions
            .iter()
            .map(|row| (row.champion_id.as_str(), row))
            .collect::<BTreeMap<_, _>>();
        let preferences = state
            .opponent_team_id
            .map(|team_id| {
                let mut profile = self.public.preview_profile.clone();
                profile.role = RoleFilter::All;
                self.index.team_pick_preferences(&profile, team_id)
            })
            .transpose()
            .map_err(|error| error.to_string())?
            .unwrap_or_default();
        let catalog = registered_champion_catalog_state()
            .champions
            .into_iter()
            .map(|champion| (champion.champion_id.clone(), champion))
            .collect::<BTreeMap<_, _>>();
        let current = &state.sets[state.current_set - 1];
        let (ally_picks, enemy_picks) = match state.current_our_side() {
            DraftSide::Blue => (&current.blue_picks, &current.red_picks),
            DraftSide::Red => (&current.red_picks, &current.blue_picks),
        };
        let used = current
            .blue_bans
            .iter()
            .chain(&current.red_bans)
            .chain(&current.blue_picks)
            .chain(&current.red_picks)
            .cloned()
            .collect::<BTreeSet<_>>();
        let carryover = match state.current_our_side() {
            DraftSide::Blue => state.excluded_blue.iter(),
            DraftSide::Red => state.excluded_red.iter(),
        }
        .cloned()
        .collect::<BTreeSet<_>>();
        let available = state
            .available_champions
            .iter()
            .filter(|champion_id| !used.contains(*champion_id) && !carryover.contains(*champion_id))
            .cloned()
            .collect::<Vec<_>>();
        let ally_role_sets = role_sets(ally_picks, &catalog);
        let enemy_role_sets = role_sets(enemy_picks, &catalog);
        let ally_gate = analyze_role_gate(&ally_role_sets);
        let enemy_gate = analyze_role_gate(&enemy_role_sets);
        let ally_remaining_roles = remaining_roles_from_gate(&ally_gate);
        let enemy_remaining_roles = remaining_roles_from_gate(&enemy_gate);
        let projected_ally =
            project_lineup(analytics, &BTreeMap::new(), ally_picks, &used, &catalog);
        let projected_enemy = project_lineup(analytics, &preferences, enemy_picks, &used, &catalog);
        let mut settings = self.draft_settings.clone();
        let pick_gate_enabled = settings.pick_role_gate;
        let ban_gate_enabled = settings.ban_role_gate;
        settings.pick_role_gate = false;
        settings.ban_role_gate = false;
        let mut scored = Vec::new();
        for champion_id in available {
            let row = rows.get(champion_id.as_str()).copied();
            let candidate_catalog = catalog.get(&champion_id);
            let candidate_roles = game_roles(&champion_id, &catalog);
            let (gate_enabled, gate_state, current_role_sets) =
                if state.last_own_phase == DraftPhase::Pick {
                    (pick_gate_enabled, &ally_gate, &ally_role_sets)
                } else {
                    (ban_gate_enabled, &enemy_gate, &enemy_role_sets)
                };
            if gate_enabled
                && gate_state.feasible
                && !candidate_passes_role_gate(current_role_sets, &candidate_roles)
            {
                continue;
            }
            let role_confidence =
                if candidate_catalog.is_some_and(|entry| !entry.positions.is_empty()) {
                    DraftRoleConfidence::High
                } else {
                    DraftRoleConfidence::Low
                };
            let input = DraftCandidateInput {
                champion_id: champion_id.clone(),
                op_score: row.and_then(|row| row.overall).unwrap_or(50.0),
                matchup_score: row
                    .map(|row| relation_score(&row.matchups, enemy_picks, false))
                    .unwrap_or(50.0),
                synergy_score: row
                    .map(|row| relation_score(&row.synergies, ally_picks, false))
                    .unwrap_or(50.0),
                composition_score: composition_completion(ally_picks, candidate_catalog, &catalog),
                opponent_preference_score: preferences
                    .get(&champion_id)
                    .map(|row| row.score)
                    .unwrap_or(50.0),
                ally_threat_score: row
                    .map(|row| relation_score(&row.matchups, &projected_ally, false))
                    .unwrap_or(50.0),
                projected_enemy_synergy_score: row
                    .map(|row| relation_score(&row.synergies, &projected_enemy, false))
                    .unwrap_or(50.0),
                projected_enemy_composition_score: composition_completion(
                    &projected_enemy,
                    candidate_catalog,
                    &catalog,
                ),
                usable_roles: candidate_roles,
                role_confidence,
            };
            let remaining = if state.last_own_phase == DraftPhase::Pick {
                &ally_remaining_roles
            } else {
                &enemy_remaining_roles
            };
            if let Some(score) = settings.score_candidate(state.last_own_phase, &input, remaining) {
                scored.push(score);
            }
        }
        scored.sort_by(|left, right| {
            right
                .total
                .total_cmp(&left.total)
                .then(left.champion_id.cmp(&right.champion_id))
        });
        scored.truncate(5);
        let mut notes = Vec::new();
        if preferences.is_empty() {
            notes.push("opponent_history_unavailable".to_owned());
        }
        if (!ally_gate.feasible && !ally_picks.is_empty())
            || (!enemy_gate.feasible && !enemy_picks.is_empty())
        {
            notes.push("role_assignment_conflict".to_owned());
        }
        if scored.iter().any(|candidate| candidate.low_confidence) {
            notes.push("low_role_confidence_present".to_owned());
        }
        Ok(DraftRecommendation {
            phase: state.last_own_phase,
            candidates: scored,
            projected_ally: projected_ally.clone(),
            projected_enemy: projected_enemy.clone(),
            ally_remaining_roles,
            enemy_remaining_roles,
            ally_composition: composition_state(&projected_ally, &catalog),
            enemy_composition: composition_state(&projected_enemy, &catalog),
            notes,
        })
    }

    fn apply_lock(
        &mut self,
        ctx: &mut StableServerCtx<'_>,
        mut lock: LockSet,
    ) -> Result<LockSet, (&'static str, String)> {
        if let Err(error) = lock.validate() {
            return Err((error.code(), error.to_string()));
        }
        apply_lock_once(ctx, &lock).map_err(|message| ("lock_apply_failed", message))?;
        lock.status = LockStatus::Active;
        lock.error = None;
        self.locks
            .insert((lock.target_id, lock.group), lock.clone());
        Ok(lock)
    }

    fn enforce_locks(&mut self, ctx: &mut StableServerCtx<'_>) {
        for lock in self.locks.values_mut() {
            match apply_lock_once(ctx, lock) {
                Ok(()) => {
                    lock.status = LockStatus::Active;
                    lock.error = None;
                }
                Err(error) => {
                    lock.status = LockStatus::Error;
                    lock.error = Some(error);
                }
            }
        }
    }

    fn enforce_tiers_on_tick(&mut self, ctx: &mut StableServerCtx<'_>) {
        if !self.public.active_profile.enabled || self.public.engine_status != EngineStatus::Ready {
            return;
        }
        let Some(analytics) = self.public.analytics.clone() else {
            return;
        };
        match self.write_tiers_for_profile(ctx, &analytics) {
            Ok(count) => {
                self.public.tier_application.applied = true;
                self.public.tier_application.team_id = self.public.player_team_id;
                self.public.tier_application.applied_champions = count;
                self.public.tier_application.readback_verified = true;
                self.public.tier_application.error = None;
            }
            Err(message) => {
                self.public.tier_application.applied = false;
                self.public.tier_application.readback_verified = false;
                self.public.tier_application.error = Some(message.clone());
                if !self
                    .public
                    .diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic.code == "tier_enforcement_failed")
                {
                    self.public.diagnostics.push(Diagnostic::new(
                        "tier_enforcement_failed",
                        message,
                        false,
                    ));
                }
            }
        }
    }

    fn write_tiers_for_profile(
        &self,
        ctx: &mut StableServerCtx<'_>,
        analytics: &AnalyticsState,
    ) -> Result<usize, String> {
        let team_id = self
            .public
            .player_team_id
            .ok_or_else(|| "Player team is not connected".to_owned())?;
        let team_json = ctx
            .record_get_json(RecordKindV1::Team, team_id, "")
            .ok_or_else(|| "Player team record could not be read".to_owned())?;
        let mut team: Value =
            serde_json::from_str(&team_json).map_err(|error| error.to_string())?;
        let existing = team
            .get("champion_tiers")
            .and_then(Value::as_object)
            .ok_or_else(|| "Player team champion_tiers is not an object".to_owned())?;
        let registered = registered_champion_ids();
        let rows = build_tier_rows(existing.keys().cloned(), Some(analytics), registered);
        apply_tiers_to_team(&mut team, &rows).map_err(|error| error.to_string())?;
        let expected_tiers = team
            .get("champion_tiers")
            .cloned()
            .ok_or_else(|| "Calculated champion_tiers is missing".to_owned())?;
        let readback = verified_write_whole_record(ctx, RecordKindV1::Team, team_id, &team)?;
        if readback.get("champion_tiers") != Some(&expected_tiers) {
            return Err(format!(
                "Team/{team_id} champion_tiers whole-record readback did not match the calculated policy"
            ));
        }
        Ok(rows.len())
    }

    fn current_tier_rows(&self, ctx: &StableServerCtx<'_>) -> Result<Vec<TierPolicyRow>, String> {
        let team_id = self
            .public
            .player_team_id
            .ok_or_else(|| "Player team is not connected".to_owned())?;
        let tiers_json = ctx
            .record_get_json(RecordKindV1::Team, team_id, "champion_tiers")
            .ok_or_else(|| "Player team champion_tiers could not be read".to_owned())?;
        let existing_tiers: Value =
            serde_json::from_str(&tiers_json).map_err(|error| error.to_string())?;
        let existing = existing_tiers
            .as_object()
            .ok_or_else(|| "Player team champion_tiers is not an object".to_owned())?;
        let registered = registered_champion_ids();
        Ok(build_tier_rows(
            existing.keys().cloned(),
            self.public.analytics.as_ref(),
            registered,
        ))
    }

    fn lock_values(&self) -> Vec<LockSet> {
        self.locks.values().cloned().collect()
    }

    fn publish(&mut self, surface: ApiSurface) {
        self.public.indexed_matches = self.index.match_count();
        self.public.pending_records = self.pending.len();
        self.public.data_revision = self.index.revision();
        self.public.locks = self.lock_values();
        if let Ok(champions) = hub().champion_info.read() {
            self.public.champion_info.clone_from(&champions);
        }
        self.public.champion_catalog = registered_champion_catalog_state();
        self.public
            .champion_info
            .clone_from(&self.public.champion_catalog.champions);
        let mut scopes = Vec::new();
        if let Ok(mut public) = hub().public_state.write() {
            let mut snapshot = self.public.clone();
            snapshot.tier_application.client_screen_verified =
                hub().tier_screen_verified.load(Ordering::Acquire);
            snapshot.connected = hub().client_connected.load(Ordering::Acquire);
            if !snapshot.connected {
                snapshot.mark_disconnected();
            }
            if public.champion_catalog != snapshot.champion_catalog {
                scopes.push("CATALOG_CHANGED");
            }
            if public.data_revision != snapshot.data_revision
                || public.engine_status != snapshot.engine_status
                || public.index_progress != snapshot.index_progress
            {
                scopes.push("INDEX_CHANGED");
            }
            if public.analytics != snapshot.analytics
                || public.active_profile != snapshot.active_profile
                || public.preview_profile != snapshot.preview_profile
                || public.tier_application != snapshot.tier_application
                || public.locks != snapshot.locks
                || public.connected != snapshot.connected
            {
                scopes.push(if surface == ApiSurface::Core {
                    "ANALYTICS_CHANGED"
                } else {
                    "EDITOR_CHANGED"
                });
            }
            *public = snapshot;
        }
        if !scopes.is_empty() {
            emit_state_changed(&scopes);
        }
    }
}

fn draft_team_options(
    ctx: &StableServerCtx<'_>,
    player_team_id: Option<usize>,
) -> Vec<DraftTeamOption> {
    let player_league_id = player_team_id.and_then(|team_id| {
        ctx.record_get_json(RecordKindV1::Team, team_id, "league_id")
            .and_then(|raw| serde_json::from_str::<usize>(&raw).ok())
    });
    let mut teams = ctx
        .record_ids(RecordKindV1::Team)
        .into_iter()
        .map(|team_id| {
            let team_name =
                team_name_from_record(ctx, team_id).unwrap_or_else(|| format!("Team {team_id}"));
            let league_id = ctx
                .record_get_json(RecordKindV1::Team, team_id, "league_id")
                .and_then(|raw| serde_json::from_str::<usize>(&raw).ok());
            (
                DraftTeamOption {
                    team_id,
                    team_name,
                    player_team: Some(team_id) == player_team_id,
                },
                league_id,
            )
        })
        .collect::<Vec<_>>();
    teams.sort_by(|left, right| {
        let left_same_league = player_league_id.is_some() && left.1 == player_league_id;
        let right_same_league = player_league_id.is_some() && right.1 == player_league_id;
        right
            .0
            .player_team
            .cmp(&left.0.player_team)
            .then_with(|| right_same_league.cmp(&left_same_league))
            .then_with(|| left.0.team_name.cmp(&right.0.team_name))
            .then_with(|| left.0.team_id.cmp(&right.0.team_id))
    });
    teams.into_iter().map(|(team, _)| team).collect()
}

fn team_name_from_record(ctx: &StableServerCtx<'_>, team_id: usize) -> Option<String> {
    let raw = ctx.record_get_json(RecordKindV1::Team, team_id, "")?;
    let team: Value = serde_json::from_str(&raw).ok()?;
    ["display_name", "name", "short_name"]
        .into_iter()
        .find_map(|key| team.get(key).and_then(Value::as_str))
        .map(ToOwned::to_owned)
}

fn relation_score(rows: &[crate::RelationRow], targets: &[String], invert: bool) -> f64 {
    if targets.is_empty() {
        return 50.0;
    }
    let values = targets
        .iter()
        .map(|target| {
            rows.iter()
                .find(|row| row.champion_id == *target)
                .map(|row| {
                    let reliability = row.games as f64 / (row.games as f64 + 10.0);
                    let rate = if invert {
                        100.0 - row.win_rate
                    } else {
                        row.win_rate
                    };
                    50.0 + (rate - 50.0) * reliability
                })
                .unwrap_or(50.0)
        })
        .sum::<f64>();
    values / targets.len() as f64
}

const ALL_DRAFT_ROLES: [RoleFilter; 5] = [
    RoleFilter::Top,
    RoleFilter::Jungle,
    RoleFilter::Mid,
    RoleFilter::Bot,
    RoleFilter::Support,
];

fn game_roles(
    champion_id: &str,
    catalog: &BTreeMap<String, ChampionBriefState>,
) -> Vec<RoleFilter> {
    catalog
        .get(champion_id)
        .map(|entry| entry.positions.clone())
        .filter(|roles| !roles.is_empty())
        .unwrap_or_else(|| ALL_DRAFT_ROLES.to_vec())
}

fn role_sets(
    picks: &[String],
    catalog: &BTreeMap<String, ChampionBriefState>,
) -> Vec<Vec<RoleFilter>> {
    picks
        .iter()
        .map(|champion_id| game_roles(champion_id, catalog))
        .collect()
}

fn remaining_roles_from_gate(gate: &tfm2_atlas_engine::RoleGateState) -> Vec<RoleFilter> {
    let roles = [
        RoleFilter::Top,
        RoleFilter::Jungle,
        RoleFilter::Mid,
        RoleFilter::Bot,
        RoleFilter::Support,
    ];
    if !gate.feasible {
        return roles.to_vec();
    }
    roles
        .into_iter()
        .filter(|role| !gate.definitely_filled.contains(role))
        .collect()
}

fn evaluate_mock_draft(state: &MockDraftState, analytics: &AnalyticsState) -> DraftEvaluation {
    let rows = analytics
        .champions
        .iter()
        .map(|row| (row.champion_id.as_str(), row))
        .collect::<BTreeMap<_, _>>();
    let current = &state.sets[state.current_set - 1];
    DraftEvaluation {
        blue: evaluate_mock_side(&current.blue_picks, &current.red_picks, &rows),
        red: evaluate_mock_side(&current.red_picks, &current.blue_picks, &rows),
    }
}

fn evaluate_mock_side(
    picks: &[String],
    opponents: &[String],
    rows: &BTreeMap<&str, &crate::ChampionAnalyticsRow>,
) -> Option<tfm2_atlas_engine::DraftSideEvaluation> {
    let op_scores = picks
        .iter()
        .map(|champion_id| {
            rows.get(champion_id.as_str())
                .and_then(|row| row.overall)
                .unwrap_or(50.0)
        })
        .collect::<Vec<_>>();
    let mut synergy_scores = Vec::new();
    for left in 0..picks.len() {
        for right in (left + 1)..picks.len() {
            if let Some(score) = relation_pair_score(
                rows.get(picks[left].as_str())
                    .map(|row| row.synergies.as_slice())
                    .unwrap_or_default(),
                &picks[right],
                false,
            ) {
                synergy_scores.push(score);
            }
        }
    }
    let mut matchup_scores = Vec::new();
    for champion_id in picks {
        let relations = rows
            .get(champion_id.as_str())
            .map(|row| row.matchups.as_slice())
            .unwrap_or_default();
        for opponent_id in opponents {
            if let Some(score) = relation_pair_score(relations, opponent_id, false) {
                matchup_scores.push(score);
            }
        }
    }
    evaluate_draft_side(
        &op_scores,
        &synergy_scores,
        &matchup_scores,
        picks.len().saturating_mul(picks.len().saturating_sub(1)) / 2,
        picks.len().saturating_mul(opponents.len()),
    )
}

fn relation_pair_score(rows: &[crate::RelationRow], target: &str, invert: bool) -> Option<f64> {
    rows.iter()
        .find(|row| row.champion_id == target)
        .map(|row| {
            let reliability = row.games as f64 / (row.games as f64 + 10.0);
            let rate = if invert {
                100.0 - row.win_rate
            } else {
                row.win_rate
            };
            50.0 + (rate - 50.0) * reliability
        })
}

fn project_lineup(
    analytics: &AnalyticsState,
    preferences: &BTreeMap<String, crate::TeamChampionPreference>,
    current: &[String],
    excluded: &std::collections::BTreeSet<String>,
    catalog: &BTreeMap<String, ChampionBriefState>,
) -> Vec<String> {
    let mut result = current.to_vec();
    let mut candidates = analytics
        .champions
        .iter()
        .filter(|row| !excluded.contains(&row.champion_id) && !result.contains(&row.champion_id))
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| {
        let score = |row: &crate::ChampionAnalyticsRow| {
            row.overall.unwrap_or(50.0) * 0.7
                + preferences
                    .get(&row.champion_id)
                    .map(|preference| preference.score)
                    .unwrap_or(50.0)
                    * 0.3
        };
        score(right)
            .total_cmp(&score(left))
            .then(left.champion_id.cmp(&right.champion_id))
    });
    let gate = analyze_role_gate(&role_sets(&result, catalog));
    for role in remaining_roles_from_gate(&gate) {
        if result.len() >= 5 {
            break;
        }
        let Some(index) = candidates
            .iter()
            .position(|row| game_roles(&row.champion_id, catalog).contains(&role))
        else {
            continue;
        };
        result.push(candidates.remove(index).champion_id.clone());
    }
    for row in candidates {
        if result.len() >= 5 {
            break;
        }
        result.push(row.champion_id.clone());
    }
    result
}

fn composition_state(
    champions: &[String],
    catalog: &BTreeMap<String, ChampionBriefState>,
) -> DraftComposition {
    let mut state = DraftComposition::default();
    for champion_id in champions {
        let Some(champion) = catalog.get(champion_id) else {
            continue;
        };
        for tag in &champion.tags {
            match tag.as_str() {
                "Ad" => state.ad += 1.0,
                "Ap" | "Magic" => state.ap += 1.0,
                "Tank" => state.tank += 1.0,
                "Heal" | "Shield" => state.utility += 1.0,
                "Cc" => state.cc += 1.0,
                _ => {}
            }
        }
    }
    state
}

fn composition_completion(
    current: &[String],
    candidate: Option<&ChampionBriefState>,
    catalog: &BTreeMap<String, ChampionBriefState>,
) -> f64 {
    let current = composition_state(current, catalog);
    let Some(candidate) = candidate else {
        return 50.0;
    };
    let has = |tag: &str| candidate.tags.iter().any(|candidate| candidate == tag);
    let mut values = Vec::new();
    for (missing, supplied) in [
        (current.ad < 1.0, has("Ad")),
        (current.ap < 1.0, has("Ap") || has("Magic")),
        (current.tank < 1.0, has("Tank")),
        (
            current.utility < 1.0,
            has("Heal") || has("Shield") || has("Cc"),
        ),
    ] {
        if missing {
            values.push(if supplied { 100.0 } else { 25.0 });
        }
    }
    if values.is_empty() {
        50.0
    } else {
        values.iter().sum::<f64>() / values.len() as f64
    }
}

fn build_tier_rows(
    existing_ids: impl IntoIterator<Item = String>,
    analytics: Option<&AnalyticsState>,
    registered_ids: impl IntoIterator<Item = String>,
) -> Vec<TierPolicyRow> {
    let scored: BTreeMap<_, _> = analytics
        .into_iter()
        .flat_map(|analytics| &analytics.champions)
        .map(|row| (row.champion_id.as_str(), row))
        .collect();
    let active_ids = registered_ids
        .into_iter()
        .collect::<std::collections::BTreeSet<_>>();
    let mut champion_ids = existing_ids
        .into_iter()
        .collect::<std::collections::BTreeSet<_>>();
    champion_ids.extend(active_ids.iter().cloned());
    champion_ids.extend(scored.keys().map(|champion_id| (*champion_id).to_owned()));
    champion_ids
        .into_iter()
        .map(|champion_id| {
            if !active_ids.contains(&champion_id) {
                return TierPolicyRow::new(&champion_id, Tier::NoTier, None, false);
            }
            scored.get(champion_id.as_str()).map_or_else(
                || TierPolicyRow::new(&champion_id, Tier::NoTier, None, false),
                |row| TierPolicyRow::new(&champion_id, row.tier, row.overall, row.eligible),
            )
        })
        .collect()
}

fn registered_champion_ids() -> Vec<String> {
    registered_champion_catalog()
        .into_iter()
        .map(|champion| champion.champion_id)
        .collect()
}

fn registered_champion_catalog() -> Vec<ChampionBriefState> {
    registered_champion_catalog_state().champions
}

fn registered_champion_catalog_state() -> ChampionCatalog {
    hub()
        .champion_catalog
        .read()
        .map(|catalog| catalog.clone())
        .unwrap_or_default()
}

fn player_mastery_state(
    ctx: &StableServerCtx<'_>,
    athlete_id: usize,
) -> Result<ChampionMasteryState, String> {
    let raw = ctx
        .record_get_json(RecordKindV1::Athlete, athlete_id, "")
        .ok_or_else(|| format!("athlete {athlete_id} was not found"))?;
    let record: Value = serde_json::from_str(&raw).map_err(|error| error.to_string())?;
    let mastery = record
        .get("champion_proficiency")
        .and_then(Value::as_object)
        .ok_or_else(|| format!("athlete {athlete_id}.champion_proficiency is not an object"))?;
    let catalog = registered_champion_catalog();
    let active_ids = catalog
        .iter()
        .map(|champion| champion.champion_id.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    let names = catalog
        .iter()
        .map(|champion| {
            (
                champion.champion_id.as_str(),
                champion.display_name.as_str(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let parse = |champion_id: &str, active: bool| -> Result<ChampionMasteryValue, String> {
        let entry = mastery.get(champion_id).and_then(Value::as_object);
        let raw_value = entry
            .and_then(|entry| entry.get("value"))
            .and_then(json_integer)
            .unwrap_or(0);
        let raw_floor = entry
            .and_then(|entry| entry.get("floor"))
            .and_then(json_integer)
            .unwrap_or(0);
        Ok(ChampionMasteryValue {
            champion_id: champion_id.to_owned(),
            display_name: names
                .get(champion_id)
                .copied()
                .map(ToOwned::to_owned)
                .unwrap_or_else(|| readable_champion_name(champion_id)),
            value: raw_value / 10,
            floor: raw_floor / 10,
            active,
        })
    };
    let mut active = catalog
        .iter()
        .map(|champion| parse(&champion.champion_id, true))
        .collect::<Result<Vec<_>, _>>()?;
    let mut inactive = mastery
        .keys()
        .filter(|champion_id| !active_ids.contains(champion_id.as_str()))
        .map(|champion_id| parse(champion_id, false))
        .collect::<Result<Vec<_>, _>>()?;
    active.sort_by(|left, right| {
        left.display_name
            .cmp(&right.display_name)
            .then(left.champion_id.cmp(&right.champion_id))
    });
    inactive.sort_by(|left, right| {
        left.display_name
            .cmp(&right.display_name)
            .then(left.champion_id.cmp(&right.champion_id))
    });
    Ok(ChampionMasteryState {
        athlete_id,
        active,
        inactive,
    })
}

fn json_integer(value: &Value) -> Option<i64> {
    value
        .as_i64()
        .or_else(|| value.as_u64().and_then(|value| i64::try_from(value).ok()))
}

fn api_command_mutates(command: ApiCommand) -> bool {
    matches!(
        command,
        ApiCommand::ApplyTierProfile
            | ApiCommand::SetLock
            | ApiCommand::Unlock
            | ApiCommand::SetPlayerMastery
            | ApiCommand::ApplyPlayerEdit
            | ApiCommand::ApplyStaffEdit
            | ApiCommand::MovePlayer
            | ApiCommand::ApplyEditorSettings
            | ApiCommand::SetDraftSettings
    )
}

fn serialize_saved_profile(profile: &TierProfile) -> Result<String, String> {
    profile.validate().map_err(|error| error.to_string())?;
    serde_json::to_string(profile).map_err(|error| error.to_string())
}

fn deserialize_saved_profile(raw: &str) -> Result<TierProfile, String> {
    let profile: TierProfile = serde_json::from_str(raw).map_err(|error| error.to_string())?;
    profile.validate().map_err(|error| error.to_string())?;
    Ok(profile)
}

pub fn validate_community_shapes(
    athlete: &Value,
    staff: &Value,
    team: &Value,
) -> Result<(), Vec<String>> {
    const ATHLETE_PATHS: &[&str] = &[
        "/id",
        "/name",
        "/age",
        "/stat/last_hit",
        "/stat/skill_avoid",
        "/stat/skill_hit",
        "/stat/control_speed",
        "/stat/positioning",
        "/stat/judgement",
        "/stat/mental",
        "/stat/concentration",
        "/stat/order",
        "/stat/roaming",
        "/stat/aggressive",
        "/stat/ego",
        "/stat/top",
        "/stat/jungle",
        "/stat/mid",
        "/stat/bottom",
        "/stat/support",
        "/stat/language",
        "/hidden/potential",
        "/contract",
        "/champion_proficiency",
        "/management/stamina",
        "/management/stress",
        "/management/condition",
        "/training_exp/language_by_region",
    ];
    const STAFF_PATHS: &[&str] = &[
        "/id",
        "/name",
        "/age",
        "/role",
        "/stat/banpick",
        "/stat/strategy",
        "/stat/negotiation",
        "/stat/judge_ability",
        "/stat/judge_potential",
        "/stat/feedback",
        "/stat/power_analysis",
        "/stat/control_coaching",
        "/stat/judgment_coaching",
        "/stat/mental_coaching",
        "/language",
        "/contract",
    ];
    const TEAM_PATHS: &[&str] = &[
        "/id",
        "/manager_name",
        "/league_id",
        "/champion_tiers",
        "/total_balance",
        "/transfer_budget",
        "/salary_budget",
        "/training_facility_grade",
        "/merchandise_facility_grade",
        "/stadium/grade",
    ];

    const ATHLETE_NUMBERS: &[&str] = &[
        "/id",
        "/age",
        "/stat/last_hit",
        "/stat/skill_avoid",
        "/stat/skill_hit",
        "/stat/control_speed",
        "/stat/positioning",
        "/stat/judgement",
        "/stat/mental",
        "/stat/concentration",
        "/stat/order",
        "/stat/roaming",
        "/stat/aggressive",
        "/stat/ego",
        "/stat/top",
        "/stat/jungle",
        "/stat/mid",
        "/stat/bottom",
        "/stat/support",
        "/hidden/potential",
        "/management/stamina",
        "/management/stress",
        "/management/condition",
    ];
    const STAFF_NUMBERS: &[&str] = &[
        "/id",
        "/age",
        "/stat/banpick",
        "/stat/strategy",
        "/stat/negotiation",
        "/stat/judge_ability",
        "/stat/judge_potential",
        "/stat/feedback",
        "/stat/power_analysis",
        "/stat/control_coaching",
        "/stat/judgment_coaching",
        "/stat/mental_coaching",
    ];
    const TEAM_NUMBERS: &[&str] = &[
        "/id",
        "/league_id",
        "/total_balance",
        "/transfer_budget",
        "/salary_budget",
    ];
    const ATHLETE_OBJECTS: &[&str] = &[
        "/stat/language",
        "/contract",
        "/champion_proficiency",
        "/training_exp/language_by_region",
    ];
    const STAFF_OBJECTS: &[&str] = &["/language", "/contract"];
    const TEAM_OBJECTS: &[&str] = &["/champion_tiers"];

    let mut failures = Vec::new();
    for (label, value, paths) in [
        ("Athlete", athlete, ATHLETE_PATHS),
        ("Staff", staff, STAFF_PATHS),
        ("Team", team, TEAM_PATHS),
    ] {
        for path in paths {
            if value.pointer(path).is_none() {
                failures.push(format!(
                    "{label}.{}",
                    path.trim_start_matches('/').replace('/', ".")
                ));
            }
        }
    }
    for (label, value, paths) in [
        ("Athlete", athlete, ATHLETE_NUMBERS),
        ("Staff", staff, STAFF_NUMBERS),
        ("Team", team, TEAM_NUMBERS),
    ] {
        for path in paths {
            if value.pointer(path).is_some_and(|field| !field.is_number()) {
                failures.push(format!(
                    "{label}.{} (expected number)",
                    path.trim_start_matches('/').replace('/', ".")
                ));
            }
        }
    }
    for (label, value, paths) in [
        ("Athlete", athlete, ATHLETE_OBJECTS),
        ("Staff", staff, STAFF_OBJECTS),
        ("Team", team, TEAM_OBJECTS),
    ] {
        for path in paths {
            if value.pointer(path).is_some_and(|field| !field.is_object()) {
                failures.push(format!(
                    "{label}.{} (expected object)",
                    path.trim_start_matches('/').replace('/', ".")
                ));
            }
        }
    }
    for (label, value, path) in [
        ("Athlete", athlete, "/name"),
        ("Staff", staff, "/name"),
        ("Staff", staff, "/role"),
        ("Team", team, "/manager_name"),
        ("Team", team, "/training_facility_grade"),
        ("Team", team, "/merchandise_facility_grade"),
        ("Team", team, "/stadium/grade"),
    ] {
        if value.pointer(path).is_some_and(|field| !field.is_string()) {
            failures.push(format!(
                "{label}.{} (expected string)",
                path.trim_start_matches('/').replace('/', ".")
            ));
        }
    }
    if failures.is_empty() {
        Ok(())
    } else {
        Err(failures)
    }
}

fn champion_stat(stat: mod_api_stable::StatV1) -> ChampionStatState {
    ChampionStatState {
        attack: stat.attack,
        magic_power: stat.magic_power,
        hp: stat.hp,
        defence: stat.defence,
        magic_resistance: stat.magic_resistance,
        move_speed: stat.move_speed,
        hp_regen: stat.hp_regen,
        stack: stat.stack,
        crit_chance: stat.crit_chance,
    }
}

fn localized_champion_name(ctx: &StableClient<'_>, champion_id: &str) -> String {
    let key = format!("#asset/base/text/champion?description.{champion_id}.name");
    ctx.i18n(&key)
        .filter(|value| {
            let value = value.trim();
            !value.is_empty() && value != key && value != champion_id
        })
        .unwrap_or_else(|| readable_champion_name(champion_id))
}

fn readable_champion_name(champion_id: &str) -> String {
    let readable = champion_id
        .split(|character: char| character == '_' || character == '-')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut characters = part.chars();
            match characters.next() {
                Some(first) => first.to_uppercase().collect::<String>() + characters.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ");
    if readable.is_empty() {
        "Unknown Champion".to_owned()
    } else {
        readable
    }
}

fn champion_state(
    champion_id: String,
    display_name: String,
    brief: Option<mod_api_stable::StableChampionBrief>,
) -> ChampionBriefState {
    let Some(brief) = brief else {
        return ChampionBriefState {
            champion_id,
            display_name,
            category: None,
            tags: Vec::new(),
            positions: Vec::new(),
            main_position: None,
            stat: champion_stat(mod_api_stable::StatV1::default()),
            growth: champion_stat(mod_api_stable::StatV1::default()),
        };
    };
    ChampionBriefState {
        champion_id,
        display_name,
        category: brief.category.map(|value| format!("{value:?}")),
        tags: brief
            .tags
            .into_iter()
            .map(|value| format!("{value:?}"))
            .collect(),
        positions: Vec::new(),
        main_position: None,
        stat: champion_stat(brief.stat),
        growth: champion_stat(brief.growth),
    }
}

const CLIENT_PLAYER_STATS: [&str; 12] = [
    "last_hit",
    "skill_avoid",
    "skill_hit",
    "control_speed",
    "positioning",
    "judgement",
    "mental",
    "concentration",
    "order",
    "roaming",
    "aggressive",
    "ego",
];

const CLIENT_STAFF_STATS: [&str; 10] = [
    "banpick",
    "strategy",
    "negotiation",
    "judge_ability",
    "judge_potential",
    "feedback",
    "power_analysis",
    "control_coaching",
    "judgment_coaching",
    "mental_coaching",
];

fn player_detail_value_paths(field: &str) -> Vec<String> {
    [
        format!("main.top.right.player_detail.data.info.{field}.value"),
        format!("main.top.right.player_detail.info.{field}.value"),
    ]
    .into_iter()
    .collect()
}

fn staff_detail_value_paths(field: &str) -> Vec<String> {
    [
        format!("main.top.right.staff_detail.{field}.value"),
        format!("main.top.right.staff_detail.info.{field}.value"),
        format!("main.top.right.staff_detail.data.{field}.value"),
    ]
    .into_iter()
    .collect()
}

fn record_name(record: &Value) -> Option<String> {
    record
        .get("name")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn value_text(value: &Value) -> Option<String> {
    match value {
        Value::String(value) => Some(value.clone()),
        Value::Number(value) => Some(value.to_string()),
        Value::Bool(value) => Some(value.to_string()),
        _ => None,
    }
}

fn first_ui_text(ctx: &StableClient<'_>, paths: &[&str]) -> Option<String> {
    paths
        .iter()
        .find_map(|path| ctx.ui_text(path))
        .map(|value| value.trim().to_owned())
}

fn set_ui_text(
    ctx: &mut StableClient<'_>,
    paths: impl IntoIterator<Item = String>,
    text: &str,
) -> bool {
    paths.into_iter().fold(false, |updated, path| {
        ctx.ui_set_text(&path, text) || updated
    })
}

fn visible_record<'a>(
    records: &'a BTreeMap<usize, ClientRecordSync>,
    visible_name: &str,
) -> Option<&'a ClientRecordSync> {
    records.values().rev().find(|entry| {
        record_name(&entry.record).as_deref() == Some(visible_name)
            || entry.previous_name.as_deref() == Some(visible_name)
    })
}

fn sync_visible_player_detail(ctx: &mut StableClient<'_>, sync: &ClientUiSyncState) {
    let name_paths = [
        "main.top.right.player_detail.data.info.header.name",
        "main.top.right.player_detail.info.header.name",
    ];
    let Some(visible_name) = first_ui_text(ctx, &name_paths) else {
        return;
    };
    let Some(entry) = visible_record(&sync.athletes, &visible_name) else {
        return;
    };
    if let Some(name) = record_name(&entry.record) {
        set_ui_text(ctx, name_paths.map(str::to_owned), &name);
    }
    for field in CLIENT_PLAYER_STATS {
        let Some(text) = entry
            .record
            .pointer(&format!("/stat/{field}"))
            .and_then(value_text)
        else {
            continue;
        };
        set_ui_text(ctx, player_detail_value_paths(field), &text);
    }
    for (field, path) in [
        ("stamina", "/management/stamina"),
        ("stress", "/management/stress"),
        ("condition", "/management/condition"),
    ] {
        let Some(text) = entry.record.pointer(path).and_then(value_text) else {
            continue;
        };
        set_ui_text(
            ctx,
            [
                format!("main.top.right.player_detail.data.row1.champion.{field}.value"),
                format!("main.top.right.player_detail.row1.champion.{field}.value"),
            ],
            &text,
        );
    }
}

fn sync_visible_staff_detail(ctx: &mut StableClient<'_>, sync: &ClientUiSyncState) {
    let name_paths = [
        "main.top.right.staff_detail.info.header.name",
        "main.top.right.staff_detail.data.info.header.name",
    ];
    let Some(visible_name) = first_ui_text(ctx, &name_paths) else {
        return;
    };
    let Some(entry) = visible_record(&sync.staffs, &visible_name) else {
        return;
    };
    if let Some(name) = record_name(&entry.record) {
        set_ui_text(ctx, name_paths.map(str::to_owned), &name);
    }
    for field in CLIENT_STAFF_STATS {
        let Some(text) = entry
            .record
            .pointer(&format!("/stat/{field}"))
            .and_then(value_text)
        else {
            continue;
        };
        set_ui_text(ctx, staff_detail_value_paths(field), &text);
    }
}

fn sync_visible_management_ui(ctx: &mut StableClient<'_>) {
    let tab = ctx.client_main_tab().unwrap_or_default();
    if !matches!(tab.as_str(), "PlayerDetail" | "StaffDetail") {
        return;
    }
    let sync = hub()
        .client_ui_sync
        .read()
        .map(|value| value.clone())
        .unwrap_or_default();
    match tab.as_str() {
        "PlayerDetail" => sync_visible_player_detail(ctx, &sync),
        "StaffDetail" => sync_visible_staff_detail(ctx, &sync),
        _ => {}
    }
}

fn bridge_error_response(request: &BridgeRequest, code: &str, message: &str) -> BridgeResponse {
    match request {
        BridgeRequest::Api(request) => {
            BridgeResponse::Api(ApiResponse::error(request.request_id, code, message))
        }
        BridgeRequest::TierSync => BridgeResponse::TierSync("ERR|CAREER_NOT_CONNECTED".to_owned()),
    }
}

fn fail_routed_response(route_id: u64, code: &str, message: &str) {
    if let Ok(mut published) = hub().published_responses.lock() {
        published.remove(&route_id);
    }
    let envelope = hub()
        .routed_requests
        .lock()
        .ok()
        .and_then(|mut pending| pending.remove(&route_id));
    if let Some(envelope) = envelope {
        let response = bridge_error_response(&envelope.request, code, message);
        complete_envelope(envelope, response);
    }
}

fn complete_envelope(envelope: RequestEnvelope, response: BridgeResponse) {
    envelope.execution.store(REQUEST_DONE, Ordering::Release);
    let _ = envelope.response.send(response);
}

fn pump_bridge_events(ctx: &mut StableClient<'_>, surface: ApiSurface) {
    for event in ctx.take_events() {
        if event.event != bridge_response_event(surface) {
            continue;
        }
        let Ok(routed) = serde_json::from_slice::<RoutedBridgeResponse>(&event.payload) else {
            continue;
        };
        let envelope = hub()
            .routed_requests
            .lock()
            .ok()
            .and_then(|mut pending| pending.remove(&routed.route_id));
        if let Some(envelope) = envelope {
            let response = match routed.response {
                BridgeResponse::Published => hub()
                    .published_responses
                    .lock()
                    .ok()
                    .and_then(|mut published| published.remove(&routed.route_id))
                    .unwrap_or_else(|| {
                        bridge_error_response(
                            &envelope.request,
                            "published_response_missing",
                            "Career server published a completion event without response data",
                        )
                    }),
                response => response,
            };
            complete_envelope(envelope, response);
        }
    }
}

fn pump_bridge_requests(ctx: &mut StableClient<'_>, surface: ApiSurface) {
    let envelopes = {
        let Ok(receiver) = hub().request_rx.lock() else {
            return;
        };
        let mut envelopes = Vec::new();
        while let Ok(envelope) = receiver.try_recv() {
            envelopes.push(envelope);
        }
        envelopes
    };
    for envelope in envelopes {
        if !try_begin_request(&envelope.execution) {
            continue;
        }
        let route_id = hub().next_route_id.fetch_add(1, Ordering::AcqRel);
        let routed = RoutedBridgeRequest {
            route_id,
            request: envelope.request.clone(),
        };
        let payload = match serde_json::to_vec(&routed) {
            Ok(payload) => payload,
            Err(error) => {
                let response = bridge_error_response(
                    &envelope.request,
                    "request_serialization_failed",
                    &error.to_string(),
                );
                complete_envelope(envelope, response);
                continue;
            }
        };
        if let Ok(mut pending) = hub().routed_requests.lock() {
            pending.insert(route_id, envelope);
        } else {
            continue;
        }
        if !ctx.send_command(bridge_command(surface), &payload) {
            let envelope = hub()
                .routed_requests
                .lock()
                .ok()
                .and_then(|mut pending| pending.remove(&route_id));
            if let Some(envelope) = envelope {
                let response = bridge_error_response(
                    &envelope.request,
                    "career_not_connected",
                    "Career server command channel is unavailable",
                );
                complete_envelope(envelope, response);
            }
        }
    }
}

fn fail_routed_requests(message: &str) {
    if let Ok(mut published) = hub().published_responses.lock() {
        published.clear();
    }
    let envelopes = hub()
        .routed_requests
        .lock()
        .map(|mut pending| std::mem::take(&mut *pending))
        .unwrap_or_default();
    for (_, envelope) in envelopes {
        let response = bridge_error_response(&envelope.request, "career_not_connected", message);
        complete_envelope(envelope, response);
    }
}

pub fn start_bridge_server(surface: ApiSurface, address: &'static str) {
    let bridge = Arc::clone(hub());
    if bridge.listener_started.swap(true, Ordering::AcqRel) {
        return;
    }
    thread::spawn(move || {
        let Ok(listener) = TcpListener::bind(address) else {
            if let Ok(mut state) = bridge.public_state.write() {
                state.diagnostics.push(Diagnostic::new(
                    "bridge_port_in_use",
                    format!("Could not bind localhost port {address}"),
                    true,
                ));
                state.engine_status = EngineStatus::Error;
            }
            return;
        };
        for stream in listener.incoming().flatten() {
            let client_bridge = Arc::clone(&bridge);
            thread::spawn(move || handle_stream(stream, &client_bridge, surface));
        }
    });
}

fn handle_stream(mut stream: TcpStream, bridge: &Arc<BridgeHub>, surface: ApiSurface) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(3)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(3)));
    let mut first = [0u8; 1];
    if stream.read_exact(&mut first).is_err() {
        return;
    }
    if is_frame_prefix(first[0]) {
        handle_frame_stream(stream, first[0], bridge, surface);
    } else {
        handle_line_stream(stream, first[0], bridge);
    }
}

fn is_frame_prefix(first: u8) -> bool {
    first <= (MAX_FRAME_BYTES >> 24) as u8
}

fn handle_frame_stream(
    mut stream: TcpStream,
    first: u8,
    bridge: &Arc<BridgeHub>,
    surface: ApiSurface,
) {
    let mut header = [0u8; 4];
    header[0] = first;
    if stream.read_exact(&mut header[1..]).is_err() {
        return;
    }
    let length = u32::from_be_bytes(header) as usize;
    if length > MAX_FRAME_BYTES {
        let response = ApiResponse::error(0, "frame_too_large", "JSON frame exceeds 16 MiB");
        write_api_response(&mut stream, &response);
        return;
    }
    let mut payload = vec![0u8; length];
    if stream.read_exact(&mut payload).is_err() {
        return;
    }
    let mut frame = header.to_vec();
    frame.extend_from_slice(&payload);
    let request = match decode_json_frame::<ApiRequest>(&frame) {
        Ok(request) => request,
        Err(error) => {
            write_api_response(
                &mut stream,
                &ApiResponse::error(0, "invalid_frame", error.to_string()),
            );
            return;
        }
    };
    if !surface.allows(request.command) {
        write_api_response(
            &mut stream,
            &ApiResponse::error(
                request.request_id,
                "command_not_allowed",
                "The command is not available on this Atlas service",
            ),
        );
        return;
    }
    if request.command == ApiCommand::SubscribeState {
        let response = ApiResponse::ok(request.request_id, json!({ "subscribed": true }));
        write_api_response(&mut stream, &response);
        let _ = stream.set_read_timeout(None);
        let _ = stream.set_write_timeout(Some(Duration::from_millis(500)));
        if let Ok(mut subscribers) = bridge.subscribers.lock() {
            subscribers.push(stream);
        }
        return;
    }
    if matches!(
        request.command,
        ApiCommand::GetDashboard | ApiCommand::GetLocks
    ) {
        if let Ok(state) = bridge.public_state.read() {
            let data = if request.command == ApiCommand::GetLocks {
                serde_json::to_value(&state.locks)
            } else {
                serde_json::to_value(&*state)
            };
            let response = data.map_or_else(
                |error| {
                    ApiResponse::error(
                        request.request_id,
                        "serialization_failed",
                        error.to_string(),
                    )
                },
                |data| ApiResponse::ok(request.request_id, data),
            );
            write_api_response(&mut stream, &response);
            return;
        }
    }
    if request.command == ApiCommand::GetChampionCatalog {
        if let Ok(catalog) = bridge.champion_catalog.read() {
            let response = serde_json::to_value(&*catalog).map_or_else(
                |error| {
                    ApiResponse::error(
                        request.request_id,
                        "serialization_failed",
                        error.to_string(),
                    )
                },
                |data| ApiResponse::ok(request.request_id, data),
            );
            write_api_response(&mut stream, &response);
            return;
        }
    }
    let request_id = request.request_id;
    let response = enqueue_request(bridge, BridgeRequest::Api(request));
    match response {
        Ok(BridgeResponse::Api(response)) => write_api_response(&mut stream, &response),
        Ok(BridgeResponse::TierSync(_) | BridgeResponse::Published) => {}
        Err(message) => write_api_response(
            &mut stream,
            &ApiResponse::error(request_id, "career_not_connected", message),
        ),
    }
}

fn emit_state_changed(scopes: &[&str]) {
    let event = json!({
        "event": "STATE_CHANGED",
        "scopes": scopes,
        "changed_at_unix_ms": unix_millis(),
    });
    let Ok(frame) = encode_json_frame(&event) else {
        return;
    };
    if let Ok(mut subscribers) = hub().subscribers.lock() {
        subscribers.retain_mut(|stream| stream.write_all(&frame).is_ok() && stream.flush().is_ok());
    }
}

fn handle_line_stream(mut stream: TcpStream, first: u8, bridge: &Arc<BridgeHub>) {
    let mut bytes = vec![first];
    while bytes.len() <= 1024 * 1024 {
        let mut byte = [0u8; 1];
        match stream.read(&mut byte) {
            Ok(0) | Err(_) => break,
            Ok(_) if byte[0] == b'\n' => break,
            Ok(_) => bytes.push(byte[0]),
        }
    }
    let line = String::from_utf8_lossy(&bytes).trim().to_owned();
    let response = if let Some(payload) = line.strip_prefix("SYNC_ACTIVE_CHAMPIONS|") {
        match parse_active_champion_payload(payload) {
            Ok(ids) => {
                if let Ok(mut active) = bridge.active_champions.write() {
                    *active = (!ids.is_empty()).then_some(ids);
                }
                bridge.catalog_epoch.fetch_add(1, Ordering::AcqRel);
                "OK|SYNC_ACTIVE_CHAMPIONS".to_owned()
            }
            Err(message) => format!("ERR|{message}"),
        }
    } else if let Some(payload) = line.strip_prefix("SYNC_TIER_STATUS|") {
        let values = payload.split('|').collect::<Vec<_>>();
        let parsed = (values.len() == 3)
            .then(|| {
                Some((
                    values[0].parse::<usize>().ok()?,
                    values[1].parse::<usize>().ok()?,
                    values[2].parse::<usize>().ok()?,
                ))
            })
            .flatten();
        match parsed {
            Some((team_id, applied, expected)) => {
                let player_team = bridge.player_team_id.load(Ordering::Acquire);
                let verified = expected > 0 && applied == expected && team_id == player_team;
                if bridge.tier_screen_verified.swap(verified, Ordering::AcqRel) != verified {
                    emit_state_changed(&["ANALYTICS_CHANGED"]);
                }
                "OK|SYNC_TIER_STATUS".to_owned()
            }
            None => "ERR|INVALID_TIER_STATUS".to_owned(),
        }
    } else if line == "GET_TIER_SYNC" {
        match enqueue_request(bridge, BridgeRequest::TierSync) {
            Ok(BridgeResponse::TierSync(response)) => response,
            Ok(BridgeResponse::Api(_) | BridgeResponse::Published) => {
                "ERR|INTERNAL_MESSAGE_MISMATCH".to_owned()
            }
            Err(_) => "ERR|CAREER_NOT_CONNECTED".to_owned(),
        }
    } else {
        "ERR|UNKNOWN_INTERNAL_MESSAGE".to_owned()
    };
    let _ = writeln!(stream, "{response}");
    let _ = stream.flush();
}

fn parse_active_champion_payload(
    payload: &str,
) -> Result<Vec<ActiveChampionCatalogEntry>, &'static str> {
    let mut entries = payload
        .split(';')
        .filter(|value| !value.is_empty())
        .map(|entry| {
            let (id, positions) = entry.split_once(',').unwrap_or((entry, ""));
            let champion_id = hex_decode(id)?;
            let mut parsed_positions = Vec::new();
            for position in positions.split('+').filter(|value| !value.is_empty()) {
                let role = match position {
                    "Top" => RoleFilter::Top,
                    "Jungle" => RoleFilter::Jungle,
                    "Mid" => RoleFilter::Mid,
                    "Bottom" => RoleFilter::Bot,
                    "Support" => RoleFilter::Support,
                    _ => return Err("INVALID_CHAMPION_POSITION"),
                };
                if !parsed_positions.contains(&role) {
                    parsed_positions.push(role);
                }
                if parsed_positions.len() == 2 {
                    break;
                }
            }
            Ok(ActiveChampionCatalogEntry {
                champion_id,
                positions: parsed_positions,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    entries.retain(|entry| !entry.champion_id.trim().is_empty());
    entries.sort_by(|left, right| left.champion_id.cmp(&right.champion_id));
    entries.dedup_by(|left, right| left.champion_id == right.champion_id);
    Ok(entries)
}

fn enqueue_request(bridge: &BridgeHub, request: BridgeRequest) -> Result<BridgeResponse, String> {
    let (response_tx, response_rx) = mpsc::channel();
    let execution = Arc::new(AtomicU8::new(REQUEST_PENDING));
    bridge
        .request_tx
        .send(RequestEnvelope {
            request,
            response: response_tx,
            execution: Arc::clone(&execution),
        })
        .map_err(|_| "Bridge request channel is unavailable".to_owned())?;
    match response_rx.recv_timeout(Duration::from_secs(2)) {
        Ok(response) => Ok(response),
        Err(mpsc::RecvTimeoutError::Disconnected) => {
            Err("Career server request channel disconnected".to_owned())
        }
        Err(mpsc::RecvTimeoutError::Timeout) if cancel_pending_request(&execution) => {
            Err("Career server did not answer in time; the request was cancelled".to_owned())
        }
        Err(mpsc::RecvTimeoutError::Timeout) => response_rx
            .recv()
            .map_err(|_| "Career server stopped while processing the request".to_owned()),
    }
}

fn try_begin_request(execution: &AtomicU8) -> bool {
    execution
        .compare_exchange(
            REQUEST_PENDING,
            REQUEST_PROCESSING,
            Ordering::AcqRel,
            Ordering::Acquire,
        )
        .is_ok()
}

fn cancel_pending_request(execution: &AtomicU8) -> bool {
    execution
        .compare_exchange(
            REQUEST_PENDING,
            REQUEST_CANCELLED,
            Ordering::AcqRel,
            Ordering::Acquire,
        )
        .is_ok()
}

fn write_api_response(stream: &mut TcpStream, response: &ApiResponse) {
    if let Ok(frame) = encode_json_frame(response) {
        let _ = stream.write_all(&frame);
        let _ = stream.flush();
    }
}

fn validate_first_record(
    ctx: &StableServerCtx<'_>,
    kind: RecordKindV1,
    label: &str,
    paths: &[&str],
    failures: &mut Vec<Diagnostic>,
) {
    let ids = ctx.record_ids(kind);
    let Some(id) = ids.first().copied() else {
        failures.push(Diagnostic::new(
            "record_table_empty",
            format!("{label} record table is empty"),
            true,
        ));
        return;
    };
    let Some(json) = ctx.record_get_json(kind, id, "") else {
        failures.push(Diagnostic::new(
            "record_read_failed",
            format!("Could not read {label} record {id}"),
            true,
        ));
        return;
    };
    let Ok(value) = serde_json::from_str::<Value>(&json) else {
        failures.push(Diagnostic::new(
            "record_json_invalid",
            format!("{label} record {id} is not valid JSON"),
            true,
        ));
        return;
    };
    for path in paths {
        if value.get(*path).is_none() {
            failures.push(Diagnostic::new(
                "record_path_missing",
                format!("{label}.{path} is unavailable in 0.5.5 record JSON"),
                true,
            ));
        }
    }
}

fn tournament_region_division(
    ctx: &StableServerCtx<'_>,
    replay_json: &str,
) -> Result<(tfm2_atlas_engine::RegionFilter, Division), crate::ReplayError> {
    let replay: Value = serde_json::from_str(replay_json)
        .map_err(|error| crate::ReplayError::InvalidJson(error.to_string()))?;
    let team_id = replay
        .get("blue_team_id")
        .and_then(Value::as_u64)
        .ok_or_else(|| crate::ReplayError::MissingField("blue_team_id".to_owned()))?
        as usize;
    let league_id = ctx
        .record_get_json(RecordKindV1::Team, team_id, "league_id")
        .and_then(|value| serde_json::from_str::<usize>(&value).ok())
        .ok_or_else(|| crate::ReplayError::MissingField("team.league_id".to_owned()))?;
    let (region_id, division) = if league_id < 6 {
        (league_id, Division::First)
    } else if league_id < 12 {
        (league_id - 6, Division::Second)
    } else {
        return Err(crate::ReplayError::InvalidField(format!(
            "team.league_id:{league_id}"
        )));
    };
    let region = [
        tfm2_atlas_engine::RegionFilter::Kr,
        tfm2_atlas_engine::RegionFilter::Cn,
        tfm2_atlas_engine::RegionFilter::Eu,
        tfm2_atlas_engine::RegionFilter::Na,
        tfm2_atlas_engine::RegionFilter::Sa,
        tfm2_atlas_engine::RegionFilter::Jp,
    ][region_id];
    Ok((region, division))
}

fn apply_lock_once(ctx: &mut StableServerCtx<'_>, lock: &LockSet) -> Result<(), String> {
    let kind = match lock.group {
        LockGroup::PlayerStats | LockGroup::ChampionMastery | LockGroup::PlayerProfile => {
            RecordKindV1::Athlete
        }
        LockGroup::StaffStats | LockGroup::StaffProfile => RecordKindV1::Staff,
    };
    let json = ctx
        .record_get_json(kind, lock.target_id, "")
        .ok_or_else(|| format!("record {} was not found", lock.target_id))?;
    let mut record: Value = serde_json::from_str(&json).map_err(|error| error.to_string())?;
    apply_lock_to_record(&mut record, lock).map_err(|error| error.to_string())?;
    verified_write_whole_record(ctx, kind, lock.target_id, &record).map(|_| ())
}

pub(crate) fn hex_encode(value: &str) -> String {
    value
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

pub(crate) fn hex_decode(value: &str) -> Result<String, &'static str> {
    if value.len() % 2 != 0 {
        return Err("INVALID_HEX");
    }
    let bytes = (0..value.len())
        .step_by(2)
        .map(|offset| u8::from_str_radix(&value[offset..offset + 2], 16))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| "INVALID_HEX")?;
    String::from_utf8(bytes).map_err(|_| "INVALID_HEX")
}

pub(crate) fn patch_record(
    ctx: &mut StableServerCtx<'_>,
    kind: RecordKindV1,
    id: usize,
    patch: impl FnOnce(&mut Value) -> Result<(), String>,
) -> Result<Value, String> {
    let json = ctx
        .record_get_json(kind, id, "")
        .ok_or_else(|| "RECORD_NOT_FOUND".to_owned())?;
    let mut value: Value =
        serde_json::from_str(&json).map_err(|_| "INVALID_RECORD_JSON".to_owned())?;
    patch(&mut value)?;
    verified_write_whole_record(ctx, kind, id, &value)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WholeRecordWriteRoute {
    Team,
    Athlete,
    Generic,
}

fn whole_record_write_route(kind: RecordKindV1) -> WholeRecordWriteRoute {
    match kind {
        RecordKindV1::Team => WholeRecordWriteRoute::Team,
        RecordKindV1::Athlete => WholeRecordWriteRoute::Athlete,
        _ => WholeRecordWriteRoute::Generic,
    }
}

pub(crate) fn publish_client_record_sync(
    kind: RecordKindV1,
    id: usize,
    previous: &Value,
    record: &Value,
) {
    let Ok(mut sync) = hub().client_ui_sync.write() else {
        return;
    };
    let entry = ClientRecordSync {
        previous_name: record_name(previous),
        record: record.clone(),
    };
    match kind {
        RecordKindV1::Athlete => {
            sync.athletes.insert(id, entry);
        }
        RecordKindV1::Staff => {
            sync.staffs.insert(id, entry);
        }
        RecordKindV1::Team => {
            let next = record
                .get("champion_tiers")
                .and_then(Value::as_object)
                .map(|tiers| {
                    tiers
                        .iter()
                        .filter_map(|(champion_id, tier)| {
                            tier.as_str()
                                .map(|tier| (champion_id.clone(), tier.to_owned()))
                        })
                        .collect::<BTreeMap<_, _>>()
                })
                .unwrap_or_default();
            if sync.champion_tiers != next {
                sync.champion_tiers = next;
                hub().tier_screen_verified.store(false, Ordering::Release);
            }
        }
        _ => {}
    }
}

pub(crate) fn verified_write_whole_record(
    ctx: &mut StableServerCtx<'_>,
    kind: RecordKindV1,
    id: usize,
    candidate: &Value,
) -> Result<Value, String> {
    let current_raw = ctx
        .record_get_json(kind, id, "")
        .ok_or_else(|| format!("record {kind:?}/{id} could not be read before write"))?;
    let current: Value = serde_json::from_str(&current_raw)
        .map_err(|error| format!("record {kind:?}/{id} is invalid JSON before write: {error}"))?;
    if current == *candidate {
        publish_client_record_sync(kind, id, &current, &current);
        return Ok(current);
    }
    let mut requested_paths = Vec::new();
    collect_changed_paths(None, &current, candidate, &mut requested_paths);
    let updated = serde_json::to_string(candidate)
        .map_err(|error| format!("record {kind:?}/{id} could not be serialized: {error}"))?;
    let accepted = match whole_record_write_route(kind) {
        WholeRecordWriteRoute::Team => ctx.team_set_json(id, "", &updated),
        WholeRecordWriteRoute::Athlete => ctx.athlete_set_json(id, "", &updated),
        WholeRecordWriteRoute::Generic => ctx.record_set_json(kind, id, "", &updated),
    };
    if !accepted {
        return Err(format!("host rejected whole-record write to {kind:?}/{id}"));
    }
    let readback_raw = ctx
        .record_get_json(kind, id, "")
        .ok_or_else(|| format!("record {kind:?}/{id} could not be read after write"))?;
    let readback: Value = serde_json::from_str(&readback_raw)
        .map_err(|error| format!("record {kind:?}/{id} is invalid JSON after write: {error}"))?;
    verify_requested_paths(kind, id, candidate, &readback, &requested_paths)?;
    publish_client_record_sync(kind, id, &current, &readback);
    Ok(readback)
}

fn collect_changed_paths(
    prefix: Option<&str>,
    current: &Value,
    candidate: &Value,
    output: &mut Vec<String>,
) {
    match (current, candidate) {
        (Value::Object(current), Value::Object(candidate)) => {
            let keys = current
                .keys()
                .chain(candidate.keys())
                .collect::<std::collections::BTreeSet<_>>();
            for key in keys {
                let path = prefix
                    .map(|prefix| format!("{prefix}.{key}"))
                    .unwrap_or_else(|| key.clone());
                match (current.get(key), candidate.get(key)) {
                    (Some(current), Some(candidate)) => {
                        collect_changed_paths(Some(&path), current, candidate, output)
                    }
                    _ => output.push(path),
                }
            }
        }
        _ if !json_semantically_equal(current, candidate) => {
            output.push(prefix.unwrap_or_default().to_owned())
        }
        _ => {}
    }
}

fn verify_requested_paths(
    kind: RecordKindV1,
    id: usize,
    candidate: &Value,
    readback: &Value,
    requested_paths: &[String],
) -> Result<(), String> {
    for path in requested_paths {
        let requested = dot_path(candidate, path);
        let actual = dot_path(readback, path);
        if !option_json_semantically_equal(requested, actual) {
            return Err(format!(
                "readback mismatch at {kind:?}/{id} path={path} requested={} actual={}",
                requested
                    .map(Value::to_string)
                    .unwrap_or_else(|| "<missing>".to_owned()),
                actual
                    .map(Value::to_string)
                    .unwrap_or_else(|| "<missing>".to_owned()),
            ));
        }
    }
    Ok(())
}

fn dot_path<'a>(value: &'a Value, path: &str) -> Option<&'a Value> {
    if path.is_empty() {
        return Some(value);
    }
    path.split('.').try_fold(value, |value, segment| {
        value.as_object().and_then(|object| object.get(segment))
    })
}

fn option_json_semantically_equal(left: Option<&Value>, right: Option<&Value>) -> bool {
    match (left, right) {
        (Some(left), Some(right)) => json_semantically_equal(left, right),
        (None, None) => true,
        _ => false,
    }
}

fn json_semantically_equal(left: &Value, right: &Value) -> bool {
    match (left, right) {
        (Value::Number(left), Value::Number(right)) => left
            .as_f64()
            .zip(right.as_f64())
            .is_some_and(|(left, right)| {
                let scale = left.abs().max(right.abs()).max(1.0);
                (left - right).abs() <= scale * 1e-6
            }),
        (Value::Array(left), Value::Array(right)) => {
            left.len() == right.len()
                && left
                    .iter()
                    .zip(right)
                    .all(|(left, right)| json_semantically_equal(left, right))
        }
        (Value::Object(left), Value::Object(right)) => {
            left.len() == right.len()
                && left.iter().all(|(key, left)| {
                    right
                        .get(key)
                        .is_some_and(|right| json_semantically_equal(left, right))
                })
        }
        _ => left == right,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tfm2_atlas_engine::{GameScope, TierPreset};

    #[test]
    fn persisted_profile_round_trips_without_match_data() {
        let profile = TierProfile {
            scope: GameScope::Tournament,
            preset: TierPreset::HardFearless,
            ..TierProfile::default()
        };
        let saved = serialize_saved_profile(&profile).unwrap();
        assert!(!saved.contains("champions"));
        assert!(!saved.contains("replays"));
        assert_eq!(deserialize_saved_profile(&saved).unwrap(), profile);
    }

    #[test]
    fn whole_record_writes_use_the_sdk_specialized_management_routes() {
        assert_eq!(
            whole_record_write_route(RecordKindV1::Team),
            WholeRecordWriteRoute::Team
        );
        assert_eq!(
            whole_record_write_route(RecordKindV1::Athlete),
            WholeRecordWriteRoute::Athlete
        );
        assert_eq!(
            whole_record_write_route(RecordKindV1::Staff),
            WholeRecordWriteRoute::Generic
        );
    }

    #[test]
    fn requested_path_readback_ignores_unrelated_host_normalization() {
        let candidate = json!({
            "name": "Edited",
            "age": 24,
            "stat": {"last_hit": 77},
            "host_managed": {"revision": 10}
        });
        let readback = json!({
            "name": "Edited",
            "age": 24.0,
            "stat": {"last_hit": 77.0},
            "host_managed": {"revision": 11, "normalized": true}
        });

        verify_requested_paths(
            RecordKindV1::Athlete,
            42,
            &candidate,
            &readback,
            &[
                "name".to_owned(),
                "age".to_owned(),
                "stat.last_hit".to_owned(),
            ],
        )
        .unwrap();
    }

    #[test]
    fn requested_path_readback_reports_only_the_actual_target_mismatch() {
        let candidate = json!({"stat": {"last_hit": 77}});
        let readback = json!({"stat": {"last_hit": 76}});

        let error = verify_requested_paths(
            RecordKindV1::Athlete,
            42,
            &candidate,
            &readback,
            &["stat.last_hit".to_owned()],
        )
        .unwrap_err();

        assert!(error.contains("Athlete/42"));
        assert!(error.contains("stat.last_hit"));
        assert!(error.contains("requested=77"));
        assert!(error.contains("actual=76"));
    }

    #[test]
    fn client_ui_sync_uses_the_live_0_5_5_detail_paths() {
        assert!(player_detail_value_paths("last_hit")
            .iter()
            .any(|path| path.ends_with("data.info.last_hit.value")));
        assert!(staff_detail_value_paths("banpick")
            .iter()
            .any(|path| path.ends_with("staff_detail.banpick.value")));
    }

    #[test]
    fn active_catalog_payload_is_exact_sorted_and_deduplicated() {
        let cassiopeia = hex_encode("wythm_cassiopeia");
        let archer = hex_encode("archer");
        let parsed = parse_active_champion_payload(&format!(
            "{cassiopeia},Top+Mid;{archer},Bottom;{cassiopeia},Top+Mid"
        ))
        .unwrap();

        assert_eq!(
            parsed,
            vec![
                ActiveChampionCatalogEntry {
                    champion_id: "archer".to_owned(),
                    positions: vec![RoleFilter::Bot],
                },
                ActiveChampionCatalogEntry {
                    champion_id: "wythm_cassiopeia".to_owned(),
                    positions: vec![RoleFilter::Top, RoleFilter::Mid],
                },
            ]
        );
        assert!(parse_active_champion_payload("not-hex").is_err());
        assert!(parse_active_champion_payload(&format!("{archer},Unknown")).is_err());
    }

    #[test]
    fn career_active_ids_are_the_only_catalog_members() {
        let metadata = BTreeMap::from([
            (
                "base_archer".to_owned(),
                ChampionBriefState {
                    champion_id: "base_archer".to_owned(),
                    display_name: "Archer".to_owned(),
                    category: None,
                    tags: Vec::new(),
                    positions: vec![RoleFilter::Bot],
                    main_position: Some(RoleFilter::Bot),
                    stat: champion_stat(mod_api_stable::StatV1::default()),
                    growth: champion_stat(mod_api_stable::StatV1::default()),
                },
            ),
            (
                "removed_mage".to_owned(),
                ChampionBriefState {
                    champion_id: "removed_mage".to_owned(),
                    display_name: "Removed".to_owned(),
                    category: None,
                    tags: Vec::new(),
                    positions: Vec::new(),
                    main_position: None,
                    stat: champion_stat(mod_api_stable::StatV1::default()),
                    growth: champion_stat(mod_api_stable::StatV1::default()),
                },
            ),
        ]);

        let catalog = catalog_for_active_ids(
            &[
                ActiveChampionCatalogEntry {
                    champion_id: "other_mod_dragon".to_owned(),
                    positions: vec![RoleFilter::Jungle, RoleFilter::Mid],
                },
                ActiveChampionCatalogEntry {
                    champion_id: "base_archer".to_owned(),
                    positions: vec![RoleFilter::Top, RoleFilter::Bot],
                },
            ],
            &metadata,
        );

        assert_eq!(
            catalog
                .iter()
                .map(|row| row.champion_id.as_str())
                .collect::<Vec<_>>(),
            vec!["base_archer", "other_mod_dragon"]
        );
        assert_eq!(catalog[0].positions, vec![RoleFilter::Top, RoleFilter::Bot]);
        assert_eq!(catalog[0].main_position, Some(RoleFilter::Top));
        assert_eq!(
            catalog[1].positions,
            vec![RoleFilter::Jungle, RoleFilter::Mid]
        );
    }

    #[test]
    fn draft_roles_use_ordered_game_positions_and_only_fallback_when_missing() {
        let catalog = BTreeMap::from([(
            "yone".to_owned(),
            ChampionBriefState {
                champion_id: "yone".to_owned(),
                display_name: "Yone".to_owned(),
                category: None,
                tags: Vec::new(),
                positions: vec![RoleFilter::Top, RoleFilter::Mid],
                main_position: Some(RoleFilter::Top),
                stat: champion_stat(mod_api_stable::StatV1::default()),
                growth: champion_stat(mod_api_stable::StatV1::default()),
            },
        )]);

        assert_eq!(
            game_roles("yone", &catalog),
            vec![RoleFilter::Top, RoleFilter::Mid]
        );
        assert_eq!(game_roles("unknown", &catalog), ALL_DRAFT_ROLES.to_vec());
    }

    #[test]
    fn invalid_persisted_profile_is_rejected() {
        let saved = r#"{"scope":"solo","division":"first"}"#;
        assert!(deserialize_saved_profile(saved).is_err());
    }

    #[test]
    fn tier_rows_include_champions_registered_by_other_mods() {
        let rows = build_tier_rows(["archer".to_owned()], None, ["other_mod_dragon".to_owned()]);
        assert_eq!(
            rows.iter()
                .map(|row| row.champion_id.as_str())
                .collect::<Vec<_>>(),
            vec!["archer", "other_mod_dragon"]
        );
        assert!(rows
            .iter()
            .all(|row| row.tier == Tier::NoTier && !row.eligible));
    }

    #[test]
    fn disconnect_clears_live_data_instead_of_publishing_stale_career_state() {
        let mut state = RuntimePublicState {
            connected: true,
            engine_status: EngineStatus::Ready,
            indexed_matches: 42,
            pending_records: 3,
            data_revision: 9,
            player_team_id: Some(7),
            champion_info: vec![ChampionBriefState {
                champion_id: "other_mod_dragon".to_owned(),
                display_name: "모드 드래곤".to_owned(),
                category: None,
                tags: Vec::new(),
                positions: vec![RoleFilter::Top],
                main_position: Some(RoleFilter::Top),
                stat: champion_stat(mod_api_stable::StatV1::default()),
                growth: champion_stat(mod_api_stable::StatV1::default()),
            }],
            ..RuntimePublicState::default()
        };

        state.mark_disconnected();

        assert!(!state.connected);
        assert_eq!(state.engine_status, EngineStatus::WaitingForCareer);
        assert_eq!(state.indexed_matches, 0);
        assert_eq!(state.pending_records, 0);
        assert_eq!(state.data_revision, 0);
        assert_eq!(state.player_team_id, None);
        assert!(state.analytics.is_none());
        assert!(state.locks.is_empty());
        assert!(state.champion_info.is_empty());
    }

    #[test]
    fn maximum_sized_frame_is_recognized_without_protocol_metadata() {
        assert!(is_frame_prefix(0));
        assert!(is_frame_prefix((MAX_FRAME_BYTES >> 24) as u8));
        assert!(!is_frame_prefix(b'P'));
    }

    #[test]
    fn timed_out_pending_request_cannot_execute_on_a_later_tick() {
        let execution = AtomicU8::new(REQUEST_PENDING);
        assert!(cancel_pending_request(&execution));
        assert!(!try_begin_request(&execution));
        assert_eq!(execution.load(Ordering::Acquire), REQUEST_CANCELLED);
    }

    #[test]
    fn timeout_cannot_cancel_a_request_that_already_started() {
        let execution = AtomicU8::new(REQUEST_PENDING);
        assert!(try_begin_request(&execution));
        assert!(!cancel_pending_request(&execution));
        assert_eq!(execution.load(Ordering::Acquire), REQUEST_PROCESSING);
    }

    #[test]
    fn large_routed_responses_are_published_behind_a_small_completion_event() {
        let response = BridgeResponse::Api(ApiResponse::ok(
            77,
            json!({ "champions": "x".repeat(256 * 1024) }),
        ));
        let mut published = BTreeMap::new();

        let payload = encode_routed_bridge_response(91, response, 64 * 1024, &mut published)
            .expect("large response should encode");
        let routed: RoutedBridgeResponse =
            serde_json::from_slice(&payload).expect("completion event should decode");

        assert!(payload.len() < 1024, "completion event must stay small");
        assert!(matches!(routed.response, BridgeResponse::Published));
        assert!(matches!(
            published.remove(&91),
            Some(BridgeResponse::Api(ApiResponse { request_id: 77, .. }))
        ));
    }

    #[test]
    fn schema_failure_blocks_every_structured_write_family() {
        assert!(api_command_mutates(ApiCommand::ApplyTierProfile));
        assert!(api_command_mutates(ApiCommand::SetLock));
        assert!(api_command_mutates(ApiCommand::Unlock));
        assert!(api_command_mutates(ApiCommand::ApplyEditorSettings));
        assert!(!api_command_mutates(ApiCommand::GetDashboard));
        assert!(!api_command_mutates(ApiCommand::GetEditorData));
    }

    #[test]
    fn queued_api_timeout_preserves_the_callers_request_id() {
        let bridge = Arc::new(BridgeHub::new());
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server_bridge = Arc::clone(&bridge);
        let server = thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            handle_stream(stream, &server_bridge, ApiSurface::Core);
        });

        let request = ApiRequest {
            request_id: 9_901,
            command: ApiCommand::GetDiagnostics,
            payload: Value::Null,
        };
        let mut client = TcpStream::connect(address).unwrap();
        client
            .write_all(&encode_json_frame(&request).unwrap())
            .unwrap();

        let mut header = [0u8; 4];
        client.read_exact(&mut header).unwrap();
        let mut frame = header.to_vec();
        let mut payload = vec![0u8; u32::from_be_bytes(header) as usize];
        client.read_exact(&mut payload).unwrap();
        frame.extend_from_slice(&payload);
        let response = decode_json_frame::<ApiResponse>(&frame).unwrap();

        assert_eq!(response.request_id, 9_901);
        assert!(!response.ok);
        server.join().unwrap();
    }

    #[test]
    fn champion_catalog_read_does_not_wait_for_a_management_tick() {
        let bridge = Arc::new(BridgeHub::new());
        bridge.champion_catalog.write().unwrap().status = CatalogStatus::Ready;
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server_bridge = Arc::clone(&bridge);
        let server = thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            handle_stream(stream, &server_bridge, ApiSurface::Core);
        });

        let request = ApiRequest {
            request_id: 9_902,
            command: ApiCommand::GetChampionCatalog,
            payload: Value::Null,
        };
        let mut client = TcpStream::connect(address).unwrap();
        client
            .write_all(&encode_json_frame(&request).unwrap())
            .unwrap();
        let mut header = [0u8; 4];
        client.read_exact(&mut header).unwrap();
        let mut frame = header.to_vec();
        let mut payload = vec![0u8; u32::from_be_bytes(header) as usize];
        client.read_exact(&mut payload).unwrap();
        frame.extend_from_slice(&payload);
        let response = decode_json_frame::<ApiResponse>(&frame).unwrap();

        assert_eq!(response.request_id, 9_902);
        assert!(response.ok);
        assert_eq!(response.data.as_ref().unwrap()["status"], "ready");
        server.join().unwrap();
    }
}
