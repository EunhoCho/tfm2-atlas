use serde_json::Value;
use thiserror::Error;

use crate::{LockGroup, LockSet, Tier, TierPolicyRow, ValidationError};

pub const PLAYER_STAT_FIELDS: [&str; 12] = [
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

pub const STAFF_STAT_FIELDS: [&str; 10] = [
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

#[derive(Debug, Error, PartialEq)]
pub enum RecordPatchError {
    #[error("invalid lock: {0}")]
    InvalidLock(#[from] ValidationError),
    #[error("required JSON path is missing: {path}")]
    MissingPath { path: String },
    #[error("JSON path has the wrong value type: {path}")]
    InvalidType { path: String },
    #[error("target id does not match the record")]
    TargetMismatch,
    #[error("invalid tier policy: {reason}")]
    InvalidTierPolicy { reason: String },
}

pub fn apply_lock_to_record(record: &mut Value, lock: &LockSet) -> Result<(), RecordPatchError> {
    lock.validate()?;
    let record_id =
        record
            .get("id")
            .and_then(Value::as_u64)
            .ok_or_else(|| RecordPatchError::MissingPath {
                path: "id".to_owned(),
            })?;
    if record_id as usize != lock.target_id {
        return Err(RecordPatchError::TargetMismatch);
    }

    let mut candidate = record.clone();
    match lock.group {
        LockGroup::PlayerStats => {
            patch_numeric_fields(&mut candidate, "stat", &PLAYER_STAT_FIELDS, &lock.values)?;
        }
        LockGroup::StaffStats => {
            patch_numeric_fields(&mut candidate, "stat", &STAFF_STAT_FIELDS, &lock.values)?;
        }
        LockGroup::ChampionMastery => patch_mastery(&mut candidate, lock)?,
        LockGroup::PlayerProfile => patch_player_profile(&mut candidate, lock)?,
        LockGroup::StaffProfile => patch_staff_profile(&mut candidate, lock)?,
    }
    *record = candidate;
    Ok(())
}

fn patch_player_profile(record: &mut Value, lock: &LockSet) -> Result<(), RecordPatchError> {
    patch_keyed_profile(record, lock, true)
}

fn patch_staff_profile(record: &mut Value, lock: &LockSet) -> Result<(), RecordPatchError> {
    patch_keyed_profile(record, lock, false)
}

fn patch_keyed_profile(
    record: &mut Value,
    lock: &LockSet,
    player: bool,
) -> Result<(), RecordPatchError> {
    let mut language = serde_json::Map::new();
    for (key, value) in lock.value_keys.iter().zip(&lock.values) {
        if let Some(region) = key.strip_prefix(if player {
            "stat.language."
        } else {
            "language."
        }) {
            language.insert(region.to_owned(), integer_value(*value, key)?);
            continue;
        }
        if let Some(champion) = key.strip_prefix("champion_proficiency.") {
            patch_one_mastery(record, champion, *value)?;
            continue;
        }
        let path = key.split('.').collect::<Vec<_>>();
        replace_numeric_path(record, &path, *value, key)?;
    }
    let language_path = if player {
        vec!["stat", "language"]
    } else {
        vec!["language"]
    };
    replace_object_path(
        record,
        &language_path,
        Value::Object(language),
        &language_path.join("."),
    )
}

fn patch_one_mastery(
    record: &mut Value,
    champion_id: &str,
    value: f64,
) -> Result<(), RecordPatchError> {
    let lock = LockSet::new_mastery(
        record.get("id").and_then(Value::as_u64).unwrap_or_default() as usize,
        "profile",
        vec![(champion_id.to_owned(), value)],
    );
    patch_mastery(record, &lock)
}

fn replace_numeric_path(
    record: &mut Value,
    path: &[&str],
    value: f64,
    display_path: &str,
) -> Result<(), RecordPatchError> {
    let (last, parents) = path
        .split_last()
        .ok_or_else(|| RecordPatchError::MissingPath {
            path: display_path.to_owned(),
        })?;
    let mut current = record;
    for parent in parents {
        current = current
            .get_mut(*parent)
            .ok_or_else(|| RecordPatchError::MissingPath {
                path: display_path.to_owned(),
            })?;
    }
    let slot = current
        .get_mut(*last)
        .ok_or_else(|| RecordPatchError::MissingPath {
            path: display_path.to_owned(),
        })?;
    if !slot.is_number() {
        return Err(RecordPatchError::InvalidType {
            path: display_path.to_owned(),
        });
    }
    *slot = integer_value(value, display_path)?;
    Ok(())
}

fn replace_object_path(
    record: &mut Value,
    path: &[&str],
    value: Value,
    display_path: &str,
) -> Result<(), RecordPatchError> {
    let (last, parents) = path
        .split_last()
        .ok_or_else(|| RecordPatchError::MissingPath {
            path: display_path.to_owned(),
        })?;
    let mut current = record;
    for parent in parents {
        current = current
            .get_mut(*parent)
            .ok_or_else(|| RecordPatchError::MissingPath {
                path: display_path.to_owned(),
            })?;
    }
    let slot = current
        .get_mut(*last)
        .ok_or_else(|| RecordPatchError::MissingPath {
            path: display_path.to_owned(),
        })?;
    if !slot.is_object() {
        return Err(RecordPatchError::InvalidType {
            path: display_path.to_owned(),
        });
    }
    *slot = value;
    Ok(())
}

pub fn apply_tiers_to_team(
    team_record: &mut Value,
    rows: &[TierPolicyRow],
) -> Result<(), RecordPatchError> {
    let mut candidate = team_record.clone();
    let tiers = candidate
        .get_mut("champion_tiers")
        .ok_or_else(|| RecordPatchError::MissingPath {
            path: "champion_tiers".to_owned(),
        })?
        .as_object_mut()
        .ok_or_else(|| RecordPatchError::InvalidType {
            path: "champion_tiers".to_owned(),
        })?;

    for row in rows {
        if row.champion_id.trim().is_empty() {
            return Err(RecordPatchError::InvalidTierPolicy {
                reason: "champion_id is empty".to_owned(),
            });
        }
        if row.eligible == (row.tier == Tier::NoTier) {
            return Err(RecordPatchError::InvalidTierPolicy {
                reason: format!("eligible and tier disagree for {}", row.champion_id),
            });
        }
        let game_tier = match row.tier {
            Tier::Op => "S",
            Tier::One => "A",
            Tier::Two => "B",
            Tier::Three => "C",
            Tier::Four => "D",
            Tier::NoTier => "NoTier",
        };
        tiers.insert(row.champion_id.clone(), Value::String(game_tier.to_owned()));
    }
    *team_record = candidate;
    Ok(())
}

fn patch_numeric_fields<const N: usize>(
    record: &mut Value,
    parent: &str,
    fields: &[&str; N],
    values: &[f64],
) -> Result<(), RecordPatchError> {
    let object = record
        .get_mut(parent)
        .ok_or_else(|| RecordPatchError::MissingPath {
            path: parent.to_owned(),
        })?
        .as_object_mut()
        .ok_or_else(|| RecordPatchError::InvalidType {
            path: parent.to_owned(),
        })?;
    for (field, value) in fields.iter().zip(values) {
        let path = format!("{parent}.{field}");
        let slot = object
            .get_mut(*field)
            .ok_or_else(|| RecordPatchError::MissingPath { path: path.clone() })?;
        if !slot.is_number() {
            return Err(RecordPatchError::InvalidType { path });
        }
        *slot = integer_value(*value, &path)?;
    }
    Ok(())
}

fn patch_mastery(record: &mut Value, lock: &LockSet) -> Result<(), RecordPatchError> {
    let mastery = record
        .get_mut("champion_proficiency")
        .ok_or_else(|| RecordPatchError::MissingPath {
            path: "champion_proficiency".to_owned(),
        })?
        .as_object_mut()
        .ok_or_else(|| RecordPatchError::InvalidType {
            path: "champion_proficiency".to_owned(),
        })?;
    for (champion_id, value) in lock.value_keys.iter().zip(&lock.values) {
        let path = format!("champion_proficiency.{champion_id}.value");
        if !mastery.contains_key(champion_id) {
            mastery.insert(
                champion_id.clone(),
                serde_json::json!({"value": 0, "floor": 0}),
            );
        }
        let slot = mastery
            .get_mut(champion_id)
            .ok_or_else(|| RecordPatchError::MissingPath { path: path.clone() })?
            .as_object_mut()
            .ok_or_else(|| RecordPatchError::InvalidType { path: path.clone() })?
            .get_mut("value")
            .ok_or_else(|| RecordPatchError::MissingPath { path: path.clone() })?;
        if !slot.is_number() {
            return Err(RecordPatchError::InvalidType { path });
        }
        *slot = integer_value(*value * 10.0, &path)?;
    }
    Ok(())
}

fn integer_value(value: f64, path: &str) -> Result<Value, RecordPatchError> {
    if !value.is_finite()
        || value.fract() != 0.0
        || value < i64::MIN as f64
        || value > i64::MAX as f64
    {
        return Err(RecordPatchError::InvalidType {
            path: path.to_owned(),
        });
    }
    Ok(Value::from(value as i64))
}
