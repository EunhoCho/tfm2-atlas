use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::{RoleFilter, RoleSample};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DraftPhase {
    Ban,
    Pick,
    Waiting,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DraftRoleConfidence {
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChampionRoleStat {
    pub role: RoleFilter,
    pub matches: usize,
    pub wins: usize,
    pub share: f64,
    pub win_rate: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChampionRoleProfile {
    pub champion_id: String,
    pub roles: Vec<ChampionRoleStat>,
    pub primary_roles: Vec<RoleFilter>,
    pub total_matches: usize,
    pub required_matches: usize,
    pub sufficient: bool,
    pub used_patches: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoleGateState {
    pub feasible: bool,
    pub definitely_filled: Vec<RoleFilter>,
    pub assignment_count: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DraftSideEvaluation {
    pub total: f64,
    pub op: f64,
    pub synergy: f64,
    pub matchup: f64,
    pub data_coverage: f64,
}

const PLAYABLE_ROLES: [RoleFilter; 5] = [
    RoleFilter::Top,
    RoleFilter::Jungle,
    RoleFilter::Mid,
    RoleFilter::Bot,
    RoleFilter::Support,
];

pub fn classify_role_profile(
    champion_id: impl Into<String>,
    samples: &BTreeMap<RoleFilter, RoleSample>,
    required_matches: usize,
    used_patches: Vec<String>,
) -> ChampionRoleProfile {
    let total_matches = PLAYABLE_ROLES
        .iter()
        .map(|role| samples.get(role).map(|sample| sample.matches).unwrap_or(0))
        .sum::<usize>();
    let mut roles = PLAYABLE_ROLES
        .into_iter()
        .map(|role| {
            let sample = samples.get(&role).cloned().unwrap_or_default();
            ChampionRoleStat {
                role,
                matches: sample.matches,
                wins: sample.wins,
                share: if total_matches == 0 {
                    0.0
                } else {
                    round1(sample.matches as f64 / total_matches as f64 * 100.0)
                },
                win_rate: (sample.matches > 0)
                    .then(|| round1(sample.wins as f64 / sample.matches as f64 * 100.0)),
            }
        })
        .collect::<Vec<_>>();
    roles.sort_by(|left, right| {
        right
            .share
            .total_cmp(&left.share)
            .then(role_order(left.role).cmp(&role_order(right.role)))
    });

    let primary_roles = if total_matches == 0 {
        PLAYABLE_ROLES.to_vec()
    } else if roles.first().is_some_and(|row| row.share >= 75.0) {
        vec![roles[0].role]
    } else {
        let forty = roles
            .iter()
            .filter(|row| row.share >= 40.0)
            .take(2)
            .map(|row| row.role)
            .collect::<Vec<_>>();
        if forty.len() == 2 {
            forty
        } else {
            roles
                .iter()
                .filter(|row| row.matches > 0)
                .take(3)
                .map(|row| row.role)
                .collect()
        }
    };

    ChampionRoleProfile {
        champion_id: champion_id.into(),
        roles,
        primary_roles,
        total_matches,
        required_matches,
        sufficient: total_matches >= required_matches.max(1),
        used_patches,
    }
}

pub fn analyze_role_gate(picks: &[Vec<RoleFilter>]) -> RoleGateState {
    let assignments = role_assignments(picks);
    let feasible = !assignments.is_empty();
    let definitely_filled = if let Some(first) = assignments.first() {
        let mut intersection = first.iter().copied().collect::<BTreeSet<_>>();
        for assignment in assignments.iter().skip(1) {
            let occupied = assignment.iter().copied().collect::<BTreeSet<_>>();
            intersection.retain(|role| occupied.contains(role));
        }
        PLAYABLE_ROLES
            .into_iter()
            .filter(|role| intersection.contains(role))
            .collect()
    } else {
        Vec::new()
    };
    RoleGateState {
        feasible,
        definitely_filled,
        assignment_count: assignments.len(),
    }
}

pub fn candidate_passes_role_gate(
    current_picks: &[Vec<RoleFilter>],
    candidate_roles: &[RoleFilter],
) -> bool {
    if !analyze_role_gate(current_picks).feasible {
        return true;
    }
    let mut augmented = current_picks.to_vec();
    augmented.push(candidate_roles.to_vec());
    analyze_role_gate(&augmented).feasible
}

fn role_assignments(picks: &[Vec<RoleFilter>]) -> Vec<Vec<RoleFilter>> {
    fn visit(
        picks: &[Vec<RoleFilter>],
        index: usize,
        used: &mut BTreeSet<RoleFilter>,
        current: &mut Vec<RoleFilter>,
        output: &mut Vec<Vec<RoleFilter>>,
    ) {
        if index == picks.len() {
            output.push(current.clone());
            return;
        }
        for role in PLAYABLE_ROLES {
            if picks[index].contains(&role) && used.insert(role) {
                current.push(role);
                visit(picks, index + 1, used, current, output);
                current.pop();
                used.remove(&role);
            }
        }
    }

    let mut output = Vec::new();
    visit(
        picks,
        0,
        &mut BTreeSet::new(),
        &mut Vec::new(),
        &mut output,
    );
    output
}

pub fn evaluate_draft_side(
    op_scores: &[f64],
    synergy_scores: &[f64],
    matchup_scores: &[f64],
    expected_synergy_pairs: usize,
    expected_matchup_pairs: usize,
) -> Option<DraftSideEvaluation> {
    if op_scores.is_empty() {
        return None;
    }
    let op = average_or_neutral(op_scores);
    let synergy = average_or_neutral(synergy_scores);
    let matchup = average_or_neutral(matchup_scores);
    let expected = op_scores.len() + expected_synergy_pairs + expected_matchup_pairs;
    let observed = op_scores.len() + synergy_scores.len() + matchup_scores.len();
    let data_coverage = if expected == 0 {
        0.0
    } else {
        round1(observed as f64 / expected as f64 * 100.0)
    };
    Some(DraftSideEvaluation {
        total: round1(op * 0.50 + synergy * 0.25 + matchup * 0.25),
        op,
        synergy,
        matchup,
        data_coverage,
    })
}

fn average_or_neutral(values: &[f64]) -> f64 {
    if values.is_empty() {
        50.0
    } else {
        round1(
            values
                .iter()
                .map(|value| value.clamp(0.0, 100.0))
                .sum::<f64>()
                / values.len() as f64,
        )
    }
}

fn role_order(role: RoleFilter) -> usize {
    PLAYABLE_ROLES
        .iter()
        .position(|candidate| *candidate == role)
        .unwrap_or(PLAYABLE_ROLES.len())
}

fn round1(value: f64) -> f64 {
    (value * 10.0).round() / 10.0
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DraftCandidateInput {
    pub champion_id: String,
    pub op_score: f64,
    pub matchup_score: f64,
    pub synergy_score: f64,
    pub composition_score: f64,
    pub opponent_preference_score: f64,
    pub ally_threat_score: f64,
    pub projected_enemy_synergy_score: f64,
    pub projected_enemy_composition_score: f64,
    pub usable_roles: Vec<RoleFilter>,
    pub role_confidence: DraftRoleConfidence,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct DraftScoreComponents {
    pub op: Option<f64>,
    pub matchup: Option<f64>,
    pub synergy: Option<f64>,
    pub composition: Option<f64>,
    pub opponent_preference: Option<f64>,
    pub ally_threat: Option<f64>,
    pub projected_enemy_synergy: Option<f64>,
    pub projected_enemy_composition: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DraftCandidateScore {
    pub champion_id: String,
    pub total: f64,
    pub recommended_role: Option<RoleFilter>,
    pub low_confidence: bool,
    pub components: DraftScoreComponents,
    pub reason: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DraftSettingsError {
    PickWeightsAreZero,
    BanWeightsAreZero,
    WeightOutOfRange,
}

impl std::fmt::Display for DraftSettingsError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::PickWeightsAreZero => "at least one pick weight must be greater than zero",
            Self::BanWeightsAreZero => "at least one enabled ban weight must be greater than zero",
            Self::WeightOutOfRange => "draft weights must be in the 0-100 range",
        })
    }
}

impl std::error::Error for DraftSettingsError {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DraftScoreWeights {
    pub pick_op: u8,
    pub pick_matchup: u8,
    pub pick_synergy: u8,
    pub pick_composition: u8,
    pub pick_denial: u8,
    pub pick_role_gate: bool,
    pub ban_preference: u8,
    pub ban_op: u8,
    pub ban_threat_enabled: bool,
    pub ban_threat: u8,
    pub ban_synergy_enabled: bool,
    pub ban_synergy: u8,
    pub ban_composition_enabled: bool,
    pub ban_composition: u8,
    pub ban_role_gate: bool,
}

impl Default for DraftScoreWeights {
    fn default() -> Self {
        Self {
            pick_op: 40,
            pick_matchup: 20,
            pick_synergy: 20,
            pick_composition: 10,
            pick_denial: 10,
            pick_role_gate: true,
            ban_preference: 40,
            ban_op: 60,
            ban_threat_enabled: false,
            ban_threat: 10,
            ban_synergy_enabled: false,
            ban_synergy: 10,
            ban_composition_enabled: false,
            ban_composition: 10,
            ban_role_gate: false,
        }
    }
}

impl DraftScoreWeights {
    pub fn validate(&self) -> Result<(), DraftSettingsError> {
        let all = [
            self.pick_op,
            self.pick_matchup,
            self.pick_synergy,
            self.pick_composition,
            self.pick_denial,
            self.ban_preference,
            self.ban_op,
            self.ban_threat,
            self.ban_synergy,
            self.ban_composition,
        ];
        if all.into_iter().any(|value| value > 100) {
            return Err(DraftSettingsError::WeightOutOfRange);
        }
        self.normalized_pick()?;
        self.normalized_ban()?;
        Ok(())
    }

    pub fn normalized_pick(&self) -> Result<[f64; 5], DraftSettingsError> {
        normalize([
            self.pick_op,
            self.pick_matchup,
            self.pick_synergy,
            self.pick_composition,
            self.pick_denial,
        ])
        .ok_or(DraftSettingsError::PickWeightsAreZero)
    }

    pub fn normalized_ban(&self) -> Result<[f64; 5], DraftSettingsError> {
        normalize([
            self.ban_preference,
            self.ban_op,
            self.ban_threat_enabled
                .then_some(self.ban_threat)
                .unwrap_or(0),
            self.ban_synergy_enabled
                .then_some(self.ban_synergy)
                .unwrap_or(0),
            self.ban_composition_enabled
                .then_some(self.ban_composition)
                .unwrap_or(0),
        ])
        .ok_or(DraftSettingsError::BanWeightsAreZero)
    }

    pub fn score_candidate(
        &self,
        phase: DraftPhase,
        input: &DraftCandidateInput,
        remaining_roles: &[RoleFilter],
    ) -> Option<DraftCandidateScore> {
        let gate_enabled = match phase {
            DraftPhase::Pick => self.pick_role_gate,
            DraftPhase::Ban => self.ban_role_gate,
            DraftPhase::Waiting => return None,
        };
        let recommended_role = remaining_roles
            .iter()
            .copied()
            .find(|role| input.usable_roles.contains(role))
            .or_else(|| input.usable_roles.first().copied());
        if gate_enabled
            && input.role_confidence != DraftRoleConfidence::Low
            && !remaining_roles.is_empty()
            && !remaining_roles
                .iter()
                .any(|role| input.usable_roles.contains(role))
        {
            return None;
        }

        let (total, components, reason) = match phase {
            DraftPhase::Pick => {
                let weights = self.normalized_pick().ok()?;
                (
                    weighted(
                        weights,
                        [
                            input.op_score,
                            input.matchup_score,
                            input.synergy_score,
                            input.composition_score,
                            input.opponent_preference_score,
                        ],
                    ),
                    DraftScoreComponents {
                        op: Some(input.op_score),
                        matchup: Some(input.matchup_score),
                        synergy: Some(input.synergy_score),
                        composition: Some(input.composition_score),
                        opponent_preference: Some(input.opponent_preference_score),
                        ..DraftScoreComponents::default()
                    },
                    String::new(),
                )
            }
            DraftPhase::Ban => {
                let weights = self.normalized_ban().ok()?;
                (
                    weighted(
                        weights,
                        [
                            input.opponent_preference_score,
                            input.op_score,
                            input.ally_threat_score,
                            input.projected_enemy_synergy_score,
                            input.projected_enemy_composition_score,
                        ],
                    ),
                    DraftScoreComponents {
                        op: Some(input.op_score),
                        opponent_preference: Some(input.opponent_preference_score),
                        ally_threat: self.ban_threat_enabled.then_some(input.ally_threat_score),
                        projected_enemy_synergy: self
                            .ban_synergy_enabled
                            .then_some(input.projected_enemy_synergy_score),
                        projected_enemy_composition: self
                            .ban_composition_enabled
                            .then_some(input.projected_enemy_composition_score),
                        ..DraftScoreComponents::default()
                    },
                    "ban_weighted_score".to_owned(),
                )
            }
            DraftPhase::Waiting => unreachable!(),
        };

        Some(DraftCandidateScore {
            champion_id: input.champion_id.clone(),
            total,
            recommended_role,
            low_confidence: input.role_confidence == DraftRoleConfidence::Low,
            components,
            reason,
        })
    }
}

fn normalize(values: [u8; 5]) -> Option<[f64; 5]> {
    let total = values.iter().map(|value| f64::from(*value)).sum::<f64>();
    (total > 0.0).then(|| values.map(|value| f64::from(value) / total))
}

fn weighted(weights: [f64; 5], values: [f64; 5]) -> f64 {
    weights
        .into_iter()
        .zip(values)
        .map(|(weight, value)| weight * value.clamp(0.0, 100.0))
        .sum()
}
