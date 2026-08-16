//! Shared protocol, statistics calculation, and lock domain for TFM2 Atlas.

mod draft;
mod locks;
mod analytics;
mod profile;
mod protocol;
mod record_patch;
mod tsv;

pub use locks::{LockGroup, LockSet, LockStatus};
pub use analytics::{ChampionInput, TierComponents, TierEngine, TierScore, RoleSample, Tier};
pub use profile::{
    DivisionFilter, GameScope, TierPreset, TierProfile, RegionFilter, RoleFilter, SampleMode,
};
pub use protocol::{decode_json_frame, encode_json_frame, FrameError, MAX_FRAME_BYTES};
pub use record_patch::{
    apply_lock_to_record, apply_tiers_to_team, RecordPatchError, PLAYER_STAT_FIELDS,
    STAFF_STAT_FIELDS,
};
pub use tsv::{parse_tier_tsv, render_tier_tsv_v2, TierPolicyRow, TsvError};

use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ValidationError {
    #[error("division filters are supported only for tournament scope")]
    DivisionRequiresTournament,
    #[error("the selected group has an invalid number of values")]
    InvalidGroupSize,
    #[error("a value is outside its supported range")]
    ValueOutOfRange,
    #[error("a required identifier or label is missing")]
    MissingIdentity,
    #[error("a lock contains an unsupported or incomplete value key")]
    InvalidValueKey,
    #[error("a custom sample floor must be at least one")]
    InvalidSampleFloor,
    #[error("patch must not be blank")]
    InvalidPatch,
}

impl ValidationError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::DivisionRequiresTournament => "division_requires_tournament",
            Self::InvalidGroupSize => "invalid_group_size",
            Self::ValueOutOfRange => "value_out_of_range",
            Self::MissingIdentity => "missing_identity",
            Self::InvalidValueKey => "invalid_value_key",
            Self::InvalidSampleFloor => "invalid_sample_floor",
            Self::InvalidPatch => "invalid_patch",
        }
    }
}
pub use draft::{
    analyze_role_gate, candidate_passes_role_gate, classify_role_profile, evaluate_draft_side,
    ChampionRoleProfile, ChampionRoleStat, DraftCandidateInput, DraftCandidateScore, DraftPhase,
    DraftRoleConfidence, DraftScoreComponents, DraftScoreWeights, DraftSettingsError,
    DraftSideEvaluation, RoleGateState,
};
