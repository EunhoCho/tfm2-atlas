use std::collections::BTreeMap;

use tfm2_atlas_engine::{
    apply_lock_to_record, apply_tiers_to_team, decode_json_frame, encode_json_frame,
    parse_tier_tsv, render_tier_tsv_v2, ChampionInput, DivisionFilter, FrameError, GameScope,
    LockGroup, LockSet, TierEngine, TierPreset, TierProfile, RecordPatchError, RegionFilter,
    RoleFilter, SampleMode, Tier, TierPolicyRow, PLAYER_STAT_FIELDS,
};

#[test]
fn default_profile_matches_product_contract() {
    let profile = TierProfile::default();

    assert!(profile.enabled);
    assert_eq!(profile.scope, GameScope::SoloAndTournament);
    assert_eq!(profile.region, RegionFilter::All);
    assert_eq!(profile.division, DivisionFilter::All);
    assert_eq!(profile.role, RoleFilter::All);
    assert_eq!(profile.sample, SampleMode::Auto);
    assert_eq!(profile.preset, TierPreset::Classic);
    assert_eq!(profile.patch.as_deref(), Some("latest"));
    assert!(profile.validate().is_ok());
}

#[test]
fn division_filter_is_only_valid_for_tournament_scope() {
    let mut profile = TierProfile::default();
    profile.division = DivisionFilter::First;

    let error = profile.validate().unwrap_err();
    assert_eq!(error.code(), "division_requires_tournament");

    profile.scope = GameScope::Tournament;
    assert!(profile.validate().is_ok());
}

#[test]
fn role_aware_v6_marks_thin_samples_no_tier_and_scores_eligible_rows() {
    let champions = vec![
        ChampionInput::new("thin", 4, 4, 0, Some(10.0), None, None),
        ChampionInput::new("steady", 20, 12, 2, Some(40.0), Some(4.0), Some(44.0)),
        ChampionInput::new("popular", 20, 10, 12, Some(60.0), Some(24.0), Some(84.0)),
        ChampionInput::new("weak", 20, 6, 1, Some(20.0), Some(2.0), Some(22.0)),
    ];

    let scores = TierEngine::role_aware_v6().score(&champions);

    assert_eq!(scores["thin"].tier, Tier::NoTier);
    assert!(!scores["thin"].eligible);
    assert_eq!(scores["thin"].reason.as_deref(), Some("sample_too_small"));

    assert!(scores["steady"].eligible);
    assert!((scores["steady"].overall.unwrap() - 83.3).abs() <= 0.1);
    assert_eq!(scores["steady"].tier, Tier::One);

    assert!(scores["popular"].eligible);
    assert!((scores["popular"].overall.unwrap() - 55.0).abs() <= 0.1);
    assert_eq!(scores["popular"].tier, Tier::Two);

    assert!(scores["weak"].eligible);
    assert!((scores["weak"].overall.unwrap() - 6.7).abs() <= 0.1);
    assert_eq!(scores["weak"].tier, Tier::Four);
}

#[test]
fn fearless_presets_change_weights_without_changing_classic_scores() {
    let mut flexible = ChampionInput::new("flexible", 25, 15, 3, Some(50.0), Some(6.0), Some(56.0));
    flexible.by_position = BTreeMap::from([
        (
            RoleFilter::Top,
            tfm2_atlas_engine::RoleSample {
                matches: 12,
                wins: 7,
            },
        ),
        (
            RoleFilter::Mid,
            tfm2_atlas_engine::RoleSample {
                matches: 13,
                wins: 8,
            },
        ),
    ]);
    let specialist = ChampionInput::new("specialist", 25, 15, 3, Some(50.0), Some(6.0), Some(56.0));
    let champions = vec![flexible, specialist];

    let classic = TierEngine::role_aware_v6_for_preset(TierPreset::Classic).score(&champions);
    let default_classic = TierEngine::role_aware_v6().score(&champions);
    let fearless = TierEngine::role_aware_v6_for_preset(TierPreset::Fearless).score(&champions);
    let hard = TierEngine::role_aware_v6_for_preset(TierPreset::HardFearless).score(&champions);

    assert_eq!(classic, default_classic);
    assert_ne!(classic["flexible"].overall, fearless["flexible"].overall);
    assert_ne!(fearless["flexible"].overall, hard["flexible"].overall);
}

