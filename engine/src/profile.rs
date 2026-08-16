use serde::{Deserialize, Serialize};

use crate::ValidationError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GameScope {
    Tournament,
    Solo,
    SoloAndTournament,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RegionFilter {
    All,
    Kr,
    Cn,
    Eu,
    Na,
    Sa,
    Jp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DivisionFilter {
    All,
    First,
    Second,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RoleFilter {
    All,
    Top,
    Jungle,
    Mid,
    Bot,
    Support,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TierPreset {
    Classic,
    Fearless,
    HardFearless,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "mode", content = "games", rename_all = "snake_case")]
pub enum SampleMode {
    Auto,
    Minimum(u32),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct TierProfile {
    pub enabled: bool,
    pub scope: GameScope,
    pub region: RegionFilter,
    pub division: DivisionFilter,
    pub role: RoleFilter,
    pub patch: Option<String>,
    pub sample: SampleMode,
    pub preset: TierPreset,
}

impl Default for TierProfile {
    fn default() -> Self {
        Self {
            enabled: true,
            scope: GameScope::SoloAndTournament,
            region: RegionFilter::All,
            division: DivisionFilter::All,
            role: RoleFilter::All,
            patch: Some("latest".to_owned()),
            sample: SampleMode::Auto,
            preset: TierPreset::Classic,
        }
    }
}

impl TierProfile {
    pub fn validate(&self) -> Result<(), ValidationError> {
        if self.division != DivisionFilter::All && self.scope != GameScope::Tournament {
            return Err(ValidationError::DivisionRequiresTournament);
        }
        if matches!(self.sample, SampleMode::Minimum(0)) {
            return Err(ValidationError::InvalidSampleFloor);
        }
        if self
            .patch
            .as_ref()
            .is_some_and(|patch| patch.trim().is_empty())
        {
            return Err(ValidationError::InvalidPatch);
        }
        Ok(())
    }

    pub fn sample_floor(&self) -> usize {
        match self.sample {
            SampleMode::Auto => 5,
            SampleMode::Minimum(games) => games as usize,
        }
    }
}
