use std::{collections::BTreeSet, fmt, str::FromStr};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::Tier;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TierPolicyRow {
    pub champion_id: String,
    pub tier: Tier,
    pub overall: Option<f64>,
    pub eligible: bool,
}

impl TierPolicyRow {
    pub fn new(
        champion_id: impl Into<String>,
        tier: Tier,
        overall: Option<f64>,
        eligible: bool,
    ) -> Self {
        Self {
            champion_id: champion_id.into(),
            tier,
            overall,
            eligible,
        }
    }
}

#[derive(Debug, Error)]
pub enum TsvError {
    #[error("unsupported tier TSV header")]
    UnsupportedHeader,
    #[error("invalid tier TSV row {line}: {reason}")]
    InvalidRow { line: usize, reason: String },
}

pub fn render_tier_tsv_v2(rows: &[TierPolicyRow]) -> String {
    let mut output = String::from("champion_id\ttier\toverall\teligible\n");
    for row in rows {
        let overall = row
            .overall
            .map(|value| format!("{value:.1}"))
            .unwrap_or_default();
        output.push_str(&format!(
            "{}\t{}\t{}\t{}\n",
            row.champion_id, row.tier, overall, row.eligible
        ));
    }
    output
}

pub fn parse_tier_tsv(input: &str) -> Result<Vec<TierPolicyRow>, TsvError> {
    let mut lines = input.lines();
    let header = lines.next().unwrap_or_default().trim_end_matches('\r');
    let v2 = match header {
        "champion_id\ttier\toverall\teligible" => true,
        "champion_id\ttier\toverall" => false,
        _ => return Err(TsvError::UnsupportedHeader),
    };
    let mut rows = Vec::new();
    let mut champion_ids = BTreeSet::new();
    for (index, line) in lines.enumerate() {
        let line_number = index + 2;
        let line = line.trim_end_matches('\r');
        if line.trim().is_empty() {
            continue;
        }
        let fields: Vec<_> = line.split('\t').collect();
        let expected = if v2 { 4 } else { 3 };
        if fields.len() != expected || fields[0].trim().is_empty() {
            return Err(TsvError::InvalidRow {
                line: line_number,
                reason: format!("expected {expected} non-empty fields"),
            });
        }
        let champion_id = fields[0].trim();
        if !champion_ids.insert(champion_id.to_owned()) {
            return Err(TsvError::InvalidRow {
                line: line_number,
                reason: format!("duplicate champion_id: {champion_id}"),
            });
        }
        let tier = Tier::from_str(fields[1]).map_err(|reason| TsvError::InvalidRow {
            line: line_number,
            reason,
        })?;
        let overall = if fields[2].trim().is_empty() {
            None
        } else {
            let value = fields[2].parse::<f64>().map_err(|_| TsvError::InvalidRow {
                line: line_number,
                reason: "overall is not a number".to_owned(),
            })?;
            if !value.is_finite() || !(0.0..=100.0).contains(&value) {
                return Err(TsvError::InvalidRow {
                    line: line_number,
                    reason: "overall must be a finite number from 0 to 100".to_owned(),
                });
            }
            Some(value)
        };
        let eligible = if v2 {
            fields[3]
                .parse::<bool>()
                .map_err(|_| TsvError::InvalidRow {
                    line: line_number,
                    reason: "eligible must be true or false".to_owned(),
                })?
        } else {
            tier != Tier::NoTier
        };
        if eligible == (tier == Tier::NoTier) {
            return Err(TsvError::InvalidRow {
                line: line_number,
                reason: "eligible and tier disagree".to_owned(),
            });
        }
        rows.push(TierPolicyRow::new(champion_id, tier, overall, eligible));
    }
    Ok(rows)
}

impl fmt::Display for Tier {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Tier::Op => "OP",
            Tier::One => "1",
            Tier::Two => "2",
            Tier::Three => "3",
            Tier::Four => "4",
            Tier::NoTier => "-",
        })
    }
}

impl FromStr for Tier {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_uppercase().as_str() {
            "OP" | "S" => Ok(Self::Op),
            "1" | "A" => Ok(Self::One),
            "2" | "B" => Ok(Self::Two),
            "3" | "C" => Ok(Self::Three),
            "4" | "D" => Ok(Self::Four),
            "-" | "NO_TIER" | "NOTIER" => Ok(Self::NoTier),
            _ => Err(format!("unknown tier: {value}")),
        }
    }
}
