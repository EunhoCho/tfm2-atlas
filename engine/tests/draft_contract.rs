use std::collections::BTreeMap;

use tfm2_atlas_engine::{
    analyze_role_gate, candidate_passes_role_gate, classify_role_profile, evaluate_draft_side,
    DraftCandidateInput, DraftPhase, DraftRoleConfidence, DraftScoreWeights, RoleFilter,
    RoleSample,
};

fn candidate() -> DraftCandidateInput {
    DraftCandidateInput {
        champion_id: "archer".to_owned(),
        op_score: 90.0,
        matchup_score: 80.0,
        synergy_score: 70.0,
        composition_score: 60.0,
        opponent_preference_score: 50.0,
        ally_threat_score: 40.0,
        projected_enemy_synergy_score: 30.0,
        projected_enemy_composition_score: 20.0,
        usable_roles: vec![RoleFilter::Bot],
        role_confidence: DraftRoleConfidence::High,
    }
}

#[test]
fn draft_defaults_match_the_approved_pick_and_ban_models() {
    let settings = DraftScoreWeights::default();

    assert_eq!(settings.pick_op, 40);
    assert_eq!(settings.pick_matchup, 20);
    assert_eq!(settings.pick_synergy, 20);
    assert_eq!(settings.pick_composition, 10);
    assert_eq!(settings.pick_denial, 10);
    assert!(settings.pick_role_gate);
    assert_eq!(settings.ban_preference, 40);
    assert_eq!(settings.ban_op, 60);
    assert!(!settings.ban_threat_enabled);
    assert!(!settings.ban_synergy_enabled);
    assert!(!settings.ban_composition_enabled);
    assert_eq!(settings.ban_threat, 10);
    assert_eq!(settings.ban_synergy, 10);
    assert_eq!(settings.ban_composition, 10);
    assert!(!settings.ban_role_gate);
}

#[test]
fn active_draft_weights_are_normalized_and_all_zero_is_rejected() {
    let mut settings = DraftScoreWeights::default();
    let pick = settings.normalized_pick().unwrap();
    assert_eq!(pick, [0.4, 0.2, 0.2, 0.1, 0.1]);

    settings.ban_threat_enabled = true;
    let ban = settings.normalized_ban().unwrap();
    assert!((ban[0] - 4.0 / 11.0).abs() < 1e-9);
    assert!((ban[1] - 6.0 / 11.0).abs() < 1e-9);
    assert!((ban[2] - 1.0 / 11.0).abs() < 1e-9);
    assert_eq!(ban[3], 0.0);
    assert_eq!(ban[4], 0.0);

    settings.pick_op = 0;
    settings.pick_matchup = 0;
    settings.pick_synergy = 0;
    settings.pick_composition = 0;
    settings.pick_denial = 0;
    assert!(settings.normalized_pick().is_err());
}

#[test]
fn pick_role_gate_rejects_known_mismatches_but_keeps_low_sample_candidates() {
    let settings = DraftScoreWeights::default();
    let known = candidate();
    assert!(settings
        .score_candidate(DraftPhase::Pick, &known, &[RoleFilter::Top])
        .is_none());

    let mut uncertain = known.clone();
    uncertain.role_confidence = DraftRoleConfidence::Low;
    let scored = settings
        .score_candidate(DraftPhase::Pick, &uncertain, &[RoleFilter::Top])
        .expect("low-sample candidates must pass the role gate");
    assert!(scored.low_confidence);
    assert_eq!(scored.total, 77.0);
}

#[test]
fn ban_extensions_use_projected_lineup_components_without_affecting_pick_scores() {
    let mut settings = DraftScoreWeights::default();
    let input = candidate();
    let base = settings
        .score_candidate(DraftPhase::Ban, &input, &[])
        .unwrap();
    assert_eq!(base.total, 74.0);

    settings.ban_threat_enabled = true;
    let extended = settings
        .score_candidate(DraftPhase::Ban, &input, &[])
        .unwrap();
    assert!((extended.total - 70.909090909).abs() < 0.0001);
    assert_eq!(extended.components.ally_threat, Some(40.0));
}

