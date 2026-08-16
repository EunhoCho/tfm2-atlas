//! Server-extension boundary: authoritative management-server hooks.
//!
//! A mod's server extension receives a per-callback [`ServerCtxV1`] whose
//! table exposes namespace-scoped save data (always the mod's own id — the
//! host enforces the namespace) and client-event emission. Direct Database
//! mutation is deliberately NOT part of the stable contract.

use std::ffi::c_void;

/// Per-callback server context. Valid only for the duration of the hook.
#[repr(C)]
pub struct ServerCtxV1 {
    pub size: usize,
    pub vtable: *const ServerVtableV1,
    /// Opaque host state passed back to every slot.
    pub state: *mut c_void,
}

/// Host functions available inside server hooks.
///
/// Byte outputs use the two-call convention: pass a null buffer to receive
/// only the required length via `out_len`, then call again with a buffer;
/// the host writes at most `buf_cap` bytes and always reports the full
/// length.
#[repr(C)]
pub struct ServerVtableV1 {
    pub size: usize,
    // -- mod save data (namespace = this mod's id) --
    pub save_version: Option<unsafe extern "C" fn(state: *const c_void) -> usize>,
    pub save_set_version: Option<unsafe extern "C" fn(state: *mut c_void, version: usize) -> bool>,
    pub save_contains_key:
        Option<unsafe extern "C" fn(state: *const c_void, key: *const u8, key_len: usize) -> bool>,
    pub save_get: Option<
        unsafe extern "C" fn(
            state: *const c_void,
            key: *const u8,
            key_len: usize,
            buf: *mut u8,
            buf_cap: usize,
            out_len: *mut usize,
        ) -> bool,
    >,
    pub save_set: Option<
        unsafe extern "C" fn(
            state: *mut c_void,
            key: *const u8,
            key_len: usize,
            value: *const u8,
            value_len: usize,
        ) -> bool,
    >,
    pub save_remove:
        Option<unsafe extern "C" fn(state: *mut c_void, key: *const u8, key_len: usize) -> bool>,
    pub save_key_count: Option<unsafe extern "C" fn(state: *const c_void) -> usize>,
    pub save_key_at: Option<
        unsafe extern "C" fn(
            state: *const c_void,
            index: usize,
            buf: *mut u8,
            buf_cap: usize,
            out_len: *mut usize,
        ) -> bool,
    >,
    // -- client events --
    /// `target_kind` carries an [`crate::EventTargetKindV1`] code;
    /// `target_id` is the player/team id for the targeted kinds.
    pub emit_event: Option<
        unsafe extern "C" fn(
            state: *mut c_void,
            target_kind: u32,
            target_id: usize,
            event: *const u8,
            event_len: usize,
            payload: *const u8,
            payload_len: usize,
        ) -> bool,
    >,
    // -- lobby helpers --
    pub player_team_id: Option<
        unsafe extern "C" fn(state: *const c_void, player_id: usize, out_team: *mut usize) -> bool,
    >,
    // -- game rule settings (authoritative; affect matches created afterwards) --
    /// Reads a fragment of a settings document as JSON. `target` carries a
    /// [`crate::SettingTargetV1`] code; `path` is dot-separated (numeric
    /// segment = array index, empty = whole document). Two-call output.
    pub setting_get_json: Option<
        unsafe extern "C" fn(
            state: *const c_void,
            target: u32,
            path: *const u8,
            path_len: usize,
            buf: *mut u8,
            buf_cap: usize,
            out_len: *mut usize,
        ) -> bool,
    >,
    /// Replaces a fragment of a settings document with the given JSON. The
    /// whole document must still deserialize; otherwise nothing changes and
    /// this returns false.
    pub setting_set_json: Option<
        unsafe extern "C" fn(
            state: *mut c_void,
            target: u32,
            path: *const u8,
            path_len: usize,
            json: *const u8,
            json_len: usize,
        ) -> bool,
    >,
    // -- management data documents (team/athlete by id, same path syntax) --
    // Writes replace the fragment and re-deserialize the whole record
    // atomically. These are raw record edits: keeping cross-record
    // invariants (contracts, rosters) coherent is the mod's responsibility.
    pub team_get_json: Option<
        unsafe extern "C" fn(
            state: *const c_void,
            team_id: usize,
            path: *const u8,
            path_len: usize,
            buf: *mut u8,
            buf_cap: usize,
            out_len: *mut usize,
        ) -> bool,
    >,
    pub team_set_json: Option<
        unsafe extern "C" fn(
            state: *mut c_void,
            team_id: usize,
            path: *const u8,
            path_len: usize,
            json: *const u8,
            json_len: usize,
        ) -> bool,
    >,
    pub athlete_get_json: Option<
        unsafe extern "C" fn(
            state: *const c_void,
            athlete_id: usize,
            path: *const u8,
            path_len: usize,
            buf: *mut u8,
            buf_cap: usize,
            out_len: *mut usize,
        ) -> bool,
    >,
    pub athlete_set_json: Option<
        unsafe extern "C" fn(
            state: *mut c_void,
            athlete_id: usize,
            path: *const u8,
            path_len: usize,
            json: *const u8,
            json_len: usize,
        ) -> bool,
    >,
    /// Writes up to `cap` record ids of the given [`crate::RecordKindV1`];
    /// returns the total count. On the server the `Match` kind addresses the
    /// whole match table; the per-category `Match*` kinds are client-only.
    pub record_ids: Option<
        unsafe extern "C" fn(state: *const c_void, kind: u32, out: *mut usize, cap: usize) -> usize,
    >,
    /// Reads a fragment of any management record as JSON.
    pub record_get_json: Option<
        unsafe extern "C" fn(
            state: *const c_void,
            kind: u32,
            record_id: usize,
            path: *const u8,
            path_len: usize,
            buf: *mut u8,
            buf_cap: usize,
            out_len: *mut usize,
        ) -> bool,
    >,
    /// Replaces a fragment of any management record (atomic reject on
    /// schema violation). Server-authority write path.
    pub record_set_json: Option<
        unsafe extern "C" fn(
            state: *mut c_void,
            kind: u32,
            record_id: usize,
            path: *const u8,
            path_len: usize,
            json: *const u8,
            json_len: usize,
        ) -> bool,
    >,
    /// Pushes a simple news article onto a team's news feed, dated now.
    pub news_push: Option<
        unsafe extern "C" fn(
            state: *mut c_void,
            team_id: usize,
            title: *const u8,
            title_len: usize,
            content: *const u8,
            content_len: usize,
            author: *const u8,
            author_len: usize,
        ) -> bool,
    >,
    /// Returns the oldest retained management event with `seq > after_seq`
    /// (kind = [`crate::ManagementEventKindV1`] code, payload = JSON via the
    /// two-call convention). False when no newer event exists. Mods keep
    /// their own cursor and loop until false — typically from the
    /// management-tick hooks.
    pub management_event_next: Option<
        unsafe extern "C" fn(
            state: *const c_void,
            after_seq: u64,
            out_seq: *mut u64,
            out_kind: *mut u32,
            payload_buf: *mut u8,
            payload_cap: usize,
            out_payload_len: *mut usize,
        ) -> bool,
    >,
    /// Forces a transfer: moves the athlete to `to_team_id` now with a
    /// lump-sum `transfer_fee` (both teams' budgets and finance reports move;
    /// the buyer's budget may go negative). Free agents are signed with a
    /// fair-salary contract and the fee is ignored. False when the athlete/
    /// team is unknown or the athlete is already on the target team.
    pub force_transfer: Option<
        unsafe extern "C" fn(
            state: *mut c_void,
            athlete_id: usize,
            to_team_id: usize,
            transfer_fee: f64,
        ) -> bool,
    >,
}

