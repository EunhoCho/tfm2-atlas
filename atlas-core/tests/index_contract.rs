use tfm2_atlas_engine::{
    DivisionFilter, GameScope, TierProfile, RegionFilter, RoleFilter, SampleMode, Tier,
};
use tfm2_atlas_core::{Division, MatchKind, MatchIndex, MatchIndexCache, ReplayError};

fn tournament_replay(id: usize, version: &str, champion: &str, blue_win: bool) -> String {
    serde_json::json!({
        "id": id,
        "blue_team_id": 10,
        "red_team_id": 20,
        "blue_team_win": blue_win,
        "version": version,
        "blue_ban": ["bomber"],
        "red_ban": ["knight"],
        "blue_team": [{
            "position": "Top",
            "champion": champion,
            "items": [1, 2, 3],
            "statistics": {"dealing": 12000, "tanking": 8000, "healing": 0}
        }],
        "red_team": [{
            "position": "Top",
            "champion": "fighter",
            "items": [4, 5, 6],
            "statistics": {"dealing": 9000, "tanking": 10000, "healing": 500}
        }]
    })
    .to_string()
}

fn tournament_role_replay(id: usize, version: &str, champion: &str, position: &str) -> String {
    let mut replay: serde_json::Value =
        serde_json::from_str(&tournament_replay(id, version, champion, true)).unwrap();
    replay["blue_team"][0]["position"] = serde_json::Value::String(position.to_owned());
    replay.to_string()
}

fn solo_replay(id: usize, region_id: usize, version: &str, champion: &str) -> String {
    serde_json::json!({
        "id": id,
        "region_id": region_id,
        "blue_team_win": true,
        "version": version,
        "blue_team": [{"champion": champion, "items": ["t5_0", "t5_1", "t5_2"], "dealing": 5000, "tanking": 1000, "healing": 0}],
        "red_team": [{"champion": "fighter", "items": ["t5_3", "t5_4", "t5_5"], "dealing": 4000, "tanking": 2000, "healing": 0}]
    })
    .to_string()
}

#[test]
fn index_filters_tournament_by_region_division_role_and_latest_patch() {
    let mut index = MatchIndex::new();
    for id in 0..5 {
        index
            .record_tournament(
                &tournament_replay(id, "2026.1.0", "archer", true),
                RegionFilter::Kr,
                Division::First,
            )
            .unwrap();
    }
    for id in 5..10 {
        index
            .record_tournament(
                &tournament_replay(id, "2026.0.1", "archer", false),
                RegionFilter::Kr,
                Division::Second,
            )
            .unwrap();
    }
    index
        .record_tournament(
            &tournament_replay(11, "2026.1.0", "android", true),
            RegionFilter::Jp,
            Division::First,
        )
        .unwrap();

    let profile = TierProfile {
        scope: GameScope::Tournament,
        region: RegionFilter::Kr,
        division: DivisionFilter::First,
        role: RoleFilter::Top,
        sample: SampleMode::Minimum(5),
        ..TierProfile::default()
    };
    let state = index.preview(&profile).unwrap();
    let archer = state
        .champions
        .iter()
        .find(|row| row.champion_id == "archer")
        .unwrap();

    assert_eq!(state.selected_matches, 5);
    assert_eq!(state.latest_patch.as_deref(), Some("2026.1.0"));
    assert_eq!(state.available_patches, vec!["2026.1.0", "2026.0.1"]);
    assert_eq!(archer.sample, 5);
    assert!(archer.eligible);
    assert_ne!(archer.tier, Tier::NoTier);
    assert!(!state
        .champions
        .iter()
        .any(|row| row.champion_id == "android"));
    assert_eq!(archer.pick_count, 5);
    assert_eq!(archer.win_rate, Some(100.0));
    assert_eq!(archer.average_dealt, Some(12000.0));
    assert_eq!(archer.top_items[0].item_id, "1");
    assert_eq!(archer.top_items[0].games, 5);
    assert_eq!(state.replays.len(), 5);
    assert_eq!(state.replays[0].blue_champions, vec!["archer"]);
    assert_eq!(state.replays[0].red_champions, vec!["fighter"]);
    assert_eq!(state.replays[0].bans, vec!["bomber", "knight"]);
    let bomber = state
        .champions
        .iter()
        .find(|row| row.champion_id == "bomber")
        .expect("role-scoped scores retain tournament ban pressure");
    assert_eq!(bomber.ban_count, 5);
    assert_eq!(bomber.tier, Tier::NoTier);
    let archer_vs_fighter = archer
        .matchups
        .iter()
        .find(|row| row.champion_id == "fighter")
        .expect("matchup row");
    assert_eq!(archer_vs_fighter.games, 5);
    assert_eq!(archer_vs_fighter.win_rate, 100.0);

    let history_profile = TierProfile {
        division: DivisionFilter::All,
        ..profile
    };
    let history_state = index.preview(&history_profile).unwrap();
    let history = &history_state
        .champions
        .iter()
        .find(|row| row.champion_id == "archer")
        .unwrap()
        .patch_history;
    assert_eq!(history.len(), 2);
    assert_eq!(history[0].patch, "2026.1.0");
    assert_eq!((history[0].pick_count, history[0].wins), (5, 5));
    assert_eq!(history[1].patch, "2026.0.1");
    assert_eq!((history[1].pick_count, history[1].wins), (5, 0));
}