fn role_samples(values: &[(RoleFilter, usize, usize)]) -> BTreeMap<RoleFilter, RoleSample> {
    values
        .iter()
        .map(|(role, matches, wins)| {
            (
                *role,
                RoleSample {
                    matches: *matches,
                    wins: *wins,
                },
            )
        })
        .collect()
}

#[test]
fn role_profile_uses_the_75_and_two_40_percent_boundaries() {
    let dominant = classify_role_profile(
        "yone",
        &role_samples(&[(RoleFilter::Top, 75, 40), (RoleFilter::Mid, 25, 14)]),
        5,
        vec!["3.0".to_owned()],
    );
    assert_eq!(dominant.primary_roles, vec![RoleFilter::Top]);
    assert_eq!(dominant.roles[0].share, 75.0);

    let flex = classify_role_profile(
        "flex",
        &role_samples(&[
            (RoleFilter::Top, 45, 25),
            (RoleFilter::Mid, 40, 20),
            (RoleFilter::Jungle, 15, 7),
        ]),
        5,
        vec!["3.0".to_owned()],
    );
    assert_eq!(
        flex.primary_roles,
        vec![RoleFilter::Top, RoleFilter::Mid]
    );
}

#[test]
fn role_profile_uses_top_three_and_all_roles_for_zero_games() {
    let split = classify_role_profile(
        "split",
        &role_samples(&[
            (RoleFilter::Top, 34, 18),
            (RoleFilter::Jungle, 33, 17),
            (RoleFilter::Mid, 33, 16),
            (RoleFilter::Bot, 1, 1),
        ]),
        200,
        vec!["3.0".to_owned(), "2.1".to_owned()],
    );
    assert_eq!(
        split.primary_roles,
        vec![RoleFilter::Top, RoleFilter::Jungle, RoleFilter::Mid]
    );
    assert!(!split.sufficient);

    let unknown = classify_role_profile("new", &BTreeMap::new(), 5, vec!["3.0".to_owned()]);
    assert_eq!(
        unknown.primary_roles,
        vec![
            RoleFilter::Top,
            RoleFilter::Jungle,
            RoleFilter::Mid,
            RoleFilter::Bot,
            RoleFilter::Support,
        ]
    );
    assert_eq!(unknown.total_matches, 0);
}

#[test]
fn role_gate_enumerates_every_valid_flex_assignment() {
    let fixed = analyze_role_gate(&[
        vec![RoleFilter::Top, RoleFilter::Mid],
        vec![RoleFilter::Top, RoleFilter::Mid],
    ]);
    assert!(fixed.feasible);
    assert_eq!(
        fixed.definitely_filled,
        vec![RoleFilter::Top, RoleFilter::Mid]
    );
    assert!(!candidate_passes_role_gate(
        &[
            vec![RoleFilter::Top, RoleFilter::Mid],
            vec![RoleFilter::Top, RoleFilter::Mid],
        ],
        &[RoleFilter::Top],
    ));
    assert!(candidate_passes_role_gate(
        &[
            vec![RoleFilter::Top, RoleFilter::Mid],
            vec![RoleFilter::Top, RoleFilter::Mid],
        ],
        &[RoleFilter::Jungle],
    ));

    let conflict = analyze_role_gate(&[vec![RoleFilter::Top], vec![RoleFilter::Top]]);
    assert!(!conflict.feasible);
}

#[test]
fn current_draft_evaluation_uses_op_50_synergy_25_matchup_25() {
    let evaluation = evaluate_draft_side(&[80.0, 60.0], &[70.0], &[40.0], 1, 1)
        .expect("a side with picks is evaluated");
    assert_eq!(evaluation.op, 70.0);
    assert_eq!(evaluation.synergy, 70.0);
    assert_eq!(evaluation.matchup, 40.0);
    assert_eq!(evaluation.total, 62.5);
    assert_eq!(evaluation.data_coverage, 100.0);

    let partial = evaluate_draft_side(&[80.0], &[], &[], 0, 1).unwrap();
    assert_eq!(partial.synergy, 50.0);
    assert_eq!(partial.matchup, 50.0);
    assert_eq!(partial.total, 65.0);
    assert_eq!(partial.data_coverage, 50.0);
    assert!(evaluate_draft_side(&[], &[], &[], 0, 0).is_none());
}
