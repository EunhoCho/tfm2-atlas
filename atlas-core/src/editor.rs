//! Community Editor record adapters for the 0.5.5 Stable API.

use std::{
    collections::BTreeMap,
    sync::atomic::{AtomicBool, Ordering},
};

use mod_api_stable::{RecordKindV1, StableServerCtx};
use serde_json::{json, Map, Value};
use tfm2_atlas_engine::{apply_lock_to_record, LockSet};

use crate::runtime::{
    hex_encode, patch_record, publish_client_record_sync, verified_write_whole_record, RuntimeState,
};
use crate::{PlayerEditRequest, StaffEditRequest};

const PLAYER_STATS: [&str; 12] = [
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
const STAFF_STATS: [&str; 10] = [
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

static TRANSFER_ALWAYS_SUCCESS: AtomicBool = AtomicBool::new(false);
static RECRUITMENT_INSTANT_RETRY: AtomicBool = AtomicBool::new(false);

pub(crate) fn reset_runtime_overrides() {
    TRANSFER_ALWAYS_SUCCESS.store(false, Ordering::Release);
    RECRUITMENT_INSTANT_RETRY.store(false, Ordering::Release);
}

pub fn enforce_recruitment_overrides(ctx: &mut StableServerCtx<'_>, player_team_id: usize) {
    let force_success = TRANSFER_ALWAYS_SUCCESS.load(Ordering::Acquire);
    let instant_retry = RECRUITMENT_INSTANT_RETRY.load(Ordering::Acquire);
    if !force_success && !instant_retry {
        return;
    }
    for athlete_id in ctx.record_ids(RecordKindV1::Athlete) {
        let Some(raw) = ctx.record_get_json(RecordKindV1::Athlete, athlete_id, "") else {
            continue;
        };
        let Ok(mut record) = serde_json::from_str::<Value>(&raw) else {
            continue;
        };
        if !apply_recruitment_overrides(&mut record, player_team_id, force_success, instant_retry) {
            continue;
        }
        let _ = verified_write_whole_record(ctx, RecordKindV1::Athlete, athlete_id, &record);
    }
}

pub fn apply_recruitment_overrides(
    record: &mut Value,
    player_team_id: usize,
    force_success: bool,
    instant_retry: bool,
) -> bool {
    let Some(contract) = record.get_mut("contract").and_then(Value::as_object_mut) else {
        return false;
    };
    let mut changed = false;
    if let Some(active) = contract
        .get_mut("InContract")
        .and_then(Value::as_object_mut)
    {
        for key in ["transfer_requests", "recruit_requests"] {
            if let Some(requests) = active.get_mut(key).and_then(Value::as_array_mut) {
                changed |= patch_recruitment_requests(
                    requests,
                    player_team_id,
                    force_success,
                    instant_retry,
                );
            }
        }
    } else if let Some(free_agent) = contract.get_mut("FreeAgent").and_then(Value::as_object_mut) {
        if let Some(requests) = free_agent.get_mut("requests").and_then(Value::as_array_mut) {
            changed |=
                patch_recruitment_requests(requests, player_team_id, force_success, instant_retry);
        }
    }
    changed
}

fn patch_recruitment_requests(
    requests: &mut [Value],
    player_team_id: usize,
    force_success: bool,
    instant_retry: bool,
) -> bool {
    let mut changed = false;
    for request in requests {
        if request.get("team_id").and_then(Value::as_u64) != Some(player_team_id as u64) {
            continue;
        }
        if instant_retry {
            if let Some(last_date) = request.get("last_date").cloned() {
                if request.get("cooldown_until") != Some(&last_date) {
                    request["cooldown_until"] = last_date;
                    changed = true;
                }
            }
        }
        if force_success {
            if let Some(last_paper) = request
                .get_mut("phase")
                .and_then(Value::as_array_mut)
                .and_then(|phase| phase.last_mut())
            {
                if last_paper.get("state") != Some(&Value::String("Accepted".to_owned())) {
                    last_paper["state"] = Value::String("Accepted".to_owned());
                    changed = true;
                }
            }
        }
    }
    changed
}

pub(crate) fn editor_data(
    ctx: &StableServerCtx<'_>,
    state: &RuntimeState,
    payload: &Value,
) -> Result<Value, String> {
    match payload
        .get("view")
        .and_then(Value::as_str)
        .unwrap_or("overview")
    {
        "overview" => editor_overview(ctx, state),
        "player" => editor_player(
            ctx,
            payload
                .get("id")
                .and_then(Value::as_u64)
                .ok_or("INVALID_ID")? as usize,
        ),
        "staff" => editor_staff(
            ctx,
            payload
                .get("id")
                .and_then(Value::as_u64)
                .ok_or("INVALID_ID")? as usize,
        ),
        "economy" => editor_economy(ctx, state),
        "recruitment" => Ok(editor_recruitment()),
        _ => Err("INVALID_EDITOR_VIEW".to_owned()),
    }
}

pub(crate) fn apply_editor_settings(
    ctx: &mut StableServerCtx<'_>,
    state: &RuntimeState,
    payload: &Value,
) -> Result<Value, String> {
    let economy = payload
        .get("economy")
        .map(|value| {
            let object = value.as_object().ok_or("INVALID_ECONOMY")?;
            ["total_balance", "transfer_budget", "salary_budget"]
                .map(|key| {
                    let value = object
                        .get(key)
                        .and_then(Value::as_f64)
                        .ok_or("INVALID_ECONOMY")?;
                    (value.is_finite() && value >= 0.0)
                        .then_some(value)
                        .ok_or("INVALID_ECONOMY")
                })
                .into_iter()
                .collect::<Result<Vec<_>, _>>()
        })
        .transpose()?;
    let recruitment = payload
        .get("recruitment")
        .map(|value| {
            let object = value.as_object().ok_or("INVALID_RECRUITMENT_SETTINGS")?;
            Ok::<_, &str>((
                object
                    .get("transfer_always_success")
                    .and_then(Value::as_bool)
                    .ok_or("INVALID_RECRUITMENT_SETTINGS")?,
                object
                    .get("instant_retry")
                    .and_then(Value::as_bool)
                    .ok_or("INVALID_RECRUITMENT_SETTINGS")?,
            ))
        })
        .transpose()?;
    if economy.is_none() && recruitment.is_none() {
        return Err("EMPTY_EDITOR_SETTINGS".to_owned());
    }
    if let Some(values) = economy {
        set_economy(ctx, state, values)?;
    }
    if let Some((transfer, retry)) = recruitment {
        TRANSFER_ALWAYS_SUCCESS.store(transfer, Ordering::Release);
        RECRUITMENT_INSTANT_RETRY.store(retry, Ordering::Release);
    }
    Ok(json!({
        "economy": editor_economy(ctx, state)?,
        "recruitment": editor_recruitment(),
    }))
}

fn editor_overview(ctx: &StableServerCtx<'_>, state: &RuntimeState) -> Result<Value, String> {
    let athletes = read_all(ctx, RecordKindV1::Athlete)?;
    let staffs = read_all(ctx, RecordKindV1::Staff)?;
    let teams = read_all(ctx, RecordKindV1::Team)?;
    let team_names = teams
        .iter()
        .map(|(id, team)| (*id, team_display_name(team, *id)))
        .collect::<BTreeMap<_, _>>();
    let mut players = athletes
        .iter()
        .map(|(id, player)| {
            let stat = object(player, "stat")?;
            let contract = ContractView::from(player);
            let team = contract
                .team_id
                .and_then(|id| team_names.get(&id).cloned())
                .unwrap_or_else(|| "Free Agent".to_owned());
            Ok(json!({
                "id": id,
                "name": scalar(&player["name"]),
                "age": scalar(&player["age"]),
                "team": team,
                "teamId": contract.team_id,
                "region": primary_region(stat),
                "position": position_summary(stat),
                "salary": contract.annual,
                "potential": nested(player, &["hidden", "potential"]),
            }))
        })
        .collect::<Result<Vec<_>, String>>()?;
    players.sort_by_key(|row| {
        row.get("name")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_lowercase()
    });
    let mut staff_rows = staffs
        .iter()
        .map(|(id, staff)| {
            let contract = ContractView::from(staff);
            let team = contract
                .team_id
                .and_then(|id| team_names.get(&id).cloned())
                .unwrap_or_else(|| "Free Agent".to_owned());
            json!({
                "id": id,
                "name": scalar(&staff["name"]),
                "age": scalar(&staff["age"]),
                "team": team,
                "teamId": contract.team_id,
                "role": scalar(&staff["role"]),
                "salary": contract.annual,
            })
        })
        .collect::<Vec<_>>();
    staff_rows.sort_by_key(|row| {
        row.get("name")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_lowercase()
    });
    let player_team = state.player_team_id(ctx);
    let mut team_rows = teams
        .iter()
        .map(|(id, team)| {
            let roster = athletes
                .iter()
                .filter(|(_, value)| ContractView::from(value).team_id == Some(*id))
                .count();
            let staff_count = staffs
                .iter()
                .filter(|(_, value)| ContractView::from(value).team_id == Some(*id))
                .count();
            json!({
                "id": id,
                "name": team_display_name(team, *id),
                "playerTeam": player_team == Some(*id),
                "roster": roster,
                "staffs": staff_count,
            })
        })
        .collect::<Vec<_>>();
    team_rows.sort_by_key(|row| {
        row.get("name")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_lowercase()
    });
    Ok(json!({ "players": players, "staffs": staff_rows, "teams": team_rows }))
}

fn editor_player(ctx: &StableServerCtx<'_>, id: usize) -> Result<Value, String> {
    let player = read_record(ctx, RecordKindV1::Athlete, id)?;
    let stat = object(&player, "stat")?;
    let contract = ContractView::from(&player);
    let bonuses = incentive_fields(&contract.body);
    let stats = PLAYER_STATS.map(|key| stat.get(key).cloned().unwrap_or(Value::Null));
    let positions = ["top", "jungle", "mid", "bottom", "support"]
        .map(|key| stat.get(key).cloned().unwrap_or(Value::Null));
    Ok(json!({
        "id": id,
        "name": scalar(&player["name"]),
        "stats": stats,
        "positions": positions,
        "potential": nested(&player, &["hidden", "potential"]),
        "annualSalary": contract.annual,
        "weeklySalary": contract.weekly,
        "teamId": contract.team_id,
        "startDate": contract.start_date.chars().take(10).collect::<String>(),
        "endDate": contract.end_date.chars().take(10).collect::<String>(),
        "transferFee": contract.transfer_fee,
        "squadStatus": scalar(&player["squad_status"]),
        "bonuses": { "pog": bonuses[0], "league": bonuses[1], "rank": bonuses[2], "match": bonuses[3], "win": bonuses[4] },
        "primaryRegion": primary_region(stat),
        "communication": stat.get("language").cloned().unwrap_or_else(|| json!({})),
        "communicationXp": player.pointer("/training_exp/language_by_region").cloned().unwrap_or_else(|| json!({})),
        "age": player.get("age").cloned().unwrap_or(Value::Null),
        "stamina": player.pointer("/management/stamina").cloned().unwrap_or(Value::Null),
        "condition": player.pointer("/management/condition").cloned().unwrap_or(Value::Null),
        "stress": player.pointer("/management/stress").cloned().unwrap_or(Value::Null),
    }))
}

fn editor_staff(ctx: &StableServerCtx<'_>, id: usize) -> Result<Value, String> {
    let staff = read_record(ctx, RecordKindV1::Staff, id)?;
    let stat = object(&staff, "stat")?;
    let contract = ContractView::from(&staff);
    Ok(json!({
        "id": id,
        "name": scalar(&staff["name"]),
        "age": staff.get("age").cloned().unwrap_or(Value::Null),
        "role": scalar(&staff["role"]),
        "stats": STAFF_STATS.map(|key| stat.get(key).cloned().unwrap_or(Value::Null)),
        "annualSalary": contract.annual,
        "teamId": contract.team_id,
        "startDate": contract.start_date.chars().take(10).collect::<String>(),
        "endDate": contract.end_date.chars().take(10).collect::<String>(),
        "communication": staff.get("language").cloned().unwrap_or_else(|| json!({})),
    }))
}

fn editor_economy(ctx: &StableServerCtx<'_>, state: &RuntimeState) -> Result<Value, String> {
    let team = read_record(ctx, RecordKindV1::Team, player_team(ctx, state)?)?;
    Ok(json!({
        "total_balance": team.get("total_balance").cloned().unwrap_or(Value::Null),
        "transfer_budget": team.get("transfer_budget").cloned().unwrap_or(Value::Null),
        "salary_budget": team.get("salary_budget").cloned().unwrap_or(Value::Null),
    }))
}

fn editor_recruitment() -> Value {
    json!({
        "transfer_always_success": TRANSFER_ALWAYS_SUCCESS.load(Ordering::Acquire),
        "instant_retry": RECRUITMENT_INSTANT_RETRY.load(Ordering::Acquire),
    })
}

pub(crate) fn get_tier_sync(
    ctx: &StableServerCtx<'_>,
    state: &RuntimeState,
) -> Result<String, String> {
    if !state.tier_profile_enabled() {
        return Ok(tier_sync_response(None, &BTreeMap::new()));
    }
    let team_id = player_team(ctx, state)?;
    let team = read_record(ctx, RecordKindV1::Team, team_id)?;
    let tiers = team
        .get("champion_tiers")
        .and_then(Value::as_object)
        .ok_or_else(|| "TEAM_PATH_MISSING:champion_tiers".to_owned())?
        .iter()
        .filter_map(|(champion_id, tier)| {
            let tier = tier.as_str()?;
            matches!(tier, "S" | "A" | "B" | "C" | "D" | "NoTier")
                .then(|| (champion_id.clone(), tier.to_owned()))
        })
        .collect::<BTreeMap<_, _>>();
    Ok(tier_sync_response(Some(team_id), &tiers))
}

pub fn tier_sync_response(team_id: Option<usize>, tiers: &BTreeMap<String, String>) -> String {
    let Some(team_id) = team_id else {
        return "OK|TIER_SYNC||".to_owned();
    };
    let payload = tiers
        .iter()
        .map(|(champion_id, tier)| format!("{}:{tier}", hex_encode(champion_id)))
        .collect::<Vec<_>>()
        .join(";");
    format!("OK|TIER_SYNC|{team_id}|{payload}")
}

fn set_economy(
    ctx: &mut StableServerCtx<'_>,
    state: &RuntimeState,
    values: Vec<f64>,
) -> Result<(), String> {
    let team_id = player_team(ctx, state)?;
    patch_record(ctx, RecordKindV1::Team, team_id, |record| {
        for (key, value) in ["total_balance", "transfer_budget", "salary_budget"]
            .into_iter()
            .zip(&values)
        {
            if record.get(key).is_none() {
                return Err(format!("TEAM_PATH_MISSING:{key}"));
            }
            record[key] = json!(value);
        }
        Ok(())
    })?;
    Ok(())
}

pub(crate) fn move_player(
    ctx: &mut StableServerCtx<'_>,
    athlete: usize,
    team: usize,
) -> Result<String, String> {
    let before = read_record(ctx, RecordKindV1::Athlete, athlete)?;
    if !ctx.force_transfer(athlete, team, 0.0) {
        return Err("TRANSFER_REJECTED".to_owned());
    }
    let readback = read_record(ctx, RecordKindV1::Athlete, athlete)?;
    let actual_team = readback
        .pointer("/contract/InContract/team_id")
        .and_then(Value::as_u64)
        .map(|value| value as usize);
    if actual_team != Some(team) {
        return Err(format!(
            "TRANSFER_READBACK_MISMATCH:expected={team}:actual={actual_team:?}"
        ));
    }
    publish_client_record_sync(RecordKindV1::Athlete, athlete, &before, &readback);
    Ok("OK|UPDATED".to_owned())
}

pub(crate) fn apply_player_edit(
    ctx: &mut StableServerCtx<'_>,
    request: &PlayerEditRequest,
) -> Result<(Value, Vec<String>), String> {
    validate_player_edit(request)?;
    let mut changed_paths = Vec::new();
    let record = patch_record(ctx, RecordKindV1::Athlete, request.athlete_id, |record| {
        if let Some(name) = request.name.as_deref() {
            require_string_path(record, &["name"])?;
            record["name"] = Value::String(name.trim().to_owned());
            changed_paths.push("name".to_owned());
        }
        if let Some(age) = request.age {
            replace_existing(record, &["age"], json!(age))?;
            changed_paths.push("age".to_owned());
        }
        if !request.stats.is_empty() {
            let stat = record
                .get_mut("stat")
                .and_then(Value::as_object_mut)
                .ok_or("STAT_PATH_MISSING")?;
            for (key, value) in &request.stats {
                if !stat.contains_key(key) {
                    return Err(format!("STAT_PATH_MISSING:{key}"));
                }
                stat.insert(key.clone(), json!(value));
                changed_paths.push(format!("stat.{key}"));
            }
        }
        if !request.positions.is_empty() {
            let stat = record
                .get_mut("stat")
                .and_then(Value::as_object_mut)
                .ok_or("STAT_PATH_MISSING")?;
            for (key, value) in &request.positions {
                if !stat.contains_key(key) {
                    return Err(format!("POSITION_PATH_MISSING:{key}"));
                }
                stat.insert(key.clone(), json!(value));
                changed_paths.push(format!("stat.{key}"));
            }
        }
        if let Some(value) = request.potential {
            replace_existing(record, &["hidden", "potential"], json!(value))?;
            changed_paths.push("hidden.potential".to_owned());
        }
        if let Some(value) = request.stamina {
            replace_existing(record, &["management", "stamina"], json!(value))?;
            changed_paths.push("management.stamina".to_owned());
        }
        if let Some(value) = request.stress {
            replace_existing(record, &["management", "stress"], json!(value))?;
            changed_paths.push("management.stress".to_owned());
        }
        if let Some(value) = request.condition {
            replace_existing(record, &["management", "condition"], json!(value))?;
            changed_paths.push("management.condition".to_owned());
        }
        if let Some(annual) = request.annual_salary {
            let contract = active_contract_mut(record).ok_or("FREE_AGENT")?;
            if !contract.contains_key("weekly_salary") {
                return Err("CONTRACT_PATH_MISSING:weekly_salary".to_owned());
            }
            contract.insert("weekly_salary".to_owned(), json!(annual / 52.0));
            changed_paths.push("contract.InContract.weekly_salary".to_owned());
        }
        if let Some(communication) = &request.communication {
            let values = record
                .pointer_mut("/stat/language")
                .and_then(Value::as_object_mut)
                .ok_or("LANGUAGE_PATH_MISSING")?;
            values.clear();
            for (region, value) in communication.iter().filter(|(_, value)| **value > 0) {
                values.insert(region.to_string(), json!(value));
            }
            changed_paths.push("stat.language".to_owned());
        }
        if !request.mastery.is_empty() {
            let lock = LockSet::new_mastery(
                request.athlete_id,
                request.athlete_id.to_string(),
                request
                    .mastery
                    .iter()
                    .map(|value| (value.champion_id.clone(), value.value as f64))
                    .collect(),
            );
            apply_lock_to_record(record, &lock).map_err(|error| error.to_string())?;
            changed_paths.extend(
                request
                    .mastery
                    .iter()
                    .map(|value| format!("champion_proficiency.{}.value", value.champion_id)),
            );
        }
        if let Some(contract) = &request.contract {
            apply_contract_fields(
                record,
                contract.team_id,
                &contract.start_date,
                &contract.end_date,
                contract.annual_salary,
                Some(contract.transfer_fee),
                Some(contract.squad_status.clone()),
                Some(Value::Array(contract.incentives.clone())),
            )?;
            changed_paths.extend([
                "contract.InContract.team_id".to_owned(),
                "contract.InContract.start_date".to_owned(),
                "contract.InContract.end_date".to_owned(),
                "contract.InContract.weekly_salary".to_owned(),
                "contract.InContract.transfer_fee".to_owned(),
                "contract.InContract.incentives".to_owned(),
                "squad_status".to_owned(),
            ]);
        }
        Ok(())
    })?;
    changed_paths.sort();
    changed_paths.dedup();
    Ok((record, changed_paths))
}

pub(crate) fn apply_staff_edit(
    ctx: &mut StableServerCtx<'_>,
    request: &StaffEditRequest,
) -> Result<(Value, Vec<String>), String> {
    validate_staff_edit(request)?;
    let mut changed_paths = Vec::new();
    let record = patch_record(ctx, RecordKindV1::Staff, request.staff_id, |record| {
        if let Some(name) = request.name.as_deref() {
            require_string_path(record, &["name"])?;
            record["name"] = Value::String(name.trim().to_owned());
            changed_paths.push("name".to_owned());
        }
        if let Some(age) = request.age {
            replace_existing(record, &["age"], json!(age))?;
            changed_paths.push("age".to_owned());
        }
        if !request.stats.is_empty() {
            let stat = record
                .get_mut("stat")
                .and_then(Value::as_object_mut)
                .ok_or("STAT_PATH_MISSING")?;
            for (key, value) in &request.stats {
                if !stat.contains_key(key) {
                    return Err(format!("STAT_PATH_MISSING:{key}"));
                }
                stat.insert(key.clone(), json!(value));
                changed_paths.push(format!("stat.{key}"));
            }
        }
        if let Some(annual) = request.annual_salary {
            let contract = active_contract_mut(record).ok_or("FREE_AGENT")?;
            if !contract.contains_key("weekly_salary") {
                return Err("CONTRACT_PATH_MISSING:weekly_salary".to_owned());
            }
            contract.insert("weekly_salary".to_owned(), json!(annual / 52.0));
            changed_paths.push("contract.InContract.weekly_salary".to_owned());
        }
        if let Some(communication) = &request.communication {
            let language = record
                .get_mut("language")
                .and_then(Value::as_object_mut)
                .ok_or("LANGUAGE_PATH_MISSING")?;
            language.clear();
            for (region, value) in communication.iter().filter(|(_, value)| **value > 0) {
                language.insert(region.to_string(), json!(value));
            }
            changed_paths.push("language".to_owned());
        }
        if let Some(contract) = &request.contract {
            apply_contract_fields(
                record,
                contract.team_id,
                &contract.start_date,
                &contract.end_date,
                contract.annual_salary,
                None,
                None,
                None,
            )?;
            changed_paths.extend([
                "contract.InContract.team_id".to_owned(),
                "contract.InContract.start_date".to_owned(),
                "contract.InContract.end_date".to_owned(),
                "contract.InContract.weekly_salary".to_owned(),
            ]);
        }
        Ok(())
    })?;
    changed_paths.sort();
    changed_paths.dedup();
    Ok((record, changed_paths))
}

fn validate_player_edit(request: &PlayerEditRequest) -> Result<(), String> {
    validate_optional_name(request.name.as_deref())?;
    if request
        .age
        .is_some_and(|value| !(16..=100).contains(&value))
    {
        return Err("AGE_OUT_OF_RANGE".to_owned());
    }
    validate_exact_stat_map(&request.stats, &PLAYER_STATS, 1, 100)?;
    validate_exact_stat_map(
        &request.positions,
        &["top", "jungle", "mid", "bottom", "support"],
        0,
        100,
    )?;
    if request
        .positions
        .values()
        .filter(|value| **value > 0)
        .count()
        > 3
    {
        return Err("TOO_MANY_POSITIONS".to_owned());
    }
    if request
        .potential
        .is_some_and(|value| !(1..=100).contains(&value))
    {
        return Err("POTENTIAL_OUT_OF_RANGE".to_owned());
    }
    for value in [request.stamina, request.stress, request.condition]
        .into_iter()
        .flatten()
    {
        if !(0..=100).contains(&value) {
            return Err("CONDITION_OUT_OF_RANGE".to_owned());
        }
    }
    validate_salary(request.annual_salary)?;
    validate_communication(request.communication.as_ref())?;
    if request
        .mastery
        .iter()
        .any(|value| value.champion_id.trim().is_empty() || !(0..=100).contains(&value.value))
    {
        return Err("INVALID_MASTERY".to_owned());
    }
    if let Some(contract) = &request.contract {
        validate_contract_values(
            &contract.start_date,
            &contract.end_date,
            contract.annual_salary,
            contract.transfer_fee,
        )?;
    }
    Ok(())
}

fn validate_staff_edit(request: &StaffEditRequest) -> Result<(), String> {
    validate_optional_name(request.name.as_deref())?;
    if request
        .age
        .is_some_and(|value| !(16..=100).contains(&value))
    {
        return Err("AGE_OUT_OF_RANGE".to_owned());
    }
    validate_exact_stat_map(&request.stats, &STAFF_STATS, 1, 100)?;
    validate_salary(request.annual_salary)?;
    validate_communication(request.communication.as_ref())?;
    if let Some(contract) = &request.contract {
        validate_contract_values(
            &contract.start_date,
            &contract.end_date,
            contract.annual_salary,
            0.0,
        )?;
    }
    Ok(())
}

fn validate_optional_name(name: Option<&str>) -> Result<(), String> {
    if let Some(name) = name {
        let name = name.trim();
        if name.is_empty() || name.chars().count() > 100 || name.chars().any(char::is_control) {
            return Err("INVALID_NAME".to_owned());
        }
    }
    Ok(())
}

fn validate_exact_stat_map(
    values: &BTreeMap<String, i64>,
    allowed: &[&str],
    min: i64,
    max: i64,
) -> Result<(), String> {
    if values.keys().any(|key| !allowed.contains(&key.as_str())) {
        return Err("UNKNOWN_STAT".to_owned());
    }
    if values.values().any(|value| !(min..=max).contains(value)) {
        return Err("STAT_OUT_OF_RANGE".to_owned());
    }
    Ok(())
}

fn validate_salary(annual: Option<f64>) -> Result<(), String> {
    if annual.is_some_and(|value| !value.is_finite() || value < 0.0) {
        return Err("SALARY_OUT_OF_RANGE".to_owned());
    }
    Ok(())
}

fn validate_communication(values: Option<&BTreeMap<usize, i64>>) -> Result<(), String> {
    if values.is_some_and(|values| {
        values
            .iter()
            .any(|(region, value)| *region > 5 || !(0..=100).contains(value))
    }) {
        return Err("COMMUNICATION_OUT_OF_RANGE".to_owned());
    }
    Ok(())
}

fn require_string_path(record: &Value, path: &[&str]) -> Result<(), String> {
    let mut current = record;
    for segment in path {
        current = current.get(*segment).ok_or("PATH_MISSING")?;
    }
    current
        .as_str()
        .map(|_| ())
        .ok_or_else(|| "PATH_TYPE_MISMATCH".to_owned())
}

fn apply_contract_fields(
    record: &mut Value,
    team: usize,
    start: &str,
    end: &str,
    annual: f64,
    transfer: Option<f64>,
    squad: Option<String>,
    incentives: Option<Value>,
) -> Result<(), String> {
    if active_contract(record).is_none() {
        let contract = record
            .get_mut("contract")
            .and_then(Value::as_object_mut)
            .ok_or("CONTRACT_PATH_MISSING:contract")?;
        if !contract.contains_key("FreeAgent") {
            return Err("FREE_AGENT_ACTIVE_CONTRACT_UNVERIFIED".to_owned());
        }
        *contract = Map::from_iter([(
            "InContract".to_owned(),
            json!({
                "team_id": team,
                "start_date": format!("{start}T00:00:00"),
                "end_date": format!("{end}T23:59:59"),
                "weekly_salary": annual / 52.0,
                "transfer_fee": transfer.unwrap_or(0.0),
                "incentives": incentives.clone().unwrap_or_else(|| json!([])),
                "transfer_requests": [],
                "recruit_requests": []
            }),
        )]);
    } else {
        let contract = active_contract_mut(record)
            .ok_or_else(|| "FREE_AGENT_ACTIVE_CONTRACT_UNVERIFIED".to_owned())?;
        for key in ["team_id", "start_date", "end_date", "weekly_salary"] {
            if !contract.contains_key(key) {
                return Err(format!("CONTRACT_PATH_MISSING:{key}"));
            }
        }
        contract.insert("team_id".to_owned(), json!(team));
        contract.insert(
            "start_date".to_owned(),
            json!(with_time(
                start,
                &scalar(&contract["start_date"]),
                "T00:00:00"
            )),
        );
        contract.insert(
            "end_date".to_owned(),
            json!(with_time(end, &scalar(&contract["end_date"]), "T23:59:59")),
        );
        contract.insert("weekly_salary".to_owned(), json!(annual / 52.0));
        if let Some(value) = transfer {
            contract.insert("transfer_fee".to_owned(), json!(value));
        }
        if let Some(value) = incentives.clone() {
            contract.insert("incentives".to_owned(), value);
        }
    }
    if let Some(value) = squad {
        record["squad_status"] = Value::String(value);
    }
    Ok(())
}

fn validate_contract_values(
    start: &str,
    end: &str,
    annual: f64,
    transfer: f64,
) -> Result<(), String> {
    if end < start {
        return Err("CONTRACT_END_BEFORE_START".to_owned());
    }
    if annual < 0.0 || transfer < 0.0 {
        return Err("CONTRACT_VALUE_OUT_OF_RANGE".to_owned());
    }
    Ok(())
}

fn read_record(ctx: &StableServerCtx<'_>, kind: RecordKindV1, id: usize) -> Result<Value, String> {
    serde_json::from_str(
        &ctx.record_get_json(kind, id, "")
            .ok_or("RECORD_NOT_FOUND")?,
    )
    .map_err(|_| "INVALID_RECORD_JSON".to_owned())
}

fn read_all(ctx: &StableServerCtx<'_>, kind: RecordKindV1) -> Result<Vec<(usize, Value)>, String> {
    ctx.record_ids(kind)
        .into_iter()
        .map(|id| read_record(ctx, kind, id).map(|record| (id, record)))
        .collect()
}

fn player_team(ctx: &StableServerCtx<'_>, state: &RuntimeState) -> Result<usize, String> {
    state
        .player_team_id(ctx)
        .ok_or_else(|| "PLAYER_TEAM_NOT_FOUND".to_owned())
}

fn object<'a>(value: &'a Value, key: &str) -> Result<&'a Map<String, Value>, String> {
    value
        .get(key)
        .and_then(Value::as_object)
        .ok_or_else(|| format!("{}_PATH_MISSING", key.to_uppercase()))
}

