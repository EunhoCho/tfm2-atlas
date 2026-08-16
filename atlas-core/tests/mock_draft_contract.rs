use tfm2_atlas_engine::DraftPhase;
use tfm2_atlas_core::{
    DraftRule, DraftSide, MockDraftAction, MockDraftContextUpdate, MockDraftSession,
};

fn action(side: DraftSide, phase: DraftPhase, champion_id: &str) -> MockDraftAction {
    MockDraftAction {
        side,
        phase,
        champion_id: champion_id.to_owned(),
    }
}

fn catalog(count: usize) -> Vec<String> {
    (0..count)
        .map(|index| format!("champion_{index}"))
        .collect()
}

#[test]
fn manual_actions_enforce_uniqueness_and_three_ban_five_pick_caps() {
    let active = catalog(12);
    let mut session = MockDraftSession::default();

    for champion in &active[0..3] {
        session
            .apply(action(DraftSide::Blue, DraftPhase::Ban, champion), &active)
            .unwrap();
    }
    assert_eq!(
        session
            .apply(
                action(DraftSide::Blue, DraftPhase::Ban, &active[3]),
                &active
            )
            .unwrap_err()
            .code(),
        "draft_slot_full"
    );
    assert_eq!(
        session
            .apply(
                action(DraftSide::Red, DraftPhase::Pick, &active[0]),
                &active
            )
            .unwrap_err()
            .code(),
        "champion_already_selected"
    );
}

#[test]
fn fearless_and_hard_fearless_exclude_the_correct_previous_set_picks() {
    let active = catalog(10);
    let mut session = MockDraftSession::default();
    session
        .apply(
            action(DraftSide::Blue, DraftPhase::Pick, &active[0]),
            &active,
        )
        .unwrap();
    session
        .apply(
            action(DraftSide::Red, DraftPhase::Pick, &active[1]),
            &active,
        )
        .unwrap();
    session
        .set_context(MockDraftContextUpdate {
            current_set: Some(2),
            rule: Some(DraftRule::Fearless),
            ..MockDraftContextUpdate::default()
        })
        .unwrap();

    assert_eq!(
        session
            .apply(
                action(DraftSide::Blue, DraftPhase::Pick, &active[0]),
                &active
            )
            .unwrap_err()
            .code(),
        "champion_fearless_excluded"
    );
    session
        .apply(
            action(DraftSide::Blue, DraftPhase::Pick, &active[1]),
            &active,
        )
        .unwrap();

    session.reset_current_set();
    session
        .set_context(MockDraftContextUpdate {
            rule: Some(DraftRule::HardFearless),
            ..MockDraftContextUpdate::default()
        })
        .unwrap();
    assert_eq!(
        session
            .apply(
                action(DraftSide::Blue, DraftPhase::Ban, &active[1]),
                &active
            )
            .unwrap_err()
            .code(),
        "champion_fearless_excluded"
    );
}

#[test]
fn undo_and_reset_restore_only_the_current_career_session_state() {
    let active = catalog(4);
    let mut session = MockDraftSession::default();
    session
        .apply(
            action(DraftSide::Blue, DraftPhase::Pick, &active[0]),
            &active,
        )
        .unwrap();
    session
        .apply(action(DraftSide::Red, DraftPhase::Ban, &active[1]), &active)
        .unwrap();
    assert!(session.undo());
    assert!(session.state().sets[0].red_bans.is_empty());

    session.reset_current_set();
    assert!(session.state().sets[0].blue_picks.is_empty());
    assert!(session.undo());
    assert_eq!(session.state().sets[0].blue_picks, vec![active[0].clone()]);
}

#[test]
fn changing_player_side_keeps_physical_blue_and_red_slots() {
    let active = catalog(2);
    let mut session = MockDraftSession::default();
    session
        .apply(
            action(DraftSide::Blue, DraftPhase::Pick, &active[0]),
            &active,
        )
        .unwrap();
    session
        .set_context(MockDraftContextUpdate {
            player_side: Some(DraftSide::Red),
            ..MockDraftContextUpdate::default()
        })
        .unwrap();

    assert_eq!(session.state().sets[0].our_side, DraftSide::Red);
    assert_eq!(session.state().sets[0].blue_picks, vec![active[0].clone()]);
    assert!(session.state().sets[0].red_picks.is_empty());
}

#[test]
fn each_set_keeps_its_own_player_side() {
    let active = catalog(1);
    let mut session = MockDraftSession::default();
    session
        .set_context(MockDraftContextUpdate {
            current_set: Some(2),
            player_side: Some(DraftSide::Red),
            ..MockDraftContextUpdate::default()
        })
        .unwrap();

    assert_eq!(session.state().sets[0].our_side, DraftSide::Blue);
    assert_eq!(session.state().sets[1].our_side, DraftSide::Red);
    session
        .apply(
            action(DraftSide::Red, DraftPhase::Pick, &active[0]),
            &active,
        )
        .unwrap();
    assert!(session.reset_current_set());
    assert_eq!(session.state().sets[1].our_side, DraftSide::Red);

    session
        .set_context(MockDraftContextUpdate {
            current_set: Some(1),
            ..MockDraftContextUpdate::default()
        })
        .unwrap();
    assert_eq!(session.state().sets[0].our_side, DraftSide::Blue);
}

#[test]
fn fearless_follows_team_identity_when_sides_swap_between_sets() {
    let active = catalog(4);
    let mut session = MockDraftSession::default();
    session
        .apply(
            action(DraftSide::Blue, DraftPhase::Pick, &active[0]),
            &active,
        )
        .unwrap();
    session
        .apply(
            action(DraftSide::Red, DraftPhase::Pick, &active[1]),
            &active,
        )
        .unwrap();
    session
        .set_context(MockDraftContextUpdate {
            current_set: Some(2),
            player_side: Some(DraftSide::Red),
            rule: Some(DraftRule::Fearless),
            ..MockDraftContextUpdate::default()
        })
        .unwrap();

    assert_eq!(
        session
            .apply(
                action(DraftSide::Red, DraftPhase::Pick, &active[0]),
                &active
            )
            .unwrap_err()
            .code(),
        "champion_fearless_excluded"
    );
    assert_eq!(
        session
            .apply(
                action(DraftSide::Blue, DraftPhase::Pick, &active[1]),
                &active
            )
            .unwrap_err()
            .code(),
        "champion_fearless_excluded"
    );
}