/// Mod-implemented server extension hooks. `handle_command` returns an
/// [`crate::CommandResultV1`] code.
#[repr(C)]
pub struct ServerExtVtableV1 {
    pub size: usize,
    pub destroy: Option<unsafe extern "C" fn(userdata: *mut c_void)>,
    pub on_server_start: Option<unsafe extern "C" fn(userdata: *mut c_void, ctx: *mut ServerCtxV1)>,
    pub before_management_tick:
        Option<unsafe extern "C" fn(userdata: *mut c_void, ctx: *mut ServerCtxV1)>,
    pub after_management_tick:
        Option<unsafe extern "C" fn(userdata: *mut c_void, ctx: *mut ServerCtxV1)>,
    pub handle_command: Option<
        unsafe extern "C" fn(
            userdata: *mut c_void,
            ctx: *mut ServerCtxV1,
            command: *const u8,
            command_len: usize,
            payload: *const u8,
            payload_len: usize,
            sender_player_id: usize,
            has_sender_player: bool,
            sender_team_id: usize,
            has_sender_team: bool,
        ) -> u32,
    >,
}

/// A registered server extension: opaque mod-side state plus its table.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct ServerExtRegV1 {
    pub userdata: *mut c_void,
    pub vtable: *const ServerExtVtableV1,
}

impl ServerExtRegV1 {
    pub const NULL: Self = Self {
        userdata: std::ptr::null_mut(),
        vtable: std::ptr::null(),
    };

    pub fn is_null(&self) -> bool {
        self.vtable.is_null()
    }
}