fn team_display_name(team: &Value, id: usize) -> String {
    ["display_name", "name", "short_name"]
        .into_iter()
        .map(|key| scalar(&team[key]))
        .find(|value| !value.is_empty())
        .unwrap_or_else(|| format!("Team {id}"))
}

#[derive(Default)]
struct ContractView {
    team_id: Option<usize>,
    weekly: String,
    annual: String,
    transfer_fee: String,
    start_date: String,
    end_date: String,
    body: Value,
}

impl From<&Value> for ContractView {
    fn from(record: &Value) -> Self {
        let Some(body) = active_contract(record) else {
            return Self::default();
        };
        let weekly = scalar(body.get("weekly_salary").unwrap_or(&Value::Null));
        let annual = weekly
            .parse::<f64>()
            .ok()
            .map(|value| (value * 52.0).to_string())
            .unwrap_or_default();
        Self {
            team_id: body
                .get("team_id")
                .and_then(Value::as_u64)
                .map(|value| value as usize),
            weekly,
            annual,
            transfer_fee: scalar(body.get("transfer_fee").unwrap_or(&Value::Null)),
            start_date: scalar(body.get("start_date").unwrap_or(&Value::Null)),
            end_date: scalar(body.get("end_date").unwrap_or(&Value::Null)),
            body: Value::Object(body.clone()),
        }
    }
}