#[test]
fn lock_groups_validate_complete_player_and_staff_payloads() {
    let player = LockSet::new(42, "Player 42", LockGroup::PlayerStats, vec![70.0; 12]);
    let staff = LockSet::new(7, "Staff 7", LockGroup::StaffStats, vec![55.0; 10]);
    let mastery = LockSet::new_mastery(
        42,
        "Player 42",
        vec![("archer".to_owned(), 90.0), ("knight".to_owned(), 80.0)],
    );

    assert!(player.validate().is_ok());
    assert!(staff.validate().is_ok());
    assert!(mastery.validate().is_ok());

    let incomplete = LockSet::new(42, "Player 42", LockGroup::PlayerStats, vec![70.0; 11]);
    assert_eq!(
        incomplete.validate().unwrap_err().code(),
        "invalid_group_size"
    );

    let out_of_range = LockSet::new(7, "Staff 7", LockGroup::StaffStats, vec![101.0; 10]);
    assert_eq!(
        out_of_range.validate().unwrap_err().code(),
        "value_out_of_range"
    );
}

#[test]
fn record_patch_updates_only_the_selected_lock_group() {
    let mut player = serde_json::json!({
        "id": 42,
        "name": "Player 42",
        "stat": {
            "last_hit": 10, "skill_avoid": 11, "skill_hit": 12,
            "control_speed": 13, "positioning": 14, "judgement": 15,
            "mental": 16, "concentration": 17, "order": 18,
            "roaming": 19, "aggressive": 20, "ego": 21,
            "top": 88
        },
        "management": {"condition": 61}
    });
    let lock = LockSet::new(42, "Player 42", LockGroup::PlayerStats, vec![77.0; 12]);

    apply_lock_to_record(&mut player, &lock).unwrap();

    assert_eq!(player["stat"]["last_hit"], 77);
    assert!(player["stat"]["last_hit"].as_i64().is_some());
    assert_eq!(player["stat"]["ego"], 77);
    assert_eq!(player["stat"]["top"], 88);
    assert_eq!(player["management"]["condition"], 61);
}

#[test]
fn mastery_patch_scales_display_values_to_raw_proficiency_and_preserves_metadata() {
    let mut player = serde_json::json!({
        "id": 42,
        "champion_proficiency": {
            "archer": {"value": 120, "floor": 100},
            "knight": {"value": 500, "floor": 400}
        }
    });
    let lock = LockSet::new_mastery(
        42,
        "Player 42",
        vec![("archer".to_owned(), 90.0), ("knight".to_owned(), 80.0)],
    );

    apply_lock_to_record(&mut player, &lock).unwrap();

    assert_eq!(player["champion_proficiency"]["archer"]["value"], 900);
    assert!(player["champion_proficiency"]["archer"]["value"]
        .as_i64()
        .is_some());
    assert_eq!(player["champion_proficiency"]["archer"]["floor"], 100);
    assert_eq!(player["champion_proficiency"]["knight"]["value"], 800);
}

#[test]
fn mastery_patch_can_initialize_a_current_patch_champion_missing_from_an_old_athlete_record() {
    let mut player = serde_json::json!({
        "id": 42,
        "champion_proficiency": {
            "archer": {"value": 120, "floor": 100}
        }
    });
    let lock = LockSet::new_mastery(42, "Player 42", vec![("new_mod_champion".to_owned(), 75.0)]);

    apply_lock_to_record(&mut player, &lock).unwrap();

    assert_eq!(
        player["champion_proficiency"]["new_mod_champion"]["value"],
        750
    );
    assert_eq!(
        player["champion_proficiency"]["new_mod_champion"]["floor"],
        0
    );
}

