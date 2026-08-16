use serde_json::json;
use tfm2_atlas_core::validate_community_shapes;

fn fixtures() -> (serde_json::Value, serde_json::Value, serde_json::Value) {
    let athlete = json!({
        "id": 1, "name": "Player", "age": 20,
        "stat": {
            "last_hit": 50, "skill_avoid": 50, "skill_hit": 50, "control_speed": 50,
            "positioning": 50, "judgement": 50, "mental": 50, "concentration": 50,
            "order": 50, "roaming": 50, "aggressive": 50, "ego": 50,
            "top": 100, "jungle": 0, "mid": 0, "bottom": 0, "support": 0,
            "language": {"0": 100}
        },
        "hidden": {"potential": 60},
        "contract": {"InContract": {"team_id": 0, "weekly_salary": 1, "start_date": "2026-01-01", "end_date": "2027-01-01", "transfer_fee": 0, "incentives": [], "transfer_requests": [], "recruit_requests": []}},
        "champion_proficiency": {"champion": {"value": 500}},
        "management": {"stamina": 100, "stress": 17, "condition": 50},
        "training_exp": {"language_by_region": {}}
    });
    let staff = json!({
        "id": 2, "name": "Coach", "age": 40, "role": "HeadCoach",
        "stat": {"banpick": 50, "strategy": 50, "negotiation": 50, "judge_ability": 50, "judge_potential": 50, "feedback": 50, "power_analysis": 50, "control_coaching": 50, "judgment_coaching": 50, "mental_coaching": 50},
        "language": {"0": 100},
        "contract": {"InContract": {"team_id": 0, "weekly_salary": 1, "start_date": "2026-01-01", "end_date": "2027-01-01", "transfer_requests": [], "recruit_requests": []}}
    });
    let team = json!({
        "id": 0, "name": "Team", "manager_name": "Manager", "league_id": 0,
        "champion_tiers": {"champion": "A"}, "total_balance": 1,
        "transfer_budget": 1, "salary_budget": 1, "training_facility_grade": "S",
        "merchandise_facility_grade": "A", "stadium": {"grade": "B"}
    });
    (athlete, staff, team)
}

#[test]
fn all_public_community_paths_are_preflighted() {
    let (athlete, staff, team) = fixtures();
    assert!(validate_community_shapes(&athlete, &staff, &team).is_ok());

    let (athlete, staff, mut team) = fixtures();
    team.as_object_mut().unwrap().remove("salary_budget");
    let failures = validate_community_shapes(&athlete, &staff, &team).unwrap_err();
    assert!(failures
        .iter()
        .any(|failure| failure == "Team.salary_budget"));

    let (mut athlete, staff, team) = fixtures();
    athlete["stat"]["last_hit"] = json!("50");
    let failures = validate_community_shapes(&athlete, &staff, &team).unwrap_err();
    assert!(failures
        .iter()
        .any(|failure| failure == "Athlete.stat.last_hit (expected number)"));
}

#[test]
fn tfm2_0_5_5_facility_grades_are_tier_strings() {
    let (athlete, staff, team) = fixtures();

    assert!(
        validate_community_shapes(&athlete, &staff, &team).is_ok(),
        "0.5.5 serializes facility grades as tier names such as S, A, and B"
    );
}