fn active_contract(record: &Value) -> Option<&Map<String, Value>> {
    let contract = record.get("contract")?.as_object()?;
    contract
        .get("InContract")
        .and_then(Value::as_object)
        .or_else(|| contract.contains_key("team_id").then_some(contract))
}

fn active_contract_mut(record: &mut Value) -> Option<&mut Map<String, Value>> {
    let contract = record.get_mut("contract")?.as_object_mut()?;
    if contract.contains_key("InContract") {
        return contract.get_mut("InContract")?.as_object_mut();
    }
    contract.contains_key("team_id").then_some(contract)
}

fn scalar(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::String(value) => value.clone(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        value => serde_json::to_string(value).unwrap_or_default(),
    }
}

fn nested(value: &Value, keys: &[&str]) -> String {
    let mut current = value;
    for key in keys {
        let Some(next) = current.get(*key) else {
            return String::new();
        };
        current = next;
    }
    scalar(current)
}

fn primary_region(stat: &Map<String, Value>) -> String {
    stat.get("language")
        .and_then(Value::as_object)
        .and_then(|values| {
            values
                .iter()
                .max_by(|left, right| {
                    left.1
                        .as_f64()
                        .unwrap_or(0.0)
                        .total_cmp(&right.1.as_f64().unwrap_or(0.0))
                })
                .map(|(key, _)| key.clone())
        })
        .unwrap_or_default()
}

