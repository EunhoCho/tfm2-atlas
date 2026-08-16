use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tfm2_atlas_engine::{DraftCandidateScore, DraftPhase, DraftScoreWeights, RoleFilter};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ApiCommand {
    #[serde(rename = "SUBSCRIBE_STATE")]
    SubscribeState,
    #[serde(rename = "GET_DASHBOARD")]
    GetDashboard,
    #[serde(rename = "PREVIEW_TIER_PROFILE")]
    PreviewTierProfile,
    #[serde(rename = "APPLY_TIER_PROFILE")]
    ApplyTierProfile,
    GetLocks,
    SetLock,
    Unlock,
    GetDiagnostics,
    ExportTierTsv,
    ValidateTierTsv,
    #[serde(rename = "GET_CATALOG")]
    GetChampionCatalog,
    #[serde(rename = "GET_EDITOR_DATA")]
    GetEditorData,
    ApplyEditorSettings,
    GetPlayerMastery,
    SetPlayerMastery,
    ApplyPlayerEdit,
    ApplyStaffEdit,
    MovePlayer,
    SetDraftSettings,
    GetMockDraft,
    SetMockDraftContext,
    ApplyMockDraftAction,
    RemoveMockDraftAction,
    UndoMockDraft,
    ResetMockDraftSet,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApiSurface {
    Core,
    Editor,
}

impl ApiSurface {
    pub fn allows(self, command: ApiCommand) -> bool {
        match self {
            Self::Core => matches!(
                command,
                ApiCommand::SubscribeState
                    | ApiCommand::GetDashboard
                    | ApiCommand::PreviewTierProfile
                    | ApiCommand::ApplyTierProfile
                    | ApiCommand::GetDiagnostics
                    | ApiCommand::ExportTierTsv
                    | ApiCommand::ValidateTierTsv
                    | ApiCommand::GetChampionCatalog
                    | ApiCommand::SetDraftSettings
                    | ApiCommand::GetMockDraft
                    | ApiCommand::SetMockDraftContext
                    | ApiCommand::ApplyMockDraftAction
                    | ApiCommand::RemoveMockDraftAction
                    | ApiCommand::UndoMockDraft
                    | ApiCommand::ResetMockDraftSet
            ),
            Self::Editor => matches!(
                command,
                ApiCommand::SubscribeState
                    | ApiCommand::GetLocks
                    | ApiCommand::SetLock
                    | ApiCommand::Unlock
                    | ApiCommand::GetEditorData
                    | ApiCommand::ApplyEditorSettings
                    | ApiCommand::GetPlayerMastery
                    | ApiCommand::SetPlayerMastery
                    | ApiCommand::ApplyPlayerEdit
                    | ApiCommand::ApplyStaffEdit
                    | ApiCommand::MovePlayer
            ),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayerContractEdit {
    pub team_id: usize,
    pub start_date: String,
    pub end_date: String,
    pub annual_salary: f64,
    pub transfer_fee: f64,
    pub squad_status: String,
    #[serde(default)]
    pub incentives: Vec<Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StaffContractEdit {
    pub team_id: usize,
    pub start_date: String,
    pub end_date: String,
    pub annual_salary: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MasteryEdit {
    pub champion_id: String,
    pub value: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayerEditRequest {
    pub athlete_id: usize,
    pub name: Option<String>,
    pub age: Option<i64>,
    #[serde(default)]
    pub stats: BTreeMap<String, i64>,
    #[serde(default)]
    pub positions: BTreeMap<String, i64>,
    pub potential: Option<i64>,
    pub stamina: Option<i64>,
    pub stress: Option<i64>,
    pub condition: Option<i64>,
    pub annual_salary: Option<f64>,
    #[serde(default)]
    pub communication: Option<BTreeMap<usize, i64>>,
    #[serde(default)]
    pub mastery: Vec<MasteryEdit>,
    pub contract: Option<PlayerContractEdit>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StaffEditRequest {
    pub staff_id: usize,
    pub name: Option<String>,
    pub age: Option<i64>,
    #[serde(default)]
    pub stats: BTreeMap<String, i64>,
    pub annual_salary: Option<f64>,
    #[serde(default)]
    pub communication: Option<BTreeMap<usize, i64>>,
    pub contract: Option<StaffContractEdit>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayerEditReadback {
    pub target_id: usize,
    pub changed_paths: Vec<String>,
    pub record: Value,
    pub client_screen_verified: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MovePlayerReadback {
    pub athlete_id: usize,
    pub team_id: usize,
    pub record: Value,
    pub client_screen_verified: bool,
    pub status: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DraftRule {
    Classic,
    Fearless,
    #[serde(rename = "fearless_hard")]
    HardFearless,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DraftSide {
    Blue,
    Red,
}

impl Default for DraftSide {
    fn default() -> Self {
        Self::Blue
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MockDraftSet {
    pub set_number: usize,
    pub our_side: DraftSide,
    pub blue_bans: Vec<String>,
    pub red_bans: Vec<String>,
    pub blue_picks: Vec<String>,
    pub red_picks: Vec<String>,
}

impl MockDraftSet {
    pub fn empty(set_number: usize) -> Self {
        Self {
            set_number,
            our_side: DraftSide::Blue,
            blue_bans: Vec::new(),
            red_bans: Vec::new(),
            blue_picks: Vec::new(),
            red_picks: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MockDraftAction {
    pub side: DraftSide,
    pub phase: DraftPhase,
    pub champion_id: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MockDraftContextUpdate {
    pub opponent_team_id: Option<usize>,
    pub player_side: Option<DraftSide>,
    pub rule: Option<DraftRule>,
    pub current_set: Option<usize>,
    pub recommendation_phase: Option<DraftPhase>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DraftEvaluation {
    pub blue: Option<tfm2_atlas_engine::DraftSideEvaluation>,
    pub red: Option<tfm2_atlas_engine::DraftSideEvaluation>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MockDraftState {
    pub opponent_team_id: Option<usize>,
    pub rule: DraftRule,
    pub current_set: usize,
    pub sets: Vec<MockDraftSet>,
    pub last_own_phase: DraftPhase,
    pub team_options: Vec<DraftTeamOption>,
    pub available_champions: Vec<String>,
    pub excluded_blue: Vec<String>,
    pub excluded_red: Vec<String>,
    pub settings: DraftScoreWeights,
    pub recommendation: Option<DraftRecommendation>,
    pub evaluation: DraftEvaluation,
    pub blue_gate: Option<tfm2_atlas_engine::RoleGateState>,
    pub red_gate: Option<tfm2_atlas_engine::RoleGateState>,
}

impl Default for MockDraftState {
    fn default() -> Self {
        Self {
            opponent_team_id: None,
            rule: DraftRule::Classic,
            current_set: 1,
            sets: (1..=5).map(MockDraftSet::empty).collect(),
            last_own_phase: DraftPhase::Ban,
            team_options: Vec::new(),
            available_champions: Vec::new(),
            excluded_blue: Vec::new(),
            excluded_red: Vec::new(),
            settings: DraftScoreWeights::default(),
            recommendation: None,
            evaluation: DraftEvaluation {
                blue: None,
                red: None,
            },
            blue_gate: None,
            red_gate: None,
        }
    }
}

impl MockDraftState {
    pub fn current_our_side(&self) -> DraftSide {
        self.sets
            .iter()
            .find(|set| set.set_number == self.current_set)
            .map(|set| set.our_side)
            .unwrap_or_default()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DraftTeamOption {
    pub team_id: usize,
    pub team_name: String,
    pub player_team: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct DraftComposition {
    pub ad: f64,
    pub ap: f64,
    pub tank: f64,
    pub utility: f64,
    pub cc: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DraftRecommendation {
    pub phase: DraftPhase,
    pub candidates: Vec<DraftCandidateScore>,
    pub projected_ally: Vec<String>,
    pub projected_enemy: Vec<String>,
    pub ally_remaining_roles: Vec<RoleFilter>,
    pub enemy_remaining_roles: Vec<RoleFilter>,
    pub ally_composition: DraftComposition,
    pub enemy_composition: DraftComposition,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApiRequest {
    pub request_id: u64,
    pub command: ApiCommand,
    #[serde(default)]
    pub payload: Value,
}

impl ApiRequest {
    pub fn validate(&self) -> Result<(), ApiError> {
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiError {
    pub code: String,
    pub message: String,
}

impl ApiError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApiResponse {
    pub request_id: u64,
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ApiError>,
}

impl ApiResponse {
    pub fn ok(request_id: u64, data: Value) -> Self {
        Self {
            request_id,
            ok: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn error(request_id: u64, code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            request_id,
            ok: false,
            data: None,
            error: Some(ApiError::new(code, message)),
        }
    }
}
