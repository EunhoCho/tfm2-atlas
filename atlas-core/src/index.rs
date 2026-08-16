use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tfm2_atlas_engine::{
    classify_role_profile, ChampionRoleProfile, DivisionFilter, GameScope, TierEngine, TierProfile,
    TierScore, RegionFilter, RoleFilter, RoleSample, Tier, ValidationError,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MatchKind {
    Tournament,
    Solo,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Division {
    First,
    Second,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChampionAnalyticsRow {
    pub champion_id: String,
    pub eligible: bool,
    pub overall: Option<f64>,
    pub tier: Tier,
    pub sample: usize,
    pub min_sample: usize,
    pub reason: Option<String>,
    pub score: TierScore,
    pub pick_count: usize,
    pub wins: usize,
    pub ban_count: usize,
    pub win_rate: Option<f64>,
    pub pick_rate: Option<f64>,
    pub ban_rate: Option<f64>,
    pub average_dealt: Option<f64>,
    pub average_taken: Option<f64>,
    pub average_healing: Option<f64>,
    pub patch_history: Vec<PatchPerformanceRow>,
    pub top_items: Vec<ItemUsageRow>,
    pub synergies: Vec<RelationRow>,
    pub matchups: Vec<RelationRow>,
    pub role_profile: ChampionRoleProfile,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PatchPerformanceRow {
    pub patch: String,
    pub pick_count: usize,
    pub wins: usize,
    pub ban_count: usize,
    pub win_rate: Option<f64>,
    pub pick_rate: f64,
    pub ban_rate: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ItemUsageRow {
    pub item_id: String,
    pub games: usize,
    pub adoption_rate: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RelationRow {
    pub champion_id: String,
    pub games: usize,
    pub wins: usize,
    pub win_rate: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReplaySummary {
    pub id: usize,
    pub kind: MatchKind,
    pub region: RegionFilter,
    pub division: Option<Division>,
    pub patch: String,
    pub blue_win: bool,
    pub blue_champions: Vec<String>,
    pub red_champions: Vec<String>,
    pub bans: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnalyticsState {
    pub data_revision: u64,
    pub selected_matches: usize,
    pub latest_patch: Option<String>,
    pub available_patches: Vec<String>,
    pub champions: Vec<ChampionAnalyticsRow>,
    pub replays: Vec<ReplaySummary>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReplayError {
    InvalidJson(String),
    MissingField(String),
    InvalidField(String),
    InvalidProfile(ValidationError),
    Cache(String),
}

impl std::fmt::Display for ReplayError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidJson(reason) => write!(formatter, "invalid replay JSON: {reason}"),
            Self::MissingField(field) => write!(formatter, "missing replay field: {field}"),
            Self::InvalidField(field) => write!(formatter, "invalid replay field: {field}"),
            Self::InvalidProfile(reason) => write!(formatter, "invalid profile: {reason}"),
            Self::Cache(reason) => write!(formatter, "invalid analytics index cache: {reason}"),
        }
    }
}

impl std::error::Error for ReplayError {}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct PickFact {
    champion_id: String,
    role: RoleFilter,
    won: bool,
    dealt: f64,
    taken: f64,
    healing: f64,
    items: Vec<String>,
    blue: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct MatchFact {
    id: usize,
    kind: MatchKind,
    region: RegionFilter,
    division: Option<Division>,
    patch: String,
    blue_team_id: Option<usize>,
    red_team_id: Option<usize>,
    picks: Vec<PickFact>,
    bans: Vec<String>,
    blue_win: bool,
}

#[derive(Debug, Clone, Default)]
pub struct MatchIndex {
    revision: u64,
    matches: BTreeMap<(MatchKind, usize), MatchFact>,
}

pub const MATCH_INDEX_CACHE_VERSION: u32 = 5;
pub const MATCH_INDEX_FORMULA_VERSION: &str = "atlas_1_0_33_role_profiles";
pub const MATCH_INDEX_CACHE_CHUNK_SIZE: usize = 256;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CachedMatchId {
    pub kind: MatchKind,
    pub id: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MatchIndexCacheManifest {
    pub cache_version: u32,
    pub game_version: String,
    pub formula_version: String,
    pub chunk_size: usize,
    pub chunk_count: usize,
    pub match_count: usize,
    pub revision: u64,
    pub match_ids: Vec<CachedMatchId>,
    pub chunk_checksums: Vec<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchIndexCache {
    pub manifest: MatchIndexCacheManifest,
    pub chunks: Vec<Vec<u8>>,
}

impl MatchIndexCache {
    pub fn from_index(index: &MatchIndex, game_version: &str) -> Result<Self, ReplayError> {
        let facts = index.matches.values().cloned().collect::<Vec<_>>();
        let mut chunks = Vec::new();
        for slice in facts.chunks(MATCH_INDEX_CACHE_CHUNK_SIZE) {
            let encoded =
                serde_json::to_vec(slice).map_err(|error| ReplayError::Cache(error.to_string()))?;
            chunks.push(lz4_flex::compress_prepend_size(&encoded));
        }
        let manifest = MatchIndexCacheManifest {
            cache_version: MATCH_INDEX_CACHE_VERSION,
            game_version: game_version.to_owned(),
            formula_version: MATCH_INDEX_FORMULA_VERSION.to_owned(),
            chunk_size: MATCH_INDEX_CACHE_CHUNK_SIZE,
            chunk_count: chunks.len(),
            match_count: facts.len(),
            revision: index.revision,
            match_ids: facts
                .iter()
                .map(|fact| CachedMatchId {
                    kind: fact.kind,
                    id: fact.id,
                })
                .collect(),
            chunk_checksums: chunks.iter().map(|chunk| fnv1a64(chunk)).collect(),
        };
        Ok(Self { manifest, chunks })
    }

    pub fn restore(&self, game_version: &str) -> Result<MatchIndex, ReplayError> {
        if self.manifest.cache_version != MATCH_INDEX_CACHE_VERSION
            || self.manifest.game_version != game_version
            || self.manifest.formula_version != MATCH_INDEX_FORMULA_VERSION
            || self.manifest.chunk_size != MATCH_INDEX_CACHE_CHUNK_SIZE
        {
            return Err(ReplayError::Cache(
                "cache version does not match this game build".to_owned(),
            ));
        }
        if self.chunks.len() != self.manifest.chunk_count
            || self.manifest.chunk_checksums.len() != self.chunks.len()
        {
            return Err(ReplayError::Cache(
                "cache manifest does not match its chunks".to_owned(),
            ));
        }
        let mut matches = BTreeMap::new();
        for (index, chunk) in self.chunks.iter().enumerate() {
            if fnv1a64(chunk) != self.manifest.chunk_checksums[index] {
                return Err(ReplayError::Cache(format!(
                    "cache chunk {index} checksum mismatch"
                )));
            }
            let decoded = lz4_flex::decompress_size_prepended(chunk)
                .map_err(|error| ReplayError::Cache(format!("cache chunk {index}: {error}")))?;
            let facts: Vec<MatchFact> = serde_json::from_slice(&decoded)
                .map_err(|error| ReplayError::Cache(format!("cache chunk {index}: {error}")))?;
            for fact in facts {
                if matches.insert((fact.kind, fact.id), fact).is_some() {
                    return Err(ReplayError::Cache(
                        "cache contains duplicate match ids".to_owned(),
                    ));
                }
            }
        }
        if matches.len() != self.manifest.match_count
            || self.manifest.match_ids.len() != matches.len()
            || self
                .manifest
                .match_ids
                .iter()
                .any(|entry| !matches.contains_key(&(entry.kind, entry.id)))
        {
            return Err(ReplayError::Cache(
                "cache match id manifest is inconsistent".to_owned(),
            ));
        }
        Ok(MatchIndex {
            revision: self.manifest.revision,
            matches,
        })
    }
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf29ce484222325_u64, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3)
    })
}

impl MatchIndex {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }

    pub fn match_count(&self) -> usize {
        self.matches.len()
    }

    pub fn contains(&self, kind: MatchKind, id: usize) -> bool {
        self.matches.contains_key(&(kind, id))
    }

    pub fn clear(&mut self) {
        self.matches.clear();
        self.revision = self.revision.wrapping_add(1);
    }

    pub fn record_tournament(
        &mut self,
        json: &str,
        region: RegionFilter,
        division: Division,
    ) -> Result<(), ReplayError> {
        self.record_json(MatchKind::Tournament, json, Some(region), Some(division))
    }

    pub fn record_solo(&mut self, json: &str) -> Result<(), ReplayError> {
        self.record_json(MatchKind::Solo, json, None, None)
    }

    pub fn record_json(
        &mut self,
        kind: MatchKind,
        json: &str,
        region: Option<RegionFilter>,
        division: Option<Division>,
    ) -> Result<(), ReplayError> {
        let value: Value = serde_json::from_str(json)
            .map_err(|error| ReplayError::InvalidJson(error.to_string()))?;
        let fact = MatchFact::parse(kind, &value, region, division)?;
        self.matches.insert((kind, fact.id), fact);
        self.revision = self.revision.wrapping_add(1);
        Ok(())
    }

    pub fn preview(&self, profile: &TierProfile) -> Result<AnalyticsState, ReplayError> {
        profile.validate().map_err(ReplayError::InvalidProfile)?;
        let scoped: Vec<_> = self
            .matches
            .values()
            .filter(|fact| scope_matches(profile.scope, fact.kind))
            .filter(|fact| profile.region == RegionFilter::All || profile.region == fact.region)
            .collect();
        let mut available_patches = scoped
            .iter()
            .map(|fact| fact.patch.clone())
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        available_patches.sort_by(|left, right| version_key(right).cmp(&version_key(left)));
        let candidates: Vec<_> = scoped
            .into_iter()
            .filter(|fact| division_matches(profile.division, fact.division))
            .collect();
        let latest_patch = candidates
            .iter()
            .map(|fact| fact.patch.as_str())
            .max_by(|left, right| version_key(left).cmp(&version_key(right)));
        let mut patch_match_counts = BTreeMap::<String, usize>::new();
        let mut patch_aggregates = BTreeMap::<String, BTreeMap<String, PatchAggregate>>::new();
        for fact in &candidates {
            *patch_match_counts.entry(fact.patch.clone()).or_default() += 1;
            let by_champion = patch_aggregates.entry(fact.patch.clone()).or_default();
            for pick in fact
                .picks
                .iter()
                .filter(|pick| profile.role == RoleFilter::All || profile.role == pick.role)
            {
                let row = by_champion.entry(pick.champion_id.clone()).or_default();
                row.picks += 1;
                row.wins += usize::from(pick.won);
            }
            for ban in &fact.bans {
                by_champion.entry(ban.clone()).or_default().bans += 1;
            }
        }
        let selected: Vec<_> = candidates
            .iter()
            .copied()
            .filter(|fact| match profile.patch.as_deref() {
                Some("latest") => latest_patch == Some(fact.patch.as_str()),
                Some(patch) => fact.patch == patch,
                None => true,
            })
            .collect();

        let mut aggregate = BTreeMap::<String, Aggregate>::new();
        for fact in &selected {
            let visible = fact
                .picks
                .iter()
                .filter(|pick| profile.role == RoleFilter::All || profile.role == pick.role)
                .collect::<Vec<_>>();
            for pick in visible {
                let row = aggregate.entry(pick.champion_id.clone()).or_default();
                row.picks += 1;
                row.wins += usize::from(pick.won);
                row.dealt += pick.dealt;
                row.taken += pick.taken;
                row.healing += pick.healing;
                for item in &pick.items {
                    *row.items.entry(item.clone()).or_default() += 1;
                }
                let role = row.roles.entry(pick.role).or_default();
                role.matches += 1;
                role.wins += usize::from(pick.won);
                for other in fact
                    .picks
                    .iter()
                    .filter(|other| other.champion_id != pick.champion_id)
                {
                    let relation = if other.blue == pick.blue {
                        row.synergies.entry(other.champion_id.clone()).or_default()
                    } else {
                        row.matchups.entry(other.champion_id.clone()).or_default()
                    };
                    relation.games += 1;
                    relation.wins += usize::from(pick.won);
                }
            }
            for ban in &fact.bans {
                aggregate.entry(ban.clone()).or_default().bans += 1;
            }
        }

        let total_matches = selected.len().max(1) as f64;
        let inputs: Vec<_> = aggregate
            .iter()
            .filter(|(_, row)| row.picks > 0 || row.bans > 0)
            .map(|(champion_id, row)| {
                let mut input = tfm2_atlas_engine::ChampionInput::new(
                    champion_id,
                    row.picks,
                    row.wins,
                    row.bans,
                    Some(round1(row.picks as f64 / total_matches * 100.0)),
                    Some(round1(row.bans as f64 / total_matches * 100.0)),
                    Some(round1(
                        (row.picks + row.bans) as f64 / total_matches * 100.0,
                    )),
                );
                input.by_position = row.roles.clone();
                input
            })
            .collect();
        let scores = TierEngine::role_aware_v6_for_preset(profile.preset)
            .with_min_picks(profile.sample_floor())
            .score(&inputs);
        let mut champions: Vec<_> = scores
            .into_iter()
            .map(|(champion_id, score)| {
                let aggregate = aggregate.get(&champion_id).cloned().unwrap_or_default();
                let patch_history =
                    patch_performance_rows(&patch_aggregates, &patch_match_counts, &champion_id);
                let role_profile =
                    role_profile_for(&champion_id, &candidates, profile, latest_patch);
                ChampionAnalyticsRow {
                    champion_id,
                    eligible: score.eligible,
                    overall: score.overall,
                    tier: score.tier,
                    sample: score.sample,
                    min_sample: score.min_sample,
                    reason: score.reason.clone(),
                    score,
                    pick_count: aggregate.picks,
                    wins: aggregate.wins,
                    ban_count: aggregate.bans,
                    win_rate: percent(aggregate.wins, aggregate.picks),
                    pick_rate: Some(round1(aggregate.picks as f64 / total_matches * 100.0)),
                    ban_rate: Some(round1(aggregate.bans as f64 / total_matches * 100.0)),
                    average_dealt: average(aggregate.dealt, aggregate.picks),
                    average_taken: average(aggregate.taken, aggregate.picks),
                    average_healing: average(aggregate.healing, aggregate.picks),
                    patch_history,
                    top_items: item_rows(&aggregate.items, aggregate.picks),
                    synergies: relation_rows(&aggregate.synergies),
                    matchups: relation_rows(&aggregate.matchups),
                    role_profile,
                }
            })
            .collect();
        champions.sort_by(|left, right| {
            right
                .overall
                .unwrap_or(f64::NEG_INFINITY)
                .total_cmp(&left.overall.unwrap_or(f64::NEG_INFINITY))
                .then(left.champion_id.cmp(&right.champion_id))
        });

        let mut replays = selected
            .iter()
            .map(|fact| ReplaySummary {
                id: fact.id,
                kind: fact.kind,
                region: fact.region,
                division: fact.division,
                patch: fact.patch.clone(),
                blue_win: fact.blue_win,
                blue_champions: fact
                    .picks
                    .iter()
                    .filter(|pick| pick.blue)
                    .map(|pick| pick.champion_id.clone())
                    .collect(),
                red_champions: fact
                    .picks
                    .iter()
                    .filter(|pick| !pick.blue)
                    .map(|pick| pick.champion_id.clone())
                    .collect(),
                bans: fact.bans.clone(),
            })
            .collect::<Vec<_>>();
        replays.sort_by(|left, right| right.id.cmp(&left.id));

        Ok(AnalyticsState {
            data_revision: self.revision,
            selected_matches: selected.len(),
            latest_patch: latest_patch.map(ToOwned::to_owned),
            available_patches,
            champions,
            replays,
        })
    }

    pub fn team_pick_preferences(
        &self,
        profile: &TierProfile,
        team_id: usize,
    ) -> Result<BTreeMap<String, TeamChampionPreference>, ReplayError> {
        profile.validate().map_err(ReplayError::InvalidProfile)?;
        let scoped = self
            .matches
            .values()
            .filter(|fact| scope_matches(profile.scope, fact.kind))
            .filter(|fact| profile.region == RegionFilter::All || profile.region == fact.region)
            .filter(|fact| division_matches(profile.division, fact.division))
            .collect::<Vec<_>>();
        let latest_patch = scoped
            .iter()
            .map(|fact| fact.patch.as_str())
            .max_by(|left, right| version_key(left).cmp(&version_key(right)));
        let selected = scoped
            .into_iter()
            .filter(|fact| match profile.patch.as_deref() {
                Some("latest") => latest_patch == Some(fact.patch.as_str()),
                Some(patch) => fact.patch == patch,
                None => true,
            });
        let mut counts = BTreeMap::<String, usize>::new();
        let mut matches = 0usize;
        for fact in selected {
            let side = if fact.blue_team_id == Some(team_id) {
                Some(true)
            } else if fact.red_team_id == Some(team_id) {
                Some(false)
            } else {
                None
            };
            let Some(blue) = side else { continue };
            matches += 1;
            for pick in fact.picks.iter().filter(|pick| pick.blue == blue) {
                *counts.entry(pick.champion_id.clone()).or_default() += 1;
            }
        }
        let maximum = counts.values().copied().max().unwrap_or(0).max(1) as f64;
        Ok(counts
            .into_iter()
            .map(|(champion_id, picks)| {
                let reliability = matches as f64 / (matches as f64 + 10.0);
                let score = round1(50.0 + (picks as f64 / maximum * 50.0) * reliability);
                (
                    champion_id.clone(),
                    TeamChampionPreference {
                        champion_id,
                        team_matches: matches,
                        picks,
                        score,
                    },
                )
            })
            .collect())
    }
}

fn role_profile_for(
    champion_id: &str,
    candidates: &[&MatchFact],
    profile: &TierProfile,
    latest_patch: Option<&str>,
) -> ChampionRoleProfile {
    let mut patch_order = candidates
        .iter()
        .map(|fact| fact.patch.clone())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    patch_order.sort_by(|left, right| version_key(right).cmp(&version_key(left)));

    let patches = match profile.patch.as_deref() {
        None => patch_order,
        Some("latest") => latest_patch
            .and_then(|latest| patch_order.iter().position(|patch| patch == latest))
            .map(|start| patch_order[start..].to_vec())
            .unwrap_or_default(),
        Some(selected) => patch_order
            .iter()
            .position(|patch| patch == selected)
            .map(|start| patch_order[start..].to_vec())
            .unwrap_or_default(),
    };
    let stop_at_floor = profile.patch.is_some();
    let required_matches = profile.sample_floor();
    let mut samples = BTreeMap::<RoleFilter, RoleSample>::new();
    let mut used_patches = Vec::new();
    let mut total_matches = 0usize;
    for patch in patches {
        used_patches.push(patch.clone());
        for pick in candidates
            .iter()
            .filter(|fact| fact.patch == patch)
            .flat_map(|fact| fact.picks.iter())
            .filter(|pick| pick.champion_id == champion_id && pick.role != RoleFilter::All)
        {
            let sample = samples.entry(pick.role).or_default();
            sample.matches += 1;
            sample.wins += usize::from(pick.won);
            total_matches += 1;
        }
        if stop_at_floor && total_matches >= required_matches {
            break;
        }
    }
    classify_role_profile(champion_id, &samples, required_matches, used_patches)
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamChampionPreference {
    pub champion_id: String,
    pub team_matches: usize,
    pub picks: usize,
    pub score: f64,
}

#[derive(Debug, Clone, Default)]
struct Aggregate {
    picks: usize,
    wins: usize,
    bans: usize,
    dealt: f64,
    taken: f64,
    healing: f64,
    roles: BTreeMap<RoleFilter, tfm2_atlas_engine::RoleSample>,
    items: BTreeMap<String, usize>,
    synergies: BTreeMap<String, RelationAggregate>,
    matchups: BTreeMap<String, RelationAggregate>,
}

#[derive(Debug, Clone, Default)]
struct PatchAggregate {
    picks: usize,
    wins: usize,
    bans: usize,
}

#[derive(Debug, Clone, Default)]
struct RelationAggregate {
    games: usize,
    wins: usize,
}

fn patch_performance_rows(
    patches: &BTreeMap<String, BTreeMap<String, PatchAggregate>>,
    match_counts: &BTreeMap<String, usize>,
    champion_id: &str,
) -> Vec<PatchPerformanceRow> {
    let mut rows = patches
        .iter()
        .filter_map(|(patch, champions)| {
            let aggregate = champions.get(champion_id)?;
            let matches = *match_counts.get(patch).unwrap_or(&0);
            (aggregate.picks > 0 || aggregate.bans > 0).then(|| PatchPerformanceRow {
                patch: patch.clone(),
                pick_count: aggregate.picks,
                wins: aggregate.wins,
                ban_count: aggregate.bans,
                win_rate: percent(aggregate.wins, aggregate.picks),
                pick_rate: if matches == 0 {
                    0.0
                } else {
                    round1(aggregate.picks as f64 / matches as f64 * 100.0)
                },
                ban_rate: if matches == 0 {
                    0.0
                } else {
                    round1(aggregate.bans as f64 / matches as f64 * 100.0)
                },
            })
        })
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| version_key(&right.patch).cmp(&version_key(&left.patch)));
    rows
}

impl MatchFact {
    fn parse(
        kind: MatchKind,
        value: &Value,
        region: Option<RegionFilter>,
        division: Option<Division>,
    ) -> Result<Self, ReplayError> {
        let id = required_u64(value, "id")? as usize;
        let patch = required_str(value, "version")?.to_owned();
        let blue_win = required_bool(value, "blue_team_win")?;
        let blue_team_id = value.get("blue_team_id").and_then(team_reference);
        let red_team_id = value.get("red_team_id").and_then(team_reference);
        let (region, division) = match kind {
            MatchKind::Tournament => (
                region.ok_or_else(|| ReplayError::MissingField("region".to_owned()))?,
                Some(division.ok_or_else(|| ReplayError::MissingField("division".to_owned()))?),
            ),
            MatchKind::Solo => (
                region_from_solo_id(required_u64(value, "region_id")? as usize)?,
                None,
            ),
        };
        let blue = required_array(value, "blue_team")?;
        let red = required_array(value, "red_team")?;
        if blue.is_empty() || red.is_empty() {
            return Err(ReplayError::InvalidField("team arrays".to_owned()));
        }
        let mut picks = parse_team(kind, blue, blue_win, true)?;
        picks.extend(parse_team(kind, red, !blue_win, false)?);
        let bans = if kind == MatchKind::Tournament {
            ["blue_ban", "red_ban"]
                .into_iter()
                .flat_map(|field| {
                    value
                        .get(field)
                        .and_then(Value::as_array)
                        .into_iter()
                        .flatten()
                        .filter_map(Value::as_str)
                        .map(ToOwned::to_owned)
                })
                .collect()
        } else {
            Vec::new()
        };
        Ok(Self {
            id,
            kind,
            region,
            division,
            patch,
            blue_team_id,
            red_team_id,
            picks,
            bans,
            blue_win,
        })
    }
}

fn team_reference(value: &Value) -> Option<usize> {
    if let Some(value) = value.as_u64() {
        return usize::try_from(value).ok();
    }
    if let Some(value) = value.as_str().and_then(|value| value.parse::<usize>().ok()) {
        return Some(value);
    }
    match value {
        Value::Object(object) => ["Normal", "normal", "Team", "team", "id", "team_id"]
            .into_iter()
            .find_map(|key| object.get(key).and_then(team_reference))
            .or_else(|| {
                (object.len() == 1)
                    .then(|| object.values().next())
                    .flatten()
                    .and_then(team_reference)
            }),
        _ => None,
    }
}

fn parse_team(
    kind: MatchKind,
    rows: &[Value],
    won: bool,
    blue: bool,
) -> Result<Vec<PickFact>, ReplayError> {
    rows.iter()
        .enumerate()
        .map(|(index, row)| {
            let champion_id = required_str(row, "champion")?.to_owned();
            let role = match kind {
                MatchKind::Tournament => parse_role(required_str(row, "position")?)?,
                MatchKind::Solo => role_from_index(index)?,
            };
            let statistics = if kind == MatchKind::Tournament {
                row.get("statistics").unwrap_or(row)
            } else {
                row
            };
            let dealt = number_any(statistics, &["dealing", "dealt"]);
            let taken = number_any(statistics, &["tanking", "taken"]);
            let healing = number_any(statistics, &["healing"]);
            let items = row
                .get("items")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(|value| match value {
                    Value::String(value) => Some(value.clone()),
                    Value::Number(value) => Some(value.to_string()),
                    _ => None,
                })
                .collect();
            Ok(PickFact {
                champion_id,
                role,
                won,
                dealt,
                taken,
                healing,
                items,
                blue,
            })
        })
        .collect()
}

fn required_u64(value: &Value, field: &str) -> Result<u64, ReplayError> {
    value
        .get(field)
        .ok_or_else(|| ReplayError::MissingField(field.to_owned()))?
        .as_u64()
        .ok_or_else(|| ReplayError::InvalidField(field.to_owned()))
}

fn required_str<'a>(value: &'a Value, field: &str) -> Result<&'a str, ReplayError> {
    value
        .get(field)
        .ok_or_else(|| ReplayError::MissingField(field.to_owned()))?
        .as_str()
        .ok_or_else(|| ReplayError::InvalidField(field.to_owned()))
}

fn required_bool(value: &Value, field: &str) -> Result<bool, ReplayError> {
    value
        .get(field)
        .ok_or_else(|| ReplayError::MissingField(field.to_owned()))?
        .as_bool()
        .ok_or_else(|| ReplayError::InvalidField(field.to_owned()))
}

fn required_array<'a>(value: &'a Value, field: &str) -> Result<&'a [Value], ReplayError> {
    value
        .get(field)
        .ok_or_else(|| ReplayError::MissingField(field.to_owned()))?
        .as_array()
        .map(Vec::as_slice)
        .ok_or_else(|| ReplayError::InvalidField(field.to_owned()))
}

fn number_any(value: &Value, fields: &[&str]) -> f64 {
    fields
        .iter()
        .find_map(|field| value.get(*field).and_then(Value::as_f64))
        .unwrap_or_default()
}

fn parse_role(value: &str) -> Result<RoleFilter, ReplayError> {
    match value.to_ascii_lowercase().as_str() {
        "top" => Ok(RoleFilter::Top),
        "jungle" => Ok(RoleFilter::Jungle),
        "mid" => Ok(RoleFilter::Mid),
        "bottom" | "bot" => Ok(RoleFilter::Bot),
        "support" => Ok(RoleFilter::Support),
        _ => Err(ReplayError::InvalidField(format!("position:{value}"))),
    }
}

fn role_from_index(index: usize) -> Result<RoleFilter, ReplayError> {
    [
        RoleFilter::Top,
        RoleFilter::Jungle,
        RoleFilter::Mid,
        RoleFilter::Bot,
        RoleFilter::Support,
    ]
    .get(index)
    .copied()
    .ok_or_else(|| ReplayError::InvalidField(format!("position index:{index}")))
}

fn region_from_solo_id(region_id: usize) -> Result<RegionFilter, ReplayError> {
    [
        RegionFilter::Kr,
        RegionFilter::Cn,
        RegionFilter::Eu,
        RegionFilter::Na,
        RegionFilter::Sa,
        RegionFilter::Jp,
    ]
    .get(region_id)
    .copied()
    .ok_or_else(|| ReplayError::InvalidField(format!("region_id:{region_id}")))
}

fn scope_matches(scope: GameScope, kind: MatchKind) -> bool {
    matches!(
        (scope, kind),
        (GameScope::Tournament, MatchKind::Tournament)
            | (GameScope::Solo, MatchKind::Solo)
            | (GameScope::SoloAndTournament, _)
    )
}

fn division_matches(filter: DivisionFilter, division: Option<Division>) -> bool {
    match filter {
        DivisionFilter::All => true,
        DivisionFilter::First => division == Some(Division::First),
        DivisionFilter::Second => division == Some(Division::Second),
    }
}

fn version_key(version: &str) -> Vec<u64> {
    version
        .split(|character: char| !character.is_ascii_digit())
        .filter(|part| !part.is_empty())
        .filter_map(|part| part.parse().ok())
        .collect()
}

fn round1(value: f64) -> f64 {
    (value * 10.0).round() / 10.0
}

fn percent(numerator: usize, denominator: usize) -> Option<f64> {
    (denominator > 0).then(|| round1(numerator as f64 / denominator as f64 * 100.0))
}

fn average(total: f64, count: usize) -> Option<f64> {
    (count > 0).then(|| round1(total / count as f64))
}

fn item_rows(items: &BTreeMap<String, usize>, picks: usize) -> Vec<ItemUsageRow> {
    let mut rows = items
        .iter()
        .map(|(item_id, games)| ItemUsageRow {
            item_id: item_id.clone(),
            games: *games,
            adoption_rate: if picks == 0 {
                0.0
            } else {
                round1(*games as f64 / picks as f64 * 100.0)
            },
        })
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| {
        right
            .games
            .cmp(&left.games)
            .then(left.item_id.cmp(&right.item_id))
    });
    rows
}

fn relation_rows(relations: &BTreeMap<String, RelationAggregate>) -> Vec<RelationRow> {
    let mut rows = relations
        .iter()
        .map(|(champion_id, relation)| RelationRow {
            champion_id: champion_id.clone(),
            games: relation.games,
            wins: relation.wins,
            win_rate: percent(relation.wins, relation.games).unwrap_or_default(),
        })
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| {
        right
            .games
            .cmp(&left.games)
            .then_with(|| right.win_rate.total_cmp(&left.win_rate))
            .then_with(|| left.champion_id.cmp(&right.champion_id))
    });
    rows
}