#[test]
fn player_profile_lock_updates_every_requested_live_field_atomically() {
    let mut player = serde_json::json!({
        "id": 42,
        "age": 19,
        "stat": {
            "last_hit": 10, "skill_avoid": 11, "skill_hit": 12,
            "control_speed": 13, "positioning": 14, "judgement": 15,
            "mental": 16, "concentration": 17, "order": 18,
            "roaming": 19, "aggressive": 20, "ego": 21,
            "top": 100, "jungle": 0, "mid": 80, "bottom": 0, "support": 0,
            "language": {"1": 30, "2": 40}
        },
        "hidden": {"potential": 72},
        "management": {"stamina": 64, "stress": 12, "condition": 55},
        "champion_proficiency": {
            "archer": {"value": 120, "floor": 100},
            "knight": {"value": 500, "floor": 400}
        },
        "contract": {"InContract": {"team_id": 7}}
    });
    let original_contract = player["contract"].clone();
    let mut values = PLAYER_STAT_FIELDS
        .iter()
        .map(|key| (format!("stat.{key}"), 77.0))
        .collect::<Vec<_>>();
    values.extend([
        ("stat.top".to_owned(), 90.0),
        ("stat.jungle".to_owned(), 0.0),
        ("stat.mid".to_owned(), 80.0),
        ("stat.bottom".to_owned(), 0.0),
        ("stat.support".to_owned(), 0.0),
        ("hidden.potential".to_owned(), 88.0),
        ("management.stamina".to_owned(), 25.0),
        ("management.stress".to_owned(), 35.0),
        ("management.condition".to_owned(), 99.0),
        ("age".to_owned(), 24.0),
        ("stat.language.2".to_owned(), 100.0),
        ("champion_proficiency.archer".to_owned(), 91.0),
        ("champion_proficiency.knight".to_owned(), 82.0),
    ]);
    let lock = LockSet::new_keyed(42, "Player 42", LockGroup::PlayerProfile, values);

    apply_lock_to_record(&mut player, &lock).unwrap();

    assert_eq!(player["stat"]["last_hit"], 77);
    assert_eq!(player["stat"]["top"], 90);
    assert_eq!(player["hidden"]["potential"], 88);
    assert_eq!(player["management"]["stamina"], 25);
    assert_eq!(player["management"]["stress"], 35);
    assert_eq!(player["management"]["condition"], 99);
    assert_eq!(player["age"], 24);
    assert_eq!(player["stat"]["language"], serde_json::json!({"2": 100}));
    assert_eq!(player["champion_proficiency"]["archer"]["value"], 910);
    assert_eq!(player["champion_proficiency"]["archer"]["floor"], 100);
    assert_eq!(player["contract"], original_contract);
}

#[test]
fn profile_locks_reject_paths_outside_the_editor_owned_fields() {
    let mut values = PLAYER_STAT_FIELDS
        .iter()
        .map(|key| (format!("stat.{key}"), 77.0))
        .collect::<Vec<_>>();
    values.extend([
        ("stat.top".to_owned(), 100.0),
        ("stat.jungle".to_owned(), 0.0),
        ("stat.mid".to_owned(), 0.0),
        ("stat.bottom".to_owned(), 0.0),
        ("stat.support".to_owned(), 0.0),
        ("hidden.potential".to_owned(), 80.0),
        ("management.stamina".to_owned(), 50.0),
        ("management.stress".to_owned(), 50.0),
        ("management.condition".to_owned(), 50.0),
        ("age".to_owned(), 22.0),
        ("contract.InContract.team_id".to_owned(), 99.0),
    ]);
    let lock = LockSet::new_keyed(42, "Player 42", LockGroup::PlayerProfile, values);

    assert_eq!(lock.validate().unwrap_err().code(), "invalid_value_key");
}

#[test]
fn staff_profile_lock_updates_stats_age_and_exact_communication_map() {
    let mut staff = serde_json::json!({
        "id": 7,
        "age": 44,
        "stat": {
            "banpick": 10, "strategy": 11, "negotiation": 12,
            "judge_ability": 13, "judge_potential": 14, "feedback": 15,
            "power_analysis": 16, "control_coaching": 17,
            "judgment_coaching": 18, "mental_coaching": 19
        },
        "language": {"0": 20, "4": 50},
        "contract": {"InContract": {"team_id": 2}}
    });
    let original_contract = staff["contract"].clone();
    let mut values = tfm2_atlas_engine::STAFF_STAT_FIELDS
        .iter()
        .map(|key| (format!("stat.{key}"), 66.0))
        .collect::<Vec<_>>();
    values.extend([
        ("age".to_owned(), 39.0),
        ("language.1".to_owned(), 90.0),
        ("language.5".to_owned(), 100.0),
    ]);
    let lock = LockSet::new_keyed(7, "Staff 7", LockGroup::StaffProfile, values);

    apply_lock_to_record(&mut staff, &lock).unwrap();

    assert_eq!(staff["stat"]["banpick"], 66);
    assert_eq!(staff["age"], 39);
    assert_eq!(staff["language"], serde_json::json!({"1": 90, "5": 100}));
    assert_eq!(staff["contract"], original_contract);
}

