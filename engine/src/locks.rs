use serde::{Deserialize, Serialize};

use crate::ValidationError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LockGroup {
    PlayerStats,
    ChampionMastery,
    StaffStats,
    PlayerProfile,
    StaffProfile,
}

impl LockGroup {
    fn accepts_len(self, len: usize) -> bool {
        match self {
            Self::PlayerStats => len == 12,
            Self::ChampionMastery => (1..=512).contains(&len),
            Self::StaffStats => len == 10,
            Self::PlayerProfile => (22..=1050).contains(&len),
            Self::StaffProfile => (11..=522).contains(&len),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LockStatus {
    Active,
    Error,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LockSet {
    pub target_id: usize,
    pub target_name: String,
    pub group: LockGroup,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub value_keys: Vec<String>,
    pub values: Vec<f64>,
    pub status: LockStatus,
    pub error: Option<String>,
}

impl LockSet {
    pub fn new(
        target_id: usize,
        target_name: impl Into<String>,
        group: LockGroup,
        values: Vec<f64>,
    ) -> Self {
        Self {
            target_id,
            target_name: target_name.into(),
            group,
            value_keys: Vec::new(),
            values,
            status: LockStatus::Active,
            error: None,
        }
    }

    pub fn new_mastery(
        target_id: usize,
        target_name: impl Into<String>,
        values: Vec<(String, f64)>,
    ) -> Self {
        let (value_keys, values) = values.into_iter().unzip();
        Self {
            target_id,
            target_name: target_name.into(),
            group: LockGroup::ChampionMastery,
            value_keys,
            values,
            status: LockStatus::Active,
            error: None,
        }
    }

    pub fn new_keyed(
        target_id: usize,
        target_name: impl Into<String>,
        group: LockGroup,
        values: Vec<(String, f64)>,
    ) -> Self {
        let (value_keys, values) = values.into_iter().unzip();
        Self {
            target_id,
            target_name: target_name.into(),
            group,
            value_keys,
            values,
            status: LockStatus::Active,
            error: None,
        }
    }

    pub fn validate(&self) -> Result<(), ValidationError> {
        if self.target_name.trim().is_empty() {
            return Err(ValidationError::MissingIdentity);
        }
        if !self.group.accepts_len(self.values.len()) {
            return Err(ValidationError::InvalidGroupSize);
        }
        if self.values.iter().any(|value| !value.is_finite()) {
            return Err(ValidationError::ValueOutOfRange);
        }
        match self.group {
            LockGroup::ChampionMastery => {
                if self.value_keys.len() != self.values.len()
                    || self.value_keys.iter().any(|key| key.trim().is_empty())
                {
                    return Err(ValidationError::InvalidGroupSize);
                }
                if self
                    .values
                    .iter()
                    .any(|value| !(0.0..=100.0).contains(value))
                {
                    return Err(ValidationError::ValueOutOfRange);
                }
            }
            LockGroup::PlayerProfile => validate_player_profile(self)?,
            LockGroup::StaffProfile => validate_staff_profile(self)?,
            LockGroup::PlayerStats | LockGroup::StaffStats => {
                if !self.value_keys.is_empty() {
                    return Err(ValidationError::InvalidGroupSize);
                }
                if self
                    .values
                    .iter()
                    .any(|value| !(1.0..=100.0).contains(value))
                {
                    return Err(ValidationError::ValueOutOfRange);
                }
            }
        }
        Ok(())
    }
}

fn validate_player_profile(lock: &LockSet) -> Result<(), ValidationError> {
    use std::collections::BTreeSet;

    const REQUIRED: [&str; 22] = [
        "stat.last_hit",
        "stat.skill_avoid",
        "stat.skill_hit",
        "stat.control_speed",
        "stat.positioning",
        "stat.judgement",
        "stat.mental",
        "stat.concentration",
        "stat.order",
        "stat.roaming",
        "stat.aggressive",
        "stat.ego",
        "stat.top",
        "stat.jungle",
        "stat.mid",
        "stat.bottom",
        "stat.support",
        "hidden.potential",
        "management.stamina",
        "management.stress",
        "management.condition",
        "age",
    ];
    validate_keyed(lock)?;
    let keys = lock
        .value_keys
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    if REQUIRED.iter().any(|key| !keys.contains(key))
        || !keys
            .iter()
            .any(|key| key.starts_with("champion_proficiency."))
        || keys.iter().any(|key| {
            !REQUIRED.contains(key)
                && !valid_indexed_key(key, "stat.language.")
                && !valid_champion_key(key)
        })
    {
        return Err(ValidationError::InvalidValueKey);
    }
    for (key, value) in lock.value_keys.iter().zip(&lock.values) {
        let range = if key == "age" {
            16.0..=100.0
        } else if key.starts_with("stat.language.")
            || key.starts_with("champion_proficiency.")
            || matches!(
                key.as_str(),
                "stat.top"
                    | "stat.jungle"
                    | "stat.mid"
                    | "stat.bottom"
                    | "stat.support"
                    | "management.stamina"
                    | "management.stress"
                    | "management.condition"
            )
        {
            0.0..=100.0
        } else {
            1.0..=100.0
        };
        if !range.contains(value) {
            return Err(ValidationError::ValueOutOfRange);
        }
    }
    let active_positions = [
        "stat.top",
        "stat.jungle",
        "stat.mid",
        "stat.bottom",
        "stat.support",
    ]
    .iter()
    .filter(|key| value_for(lock, key).is_some_and(|value| value > 0.0))
    .count();
    if active_positions > 3 {
        return Err(ValidationError::ValueOutOfRange);
    }
    Ok(())
}

fn validate_staff_profile(lock: &LockSet) -> Result<(), ValidationError> {
    use std::collections::BTreeSet;

    const REQUIRED: [&str; 11] = [
        "stat.banpick",
        "stat.strategy",
        "stat.negotiation",
        "stat.judge_ability",
        "stat.judge_potential",
        "stat.feedback",
        "stat.power_analysis",
        "stat.control_coaching",
        "stat.judgment_coaching",
        "stat.mental_coaching",
        "age",
    ];
    validate_keyed(lock)?;
    let keys = lock
        .value_keys
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    if REQUIRED.iter().any(|key| !keys.contains(key))
        || keys
            .iter()
            .any(|key| !REQUIRED.contains(key) && !valid_indexed_key(key, "language."))
    {
        return Err(ValidationError::InvalidValueKey);
    }
    for (key, value) in lock.value_keys.iter().zip(&lock.values) {
        let range = if key == "age" {
            16.0..=100.0
        } else if key.starts_with("language.") {
            0.0..=100.0
        } else {
            1.0..=100.0
        };
        if !range.contains(value) {
            return Err(ValidationError::ValueOutOfRange);
        }
    }
    Ok(())
}

fn validate_keyed(lock: &LockSet) -> Result<(), ValidationError> {
    use std::collections::BTreeSet;
    if lock.value_keys.len() != lock.values.len()
        || lock.value_keys.iter().any(|key| key.trim().is_empty())
        || lock.value_keys.iter().collect::<BTreeSet<_>>().len() != lock.value_keys.len()
    {
        return Err(ValidationError::InvalidGroupSize);
    }
    Ok(())
}

fn valid_indexed_key(key: &str, prefix: &str) -> bool {
    key.strip_prefix(prefix).is_some_and(|suffix| {
        !suffix.is_empty() && suffix.chars().all(|value| value.is_ascii_digit())
    })
}

fn valid_champion_key(key: &str) -> bool {
    key.strip_prefix("champion_proficiency.")
        .is_some_and(|champion| {
            !champion.is_empty()
                && champion
                    .chars()
                    .all(|value| value.is_ascii_alphanumeric() || value == '_')
        })
}

fn value_for(lock: &LockSet, key: &str) -> Option<f64> {
    lock.value_keys
        .iter()
        .position(|candidate| candidate == key)
        .map(|index| lock.values[index])
}
