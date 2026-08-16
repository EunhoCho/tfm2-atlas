use tfm2_atlas_core::{ApiCommand, ApiRequest, ApiResponse, ApiSurface};

#[test]
fn api_accepts_current_commands_without_a_protocol_field() {
    let request: ApiRequest = serde_json::from_value(serde_json::json!({
        "request_id": 77,
        "command": "PREVIEW_TIER_PROFILE",
        "payload": {"enabled": true}
    }))
    .unwrap();

    assert_eq!(request.command, ApiCommand::PreviewTierProfile);
    assert!(request.validate().is_ok());

    let response = ApiResponse::ok(request.request_id, serde_json::json!({"preview": true}));
    assert_eq!(response.request_id, 77);
    assert!(response.ok);
    assert!(response.error.is_none());
    let json = serde_json::to_value(response).unwrap();
    assert!(json.get("protocol").is_none());
}

#[test]
fn legacy_protocol_and_command_names_are_not_accepted() {
    assert!(serde_json::from_value::<ApiRequest>(serde_json::json!({
        "protocol": 10,
        "request_id": 5,
        "command": "GET_DASHBOARD_STATE_V10",
        "payload": {}
    }))
    .is_err());
}

#[test]
fn error_response_has_stable_localizable_code() {
    let response = ApiResponse::error(9, "career_not_connected", "Career is not connected");

    assert!(!response.ok);
    assert_eq!(response.error.unwrap().code, "career_not_connected");
    assert!(response.data.is_none());
}

#[test]
fn api_exposes_the_manual_mock_draft_commands() {
    for (command, expected) in [
        ("GET_DASHBOARD", ApiCommand::GetDashboard),
        ("PREVIEW_TIER_PROFILE", ApiCommand::PreviewTierProfile),
        ("APPLY_TIER_PROFILE", ApiCommand::ApplyTierProfile),
        ("EXPORT_TIER_TSV", ApiCommand::ExportTierTsv),
        ("VALIDATE_TIER_TSV", ApiCommand::ValidateTierTsv),
        ("GET_CATALOG", ApiCommand::GetChampionCatalog),
        ("GET_EDITOR_DATA", ApiCommand::GetEditorData),
        ("APPLY_EDITOR_SETTINGS", ApiCommand::ApplyEditorSettings),
        ("GET_PLAYER_MASTERY", ApiCommand::GetPlayerMastery),
        ("SET_PLAYER_MASTERY", ApiCommand::SetPlayerMastery),
        ("APPLY_PLAYER_EDIT", ApiCommand::ApplyPlayerEdit),
        ("APPLY_STAFF_EDIT", ApiCommand::ApplyStaffEdit),
        ("MOVE_PLAYER", ApiCommand::MovePlayer),
        ("GET_MOCK_DRAFT", ApiCommand::GetMockDraft),
        ("SET_MOCK_DRAFT_CONTEXT", ApiCommand::SetMockDraftContext),
        ("APPLY_MOCK_DRAFT_ACTION", ApiCommand::ApplyMockDraftAction),
        (
            "REMOVE_MOCK_DRAFT_ACTION",
            ApiCommand::RemoveMockDraftAction,
        ),
        ("UNDO_MOCK_DRAFT", ApiCommand::UndoMockDraft),
        ("RESET_MOCK_DRAFT_SET", ApiCommand::ResetMockDraftSet),
        ("SET_DRAFT_SETTINGS", ApiCommand::SetDraftSettings),
    ] {
        let request: ApiRequest = serde_json::from_value(serde_json::json!({
            "request_id": 88,
            "command": command,
            "payload": {}
        }))
        .unwrap();
        assert_eq!(request.command, expected);
    }

    for command in [
        "GET_DRAFT",
        "SET_DRAFT_OVERRIDE",
        "CLEAR_DRAFT_OVERRIDE",
        "SET_DRAFT_OPPONENT",
        "SET_DRAFT_RULE",
    ] {
        assert!(serde_json::from_value::<ApiRequest>(serde_json::json!({
            "request_id": 89,
            "command": command,
            "payload": {}
        }))
        .is_err());
    }
}

#[test]
fn atlas_core_rejects_editor_only_commands() {
    for command in [
        "GET_EDITOR_DATA",
        "APPLY_EDITOR_SETTINGS",
        "GET_PLAYER_MASTERY",
        "SET_PLAYER_MASTERY",
        "APPLY_PLAYER_EDIT",
        "APPLY_STAFF_EDIT",
        "MOVE_PLAYER",
        "GET_LOCKS",
        "SET_LOCK",
        "UNLOCK",
    ] {
        let request = serde_json::from_value::<ApiRequest>(serde_json::json!({
                "request_id": 90,
                "command": command,
                "payload": {}
            }))
            .unwrap();
        assert!(
            !ApiSurface::Core.allows(request.command),
            "Atlas Core accepted Editor-only command {command}"
        );
    }
}
