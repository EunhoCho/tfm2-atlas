use tfm2_atlas_core::{ApiCommand, ApiSurface};

#[test]
fn editor_surface_accepts_editor_commands_and_rejects_core_commands() {
    assert!(ApiSurface::Editor.allows(ApiCommand::GetEditorData));
    assert!(ApiSurface::Editor.allows(ApiCommand::SetLock));
    assert!(!ApiSurface::Editor.allows(ApiCommand::GetDashboard));
    assert!(!ApiSurface::Editor.allows(ApiCommand::ApplyTierProfile));
    assert!(!ApiSurface::Editor.allows(ApiCommand::GetMockDraft));
}