#[test]
fn solo_and_combined_scopes_use_region_but_not_division() {
    let mut index = MatchIndex::new();
    for id in 0..5 {
        index
            .record_solo(&solo_replay(100 + id, 0, "2026.1.0", "archer"))
            .unwrap();
    }
    for id in 0..5 {
        index
            .record_tournament(
                &tournament_replay(200 + id, "2026.1.0", "archer", false),
                RegionFilter::Kr,
                Division::Second,
            )
            .unwrap();
    }

    let solo = TierProfile {
        scope: GameScope::Solo,
        region: RegionFilter::Kr,
        division: DivisionFilter::All,
        sample: SampleMode::Minimum(5),
        ..TierProfile::default()
    };
    let solo_state = index.preview(&solo).unwrap();
    assert_eq!(solo_state.selected_matches, 5);
    let archer = solo_state
        .champions
        .iter()
        .find(|row| row.champion_id == "archer")
        .unwrap();
    assert_eq!(archer.top_items[0].item_id, "t5_0");

    let combined = TierProfile {
        scope: GameScope::SoloAndTournament,
        region: RegionFilter::Kr,
        division: DivisionFilter::All,
        sample: SampleMode::Minimum(5),
        ..TierProfile::default()
    };
    assert_eq!(index.preview(&combined).unwrap().selected_matches, 10);
}

#[test]
fn malformed_replay_is_rejected_without_changing_revision() {
    let mut index = MatchIndex::new();
    let before = index.revision();

    let error = index
        .record_json(MatchKind::Tournament, "{\"id\": 1}", None, None)
        .unwrap_err();

    assert!(matches!(error, ReplayError::MissingField(_)));
    assert_eq!(index.revision(), before);
    assert_eq!(index.match_count(), 0);
}

#[test]
fn lz4_cache_round_trip_uses_256_match_chunks_without_raw_replay_json() {
    let mut index = MatchIndex::new();
    for id in 0..300 {
        index
            .record_tournament(
                &tournament_replay(id, "2026.1.0", "archer", id % 2 == 0),
                RegionFilter::Kr,
                Division::First,
            )
            .unwrap();
    }

    let cache = MatchIndexCache::from_index(&index, "0.5.5").unwrap();

    assert_eq!(cache.manifest.chunk_size, 256);
    assert_eq!(cache.manifest.chunk_count, 2);
    assert_eq!(cache.manifest.match_count, 300);
    assert!(cache.chunks.iter().all(|chunk| {
        let encoded = String::from_utf8_lossy(chunk);
        !encoded.contains("\"blue_team\":[") && !encoded.contains("\"statistics\"")
    }));
    let restored = cache.restore("0.5.5").unwrap();
    assert_eq!(restored.match_count(), 300);
    assert_eq!(
        restored.preview(&TierProfile::default()).unwrap(),
        index.preview(&TierProfile::default()).unwrap()
    );
}

#[test]
fn cache_rejects_game_or_formula_version_drift_and_corrupt_chunks() {
    let mut index = MatchIndex::new();
    index
        .record_solo(&solo_replay(1, 0, "2026.1.0", "archer"))
        .unwrap();
    let cache = MatchIndexCache::from_index(&index, "0.5.5").unwrap();

    assert!(cache.restore("0.5.6").is_err());
    let mut corrupt = cache.clone();
    corrupt.chunks[0][0] ^= 0xff;
    assert!(corrupt.restore("0.5.5").is_err());
}

#[test]
fn selecting_different_patches_changes_samples_scores_and_tiers() {
    let mut index = MatchIndex::new();
    for id in 0..10 {
        index
            .record_tournament(
                &tournament_replay(id, "2026.1.0", "archer", true),
                RegionFilter::Kr,
                Division::First,
            )
            .unwrap();
        index
            .record_tournament(
                &tournament_replay(100 + id, "2026.0.1", "archer", false),
                RegionFilter::Kr,
                Division::First,
            )
            .unwrap();
    }
    let profile = TierProfile {
        scope: GameScope::Tournament,
        sample: SampleMode::Minimum(5),
        patch: Some("2026.1.0".to_owned()),
        ..TierProfile::default()
    };
    let new_patch = index.preview(&profile).unwrap();
    let old_patch = index
        .preview(&TierProfile {
            patch: Some("2026.0.1".to_owned()),
            ..profile
        })
        .unwrap();
    let new_archer = new_patch
        .champions
        .iter()
        .find(|row| row.champion_id == "archer")
        .unwrap();
    let old_archer = old_patch
        .champions
        .iter()
        .find(|row| row.champion_id == "archer")
        .unwrap();

    assert_eq!(new_patch.selected_matches, 10);
    assert_eq!(old_patch.selected_matches, 10);
    assert_ne!(new_archer.overall, old_archer.overall);
    assert_ne!(new_archer.tier, old_archer.tier);
}