fn position_summary(stat: &Map<String, Value>) -> String {
    let mut best = None;
    for key in ["top", "jungle", "mid", "bottom", "support"] {
        let Some(value) = stat
            .get(key)
            .and_then(Value::as_f64)
            .filter(|value| *value > 0.0)
        else {
            continue;
        };
        if best.is_none_or(|(_, current)| value > current) {
            best = Some((key, value));
        }
    }
    best.map(|(key, _)| key.to_owned())
        .unwrap_or_else(|| "None".to_owned())
}

fn incentive_fields(contract: &Value) -> [String; 5] {
    let mut result: [String; 5] = Default::default();
    for incentive in contract
        .get("incentives")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        if let Some(value) = incentive.get("OnPog") {
            result[0] = scalar(&value["bonus"]);
        } else if let Some(value) = incentive.get("OnLeagueRank") {
            result[1] = scalar(&value["bonus"]);
            result[2] = scalar(&value["rank"]);
        } else if let Some(value) = incentive.get("OnMatch") {
            result[3] = scalar(&value["bonus"]);
        } else if let Some(value) = incentive.get("OnWin") {
            result[4] = scalar(&value["bonus"]);
        }
    }
    result
}

fn replace_existing(record: &mut Value, path: &[&str], value: Value) -> Result<(), String> {
    let (last, parents) = path.split_last().ok_or("INVALID_PATH")?;
    let mut current = record;
    for key in parents {
        current = current.get_mut(*key).ok_or("PATH_MISSING")?;
    }
    let object = current.as_object_mut().ok_or("PATH_MISSING")?;
    if !object.contains_key(*last) {
        return Err("PATH_MISSING".to_owned());
    }
    object.insert((*last).to_owned(), value);
    Ok(())
}

fn with_time(date: &str, current: &str, fallback: &str) -> String {
    if current.len() > 10 {
        format!("{date}{}", &current[10..])
    } else {
        format!("{date}{fallback}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contract_fields_can_promote_a_free_agent_without_touching_other_player_data() {
        let mut player = json!({
            "id": 4,
            "name": "Free Player",
            "contract": {"FreeAgent": {"requests": []}},
            "squad_status": "General"
        });
        apply_contract_fields(
            &mut player,
            7,
            "2026-01-01",
            "2027-12-31",
            5200.0,
            Some(300.0),
            Some("Important".to_owned()),
            Some(json!([])),
        )
        .unwrap();

        assert_eq!(player["contract"]["InContract"]["team_id"], 7);
        assert_eq!(player["contract"]["InContract"]["weekly_salary"], 100.0);
        assert_eq!(player["contract"]["InContract"]["transfer_fee"], 300.0);
        assert_eq!(
            player["contract"]["InContract"]["transfer_requests"],
            json!([])
        );
        assert_eq!(player["squad_status"], "Important");
        assert_eq!(player["name"], "Free Player");
    }
}
