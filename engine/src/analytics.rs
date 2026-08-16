use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::{TierPreset, RoleFilter};

const MODEL_VERSION: &str = "role-aware-v6";
const MIN_PICKS: usize = 5;
const PRIOR_PICKS: f64 = 24.0;
const ROLE_PRIOR_PICKS: f64 = 16.0;
const ROLE_MIN_PICKS: usize = 5;
const USABLE_ROLE_MIN_PICKS: usize = 10;
const USABLE_ROLE_STRONG_MIN_PICKS: usize = 5;
const RELIABILITY_PICKS: f64 = 40.0;
const WIN_RISK_Z: f64 = 0.65;
const POWER_WEIGHT: f64 = 0.70;
const DRAFT_WEIGHT: f64 = 0.20;
const VERSATILITY_WEIGHT: f64 = 0.10;
const TOURNAMENT_DRAFT_WEIGHT: f64 = 0.75;
const ALL_PICK_DRAFT_WEIGHT: f64 = 0.25;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Tier {
    Op,
    One,
    Two,
    Three,
    Four,
    NoTier,
}

impl Tier {
    pub fn from_score(score: f64) -> Self {
        if score >= 85.0 {
            Self::Op
        } else if score >= 70.0 {
            Self::One
        } else if score >= 55.0 {
            Self::Two
        } else if score >= 40.0 {
            Self::Three
        } else {
            Self::Four
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct RoleSample {
    pub matches: usize,
    pub wins: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChampionInput {
    pub champion_id: String,
    pub pick_count: usize,
    pub wins: usize,
    pub ban_count: usize,
    pub pick_rate: Option<f64>,
    pub ban_rate: Option<f64>,
    pub tournament_presence_rate: Option<f64>,
    #[serde(default)]
    pub by_position: BTreeMap<RoleFilter, RoleSample>,
}

impl ChampionInput {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        champion_id: impl Into<String>,
        pick_count: usize,
        wins: usize,
        ban_count: usize,
        pick_rate: Option<f64>,
        ban_rate: Option<f64>,
        tournament_presence_rate: Option<f64>,
    ) -> Self {
        Self {
            champion_id: champion_id.into(),
            pick_count,
            wins,
            ban_count,
            pick_rate,
            ban_rate,
            tournament_presence_rate,
            by_position: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TierComponents {
    pub power: f64,
    pub draft: f64,
    pub versatility: f64,
    pub reliability: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TierScore {
    pub eligible: bool,
    pub overall: Option<f64>,
    pub tier: Tier,
    pub sample: usize,
    pub min_sample: usize,
    pub reason: Option<String>,
    pub formula_version: String,
    pub components: Option<TierComponents>,
    pub best_role: Option<RoleFilter>,
    pub usable_roles: Vec<RoleFilter>,
}

#[derive(Debug, Clone, Copy)]
pub struct TierEngine {
    min_picks: usize,
    power_weight: f64,
    draft_weight: f64,
    versatility_weight: f64,
}

impl TierEngine {
    pub fn role_aware_v6() -> Self {
        Self {
            min_picks: MIN_PICKS,
            power_weight: POWER_WEIGHT,
            draft_weight: DRAFT_WEIGHT,
            versatility_weight: VERSATILITY_WEIGHT,
        }
    }

    pub fn role_aware_v6_for_preset(preset: TierPreset) -> Self {
        let mut engine = Self::role_aware_v6();
        (
            engine.power_weight,
            engine.draft_weight,
            engine.versatility_weight,
        ) = match preset {
            TierPreset::Classic => (0.70, 0.20, 0.10),
            TierPreset::Fearless => (0.60, 0.15, 0.25),
            TierPreset::HardFearless => (0.50, 0.10, 0.40),
        };
        engine
    }

    pub fn with_min_picks(mut self, min_picks: usize) -> Self {
        self.min_picks = min_picks.max(1);
        self
    }

    pub fn score(&self, champions: &[ChampionInput]) -> BTreeMap<String, TierScore> {
        let eligible: BTreeSet<_> = champions
            .iter()
            .filter(|champion| champion.pick_count >= self.min_picks)
            .map(|champion| champion.champion_id.as_str())
            .collect();

        let mut role_rows = BTreeMap::<String, Vec<RolePower>>::new();
        let mut role_values = BTreeMap::<RoleFilter, Vec<f64>>::new();
        let mut best_power = BTreeMap::<String, f64>::new();
        let mut best_role = BTreeMap::<String, RoleFilter>::new();
        for champion in champions
            .iter()
            .filter(|row| eligible.contains(row.champion_id.as_str()))
        {
            let mut rows = role_power_rows(champion);
            if rows.is_empty() {
                rows.push(overall_power_row(champion));
            }
            for row in rows.iter().filter(|row| row.position != RoleFilter::All) {
                role_values
                    .entry(row.position)
                    .or_default()
                    .push(row.power_raw);
            }
            let deployable: Vec<_> = rows
                .iter()
                .filter(|row| row.sample >= ROLE_MIN_PICKS)
                .collect();
            let candidates: Vec<_> = if deployable.is_empty() {
                rows.iter().collect()
            } else {
                deployable
            };
            let best = candidates
                .into_iter()
                .max_by(|left, right| {
                    left.power_raw
                        .total_cmp(&right.power_raw)
                        .then(left.sample.cmp(&right.sample))
                })
                .expect("eligible champion has an overall or role row");
            best_power.insert(champion.champion_id.clone(), best.power_raw);
            best_role.insert(champion.champion_id.clone(), best.position);
            role_rows.insert(champion.champion_id.clone(), rows);
        }

        let power_values: Vec<_> = best_power.values().copied().collect();
        let mut presence_logits = BTreeMap::new();
        let mut pick_logits = BTreeMap::new();
        for champion in champions {
            if champion.pick_count == 0 && champion.ban_count == 0 {
                continue;
            }
            let presence = champion.tournament_presence_rate.or_else(|| {
                (champion.ban_count > 0)
                    .then_some(champion.ban_rate)
                    .flatten()
            });
            if let Some(rate) = presence.and_then(logit_rate) {
                presence_logits.insert(champion.champion_id.clone(), rate);
            }
            if let Some(rate) = champion.pick_rate.and_then(logit_rate) {
                pick_logits.insert(champion.champion_id.clone(), rate);
            }
        }
        let presence_z = z_scores(&presence_logits);
        let pick_z = z_scores(&pick_logits);
        let keys: BTreeSet<_> = presence_z.keys().chain(pick_z.keys()).cloned().collect();
        let draft_signal: BTreeMap<_, _> = keys
            .into_iter()
            .map(|key| {
                let value = if let Some(presence) = presence_z.get(&key) {
                    TOURNAMENT_DRAFT_WEIGHT * presence
                        + ALL_PICK_DRAFT_WEIGHT * pick_z.get(&key).copied().unwrap_or_default()
                } else {
                    pick_z.get(&key).copied().unwrap_or_default()
                };
                (key, value)
            })
            .collect();
        let draft_values: Vec<_> = draft_signal.values().copied().collect();

        let mut scores = BTreeMap::new();
        for champion in champions {
            if !eligible.contains(champion.champion_id.as_str()) {
                scores.insert(
                    champion.champion_id.clone(),
                    TierScore {
                        eligible: false,
                        overall: None,
                        tier: Tier::NoTier,
                        sample: champion.pick_count,
                        min_sample: self.min_picks,
                        reason: Some("sample_too_small".to_owned()),
                        formula_version: MODEL_VERSION.to_owned(),
                        components: None,
                        best_role: None,
                        usable_roles: Vec::new(),
                    },
                );
                continue;
            }

            let rows = role_rows
                .get(&champion.champion_id)
                .expect("eligible role rows indexed");
            let mut usable_roles = Vec::new();
            let mut usable_sample = 0usize;
            for row in rows.iter().filter(|row| row.position != RoleFilter::All) {
                let values = role_values.get(&row.position).unwrap_or(&power_values);
                let power = percentile_rank(row.power_raw, values);
                if power >= 40.0
                    && (row.sample >= USABLE_ROLE_MIN_PICKS
                        || (row.sample >= USABLE_ROLE_STRONG_MIN_PICKS && power >= 70.0))
                {
                    usable_sample += row.sample;
                    usable_roles.push((row.position, row.sample));
                }
            }
            let role_count_score = usable_roles.len() as f64 / 5.0 * 100.0;
            let entropy_score = if usable_roles.len() > 1 && usable_sample > 0 {
                let entropy = usable_roles.iter().fold(0.0, |sum, (_, sample)| {
                    let probability = *sample as f64 / usable_sample as f64;
                    sum - probability * probability.ln()
                });
                entropy / 5.0_f64.ln() * 100.0
            } else {
                0.0
            };
            let versatility =
                round1((role_count_score * 0.65 + entropy_score * 0.35).clamp(0.0, 100.0));
            let power = round1(percentile_rank(
                best_power
                    .get(&champion.champion_id)
                    .copied()
                    .unwrap_or_default(),
                &power_values,
            ));
            let draft = round1(percentile_rank(
                draft_signal
                    .get(&champion.champion_id)
                    .copied()
                    .unwrap_or_default(),
                &draft_values,
            ));
            let effective_sample = champion.pick_count as f64
                + (champion.ban_count.min(if champion.pick_count > 0 {
                    champion.pick_count * 4
                } else {
                    40
                }) as f64)
                    * 0.10;
            let reliability = if effective_sample > 0.0 {
                round1(effective_sample / (effective_sample + RELIABILITY_PICKS) * 100.0)
            } else {
                0.0
            };
            let overall = round1(
                (power * self.power_weight
                    + draft * self.draft_weight
                    + versatility * self.versatility_weight)
                    .clamp(0.0, 100.0),
            );
            scores.insert(
                champion.champion_id.clone(),
                TierScore {
                    eligible: true,
                    overall: Some(overall),
                    tier: Tier::from_score(overall),
                    sample: champion.pick_count,
                    min_sample: self.min_picks,
                    reason: None,
                    formula_version: MODEL_VERSION.to_owned(),
                    components: Some(TierComponents {
                        power,
                        draft,
                        versatility,
                        reliability,
                    }),
                    best_role: best_role.get(&champion.champion_id).copied(),
                    usable_roles: usable_roles.into_iter().map(|(role, _)| role).collect(),
                },
            );
        }
        scores
    }
}

#[derive(Debug, Clone)]
struct RolePower {
    position: RoleFilter,
    sample: usize,
    power_raw: f64,
}

fn role_power_rows(champion: &ChampionInput) -> Vec<RolePower> {
    champion
        .by_position
        .iter()
        .filter(|(position, sample)| **position != RoleFilter::All && sample.matches > 0)
        .map(|(position, sample)| RolePower {
            position: *position,
            sample: sample.matches,
            power_raw: round1(risk_adjusted_win_rate(
                sample.wins,
                sample.matches,
                ROLE_PRIOR_PICKS,
            )),
        })
        .collect()
}

fn overall_power_row(champion: &ChampionInput) -> RolePower {
    RolePower {
        position: RoleFilter::All,
        sample: champion.pick_count,
        power_raw: round1(risk_adjusted_win_rate(
            champion.wins,
            champion.pick_count,
            PRIOR_PICKS,
        )),
    }
}

fn risk_adjusted_win_rate(wins: usize, games: usize, prior: f64) -> f64 {
    let games = games as f64;
    let wins = (wins as f64).clamp(0.0, games);
    let prior_wins = prior * 0.5;
    let alpha = wins + prior_wins;
    let beta = games - wins + prior_wins;
    let total = alpha + beta;
    let mean = if games > 0.0 {
        (wins + prior_wins) / (games + prior) * 100.0
    } else {
        50.0
    };
    let standard_deviation = if total > 0.0 {
        ((alpha * beta) / ((total * total) * (total + 1.0)))
            .max(0.0)
            .sqrt()
            * 100.0
    } else {
        0.0
    };
    (mean - WIN_RISK_Z * standard_deviation).clamp(0.0, 100.0)
}

fn percentile_rank(value: f64, values: &[f64]) -> f64 {
    let mut clean: Vec<_> = values
        .iter()
        .copied()
        .filter(|value| value.is_finite())
        .collect();
    if !value.is_finite() || clean.is_empty() {
        return 0.0;
    }
    clean.sort_by(f64::total_cmp);
    if clean.len() == 1 {
        return if value >= clean[0] { 100.0 } else { 0.0 };
    }
    if clean.last() == clean.first() {
        return 50.0;
    }
    let below_or_equal = clean
        .iter()
        .filter(|candidate| **candidate <= value)
        .count();
    round2((below_or_equal.saturating_sub(1)) as f64 / (clean.len() - 1) as f64 * 100.0)
}

fn logit_rate(rate: f64) -> Option<f64> {
    if !rate.is_finite() {
        return None;
    }
    let probability = (rate / 100.0).clamp(0.001, 0.999);
    Some((probability / (1.0 - probability)).ln())
}

fn z_scores(values: &BTreeMap<String, f64>) -> BTreeMap<String, f64> {
    if values.is_empty() {
        return BTreeMap::new();
    }
    let mean = values.values().sum::<f64>() / values.len() as f64;
    let variance = values
        .values()
        .map(|value| (value - mean).powi(2))
        .sum::<f64>()
        / values.len() as f64;
    let standard_deviation = variance.max(0.0).sqrt();
    if standard_deviation <= 0.0 {
        return values.keys().cloned().map(|key| (key, 0.0)).collect();
    }
    values
        .iter()
        .map(|(key, value)| (key.clone(), (value - mean) / standard_deviation))
        .collect()
}

fn round1(value: f64) -> f64 {
    (value * 10.0).round() / 10.0
}

fn round2(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}