#[test]
fn integer_record_fields_reject_fractional_lock_values() {
    let mut player = serde_json::json!({
        "id": 42,
        "stat": {
            "last_hit": 10, "skill_avoid": 11, "skill_hit": 12,
            "control_speed": 13, "positioning": 14, "judgement": 15,
            "mental": 16, "concentration": 17, "order": 18,
            "roaming": 19, "aggressive": 20, "ego": 21
        }
    });
    let lock = LockSet::new(42, "Player 42", LockGroup::PlayerStats, vec![77.5; 12]);

    let error = apply_lock_to_record(&mut player, &lock).unwrap_err();

    assert!(matches!(error, RecordPatchError::InvalidType { .. }));
}

#[test]
fn record_patch_rejects_schema_drift_without_mutating_the_record() {
    let mut player = serde_json::json!({"id": 42, "stat": {"last_hit": 10}});
    let original = player.clone();
    let lock = LockSet::new(42, "Player 42", LockGroup::PlayerStats, vec![77.0; 12]);

    let error = apply_lock_to_record(&mut player, &lock).unwrap_err();

    assert!(matches!(error, RecordPatchError::MissingPath { .. }));
    assert_eq!(player, original);
}

#[test]
fn tier_patch_maps_dashboard_tiers_and_explicitly_writes_no_tier() {
    let mut team = serde_json::json!({
        "id": 9,
        "name": "Player Team",
        "champion_tiers": {"archer": "D", "knight": "A", "legacy": "B"}
    });
    let rows = vec![
        TierPolicyRow::new("archer", Tier::Op, Some(90.0), true),
        TierPolicyRow::new("knight", Tier::NoTier, None, false),
    ];

    apply_tiers_to_team(&mut team, &rows).unwrap();

    assert_eq!(team["champion_tiers"]["archer"], "S");
    assert_eq!(team["champion_tiers"]["knight"], "NoTier");
    assert_eq!(team["champion_tiers"]["legacy"], "B");
}

#[test]
fn json_frames_round_trip_and_reject_oversized_payloads() {
    let value = serde_json::json!({"command": "GET_DASHBOARD", "한국어": true});
    let encoded = encode_json_frame(&value).unwrap();

    assert_eq!(
        u32::from_be_bytes(encoded[..4].try_into().unwrap()) as usize,
        encoded.len() - 4
    );
    assert_eq!(
        decode_json_frame::<serde_json::Value>(&encoded).unwrap(),
        value
    );

    let oversized = vec![0_u8; tfm2_atlas_engine::MAX_FRAME_BYTES + 5];
    assert!(matches!(
        decode_json_frame::<serde_json::Value>(&oversized),
        Err(FrameError::TooLarge { .. })
    ));
}

#[test]
fn tier_tsv_v2_round_trips_and_v1_defaults_to_eligible() {
    let rows = vec![
        TierPolicyRow::new("archer", Tier::One, Some(72.4), true),
        TierPolicyRow::new("knight", Tier::NoTier, None, false),
    ];
    let rendered = render_tier_tsv_v2(&rows);

    assert!(rendered.starts_with("champion_id\ttier\toverall\teligible\n"));
    assert_eq!(parse_tier_tsv(&rendered).unwrap(), rows);

    let legacy = "champion_id\ttier\toverall\narcher\t1\t72.4\n";
    assert_eq!(
        parse_tier_tsv(legacy).unwrap(),
        vec![TierPolicyRow::new("archer", Tier::One, Some(72.4), true)]
    );
}

#[test]
fn tier_tsv_rejects_duplicate_champions_and_non_finite_scores() {
    let duplicate =
        "champion_id\ttier\toverall\teligible\narcher\t1\t72.4\ttrue\narcher\t2\t61.0\ttrue\n";
    assert!(parse_tier_tsv(duplicate).is_err());

    let non_finite = "champion_id\ttier\toverall\teligible\narcher\t1\tNaN\ttrue\n";
    assert!(parse_tier_tsv(non_finite).is_err());
}

#[test]
fn profile_serialization_uses_stable_snake_case_wire_values() {
    let mut profile = TierProfile::default();
    profile.preset = TierPreset::HardFearless;
    let wire = serde_json::to_value(profile).unwrap();

    let expected = BTreeMap::from([
        ("preset", serde_json::json!("hard_fearless")),
        ("scope", serde_json::json!("solo_and_tournament")),
    ]);
    for (key, value) in expected {
        assert_eq!(wire[key], value);
    }
}