#[test]
fn team_preferences_use_cached_team_ids_and_never_fall_back_to_other_patches() {
    let mut index = MatchIndex::new();
    for id in 0..5 {
        index
            .record_tournament(
                &tournament_replay(id, "2026.1.0", "archer", true),
                RegionFilter::Kr,
                Division::First,
            )
            .unwrap();
    }
    for id in 5..10 {
        index
            .record_tournament(
                &tournament_replay(id, "2025.9.0", "old_patch_only", true),
                RegionFilter::Kr,
                Division::First,
            )
            .unwrap();
    }
    let profile = TierProfile {
        scope: GameScope::Tournament,
        region: RegionFilter::Kr,
        patch: Some("2026.1.0".to_owned()),
        ..TierProfile::default()
    };

    let preferences = index.team_pick_preferences(&profile, 10).unwrap();
    assert_eq!(preferences["archer"].picks, 5);
    assert!(!preferences.contains_key("old_patch_only"));
    assert!(index
        .team_pick_preferences(&profile, 999)
        .unwrap()
        .is_empty());
}

#[test]
fn team_preferences_accept_official_wrapped_team_references() {
    let mut index = MatchIndex::new();
    let replay = serde_json::json!({
        "id": 91,
        "blue_team_id": {"Normal": 10},
        "red_team_id": {"Normal": 20},
        "blue_team_win": true,
        "version": "2026.2.0",
        "blue_ban": [],
        "red_ban": [],
        "blue_team": [{
            "position": "Top",
            "champion": "archer",
            "items": [],
            "statistics": {"dealing": 1, "tanking": 1, "healing": 0}
        }],
        "red_team": [{
            "position": "Top",
            "champion": "fighter",
            "items": [],
            "statistics": {"dealing": 1, "tanking": 1, "healing": 0}
        }]
    });
    index
        .record_tournament(&replay.to_string(), RegionFilter::Kr, Division::First)
        .unwrap();

    let profile = TierProfile {
        scope: GameScope::Tournament,
        region: RegionFilter::Kr,
        patch: Some("2026.2.0".to_owned()),
        ..TierProfile::default()
    };
    let preferences = index.team_pick_preferences(&profile, 10).unwrap();

    assert_eq!(preferences["archer"].team_matches, 1);
    assert!(preferences["archer"].score > 50.0);
}

#[test]
fn role_profiles_backfill_older_patches_only_until_the_profile_sample_floor() {
    let mut index = MatchIndex::new();
    for id in 0..2 {
        index
            .record_tournament(
                &tournament_role_replay(id, "3.0", "yone", "Top"),
                RegionFilter::Kr,
                Division::First,
            )
            .unwrap();
    }
    for id in 2..5 {
        index
            .record_tournament(
                &tournament_role_replay(id, "2.1", "yone", "Mid"),
                RegionFilter::Kr,
                Division::First,
            )
            .unwrap();
    }
    for id in 5..10 {
        index
            .record_tournament(
                &tournament_role_replay(id, "2.0", "yone", "Jungle"),
                RegionFilter::Kr,
                Division::First,
            )
            .unwrap();
    }
    let profile = TierProfile {
        scope: GameScope::Tournament,
        region: RegionFilter::Kr,
        division: DivisionFilter::First,
        role: RoleFilter::Top,
        patch: Some("3.0".to_owned()),
        sample: SampleMode::Minimum(5),
        ..TierProfile::default()
    };

    let state = index.preview(&profile).unwrap();
    let yone = state
        .champions
        .iter()
        .find(|row| row.champion_id == "yone")
        .unwrap();

    assert_eq!(yone.role_profile.total_matches, 5);
    assert_eq!(yone.role_profile.used_patches, vec!["3.0", "2.1"]);
    assert_eq!(
        yone.role_profile.primary_roles,
        vec![RoleFilter::Mid, RoleFilter::Top]
    );
    assert!(yone.role_profile.sufficient);
    assert_eq!(
        yone.role_profile
            .roles
            .iter()
            .find(|row| row.role == RoleFilter::Mid)
            .unwrap()
            .share,
        60.0
    );
}
