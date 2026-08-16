//! Mod-side safe wrapper over [`ServerCtxV1`] — what server-extension hooks
//! use to read/write the mod's save namespace and emit client events.
//! Compiled into the mod; never crosses the boundary itself.

use std::mem::size_of;

use crate::json_help;
use crate::{ServerCtxV1, ServerVtableV1, SettingTargetV1, StableEventTarget};

/// Size-guarded view over the per-hook server context. Valid only inside
/// the hook that received it.
pub struct StableServerCtx<'a> {
    raw: *mut ServerCtxV1,
    _marker: std::marker::PhantomData<&'a mut ServerCtxV1>,
}

impl StableServerCtx<'_> {
    /// # Safety
    /// `raw` must point at a live `ServerCtxV1` for the duration of the hook.
    pub unsafe fn from_raw(raw: *mut ServerCtxV1) -> Option<Self> {
        if raw.is_null() || (*raw).size < size_of::<ServerCtxV1>() {
            return None;
        }
        Some(Self {
            raw,
            _marker: std::marker::PhantomData,
        })
    }

    fn table(&self) -> *const ServerVtableV1 {
        unsafe { (*self.raw).vtable }
    }

    fn state(&self) -> *mut std::ffi::c_void {
        unsafe { (*self.raw).state }
    }

    // -- save data (this mod's namespace) --

    pub fn save_version(&self) -> usize {
        slot!(self.table(), ServerVtableV1, save_version).map_or(0, |f| unsafe { f(self.state()) })
    }

    pub fn save_set_version(&mut self, version: usize) -> bool {
        slot!(self.table(), ServerVtableV1, save_set_version)
            .map_or(false, |f| unsafe { f(self.state(), version) })
    }

    pub fn save_contains_key(&self, key: &str) -> bool {
        slot!(self.table(), ServerVtableV1, save_contains_key).map_or(false, |f| unsafe {
            f(self.state(), key.as_ptr(), key.len())
        })
    }

    pub fn save_get_bytes(&self, key: &str) -> Option<Vec<u8>> {
        let f = slot!(self.table(), ServerVtableV1, save_get)?;
        let mut len = 0usize;
        if !unsafe {
            f(
                self.state(),
                key.as_ptr(),
                key.len(),
                std::ptr::null_mut(),
                0,
                &mut len,
            )
        } {
            return None;
        }
        let mut buf = vec![0u8; len];
        let mut full_len = 0usize;
        if !unsafe {
            f(
                self.state(),
                key.as_ptr(),
                key.len(),
                buf.as_mut_ptr(),
                buf.len(),
                &mut full_len,
            )
        } {
            return None;
        }
        buf.truncate(full_len.min(len));
        Some(buf)
    }

    pub fn save_get_string(&self, key: &str) -> Option<String> {
        String::from_utf8(self.save_get_bytes(key)?).ok()
    }

    pub fn save_set_bytes(&mut self, key: &str, value: &[u8]) -> bool {
        slot!(self.table(), ServerVtableV1, save_set).map_or(false, |f| unsafe {
            f(
                self.state(),
                key.as_ptr(),
                key.len(),
                value.as_ptr(),
                value.len(),
            )
        })
    }

    pub fn save_set_string(&mut self, key: &str, value: &str) -> bool {
        self.save_set_bytes(key, value.as_bytes())
    }

    pub fn save_remove_key(&mut self, key: &str) -> bool {
        slot!(self.table(), ServerVtableV1, save_remove).map_or(false, |f| unsafe {
            f(self.state(), key.as_ptr(), key.len())
        })
    }

    pub fn save_keys(&self) -> Vec<String> {
        let Some(count_fn) = slot!(self.table(), ServerVtableV1, save_key_count) else {
            return Vec::new();
        };
        let Some(at_fn) = slot!(self.table(), ServerVtableV1, save_key_at) else {
            return Vec::new();
        };
        let count = unsafe { count_fn(self.state()) };
        let mut keys = Vec::with_capacity(count);
        for index in 0..count {
            let mut len = 0usize;
            if !unsafe { at_fn(self.state(), index, std::ptr::null_mut(), 0, &mut len) } {
                continue;
            }
            let mut buf = vec![0u8; len];
            let mut full_len = 0usize;
            if unsafe {
                at_fn(
                    self.state(),
                    index,
                    buf.as_mut_ptr(),
                    buf.len(),
                    &mut full_len,
                )
            } {
                buf.truncate(full_len.min(len));
                if let Ok(key) = String::from_utf8(buf) {
                    keys.push(key);
                }
            }
        }
        keys
    }

    // -- client events --

    pub fn emit_event(&mut self, target: StableEventTarget, event: &str, payload: &[u8]) -> bool {
        let (kind, id) = target.to_parts();
        slot!(self.table(), ServerVtableV1, emit_event).map_or(false, |f| unsafe {
            f(
                self.state(),
                kind,
                id,
                event.as_ptr(),
                event.len(),
                payload.as_ptr(),
                payload.len(),
            )
        })
    }

    // -- lobby helpers --

    pub fn player_team_id(&self, player_id: usize) -> Option<usize> {
        let f = slot!(self.table(), ServerVtableV1, player_team_id)?;
        let mut team = 0usize;
        unsafe { f(self.state(), player_id, &mut team) }.then_some(team)
    }

    // -- game rule settings (affect matches created afterwards) --

    /// Reads a settings fragment as JSON. Empty `path` = whole document;
    /// dot-separated segments descend objects (numeric segment = array index).
    pub fn setting_get_json(&self, target: SettingTargetV1, path: &str) -> Option<String> {
        let f = slot!(self.table(), ServerVtableV1, setting_get_json)?;
        let mut len = 0usize;
        if !unsafe {
            f(
                self.state(),
                target.code(),
                path.as_ptr(),
                path.len(),
                std::ptr::null_mut(),
                0,
                &mut len,
            )
        } {
            return None;
        }
        let mut buf = vec![0u8; len];
        let mut full = 0usize;
        if !unsafe {
            f(
                self.state(),
                target.code(),
                path.as_ptr(),
                path.len(),
                buf.as_mut_ptr(),
                buf.len(),
                &mut full,
            )
        } {
            return None;
        }
        buf.truncate(full.min(len));
        String::from_utf8(buf).ok()
    }

    /// Replaces a settings fragment with the given JSON. Rejected (false)
    /// when the path is missing or the resulting document no longer parses.
    pub fn setting_set_json(&mut self, target: SettingTargetV1, path: &str, json: &str) -> bool {
        slot!(self.table(), ServerVtableV1, setting_set_json).map_or(false, |f| unsafe {
            f(
                self.state(),
                target.code(),
                path.as_ptr(),
                path.len(),
                json.as_ptr(),
                json.len(),
            )
        })
    }

    pub fn setting_get_i64(&self, target: SettingTargetV1, path: &str) -> Option<i64> {
        json_help::parse_i64(&self.setting_get_json(target, path)?)
    }

    pub fn setting_get_f64(&self, target: SettingTargetV1, path: &str) -> Option<f64> {
        json_help::parse_f64(&self.setting_get_json(target, path)?)
    }

    pub fn setting_get_bool(&self, target: SettingTargetV1, path: &str) -> Option<bool> {
        json_help::parse_bool(&self.setting_get_json(target, path)?)
    }

    pub fn setting_get_string(&self, target: SettingTargetV1, path: &str) -> Option<String> {
        json_help::parse_string(&self.setting_get_json(target, path)?)
    }

    pub fn setting_set_i64(&mut self, target: SettingTargetV1, path: &str, value: i64) -> bool {
        self.setting_set_json(target, path, &value.to_string())
    }

    pub fn setting_set_f64(&mut self, target: SettingTargetV1, path: &str, value: f64) -> bool {
        self.setting_set_json(target, path, &value.to_string())
    }

    pub fn setting_set_bool(&mut self, target: SettingTargetV1, path: &str, value: bool) -> bool {
        self.setting_set_json(target, path, if value { "true" } else { "false" })
    }

    pub fn setting_set_string(&mut self, target: SettingTargetV1, path: &str, value: &str) -> bool {
        self.setting_set_json(target, path, &json_help::quote_string(value))
    }

    // -- management data documents (raw record edits; keeping cross-record
    // invariants coherent is up to the mod) --

    pub fn team_get_json(&self, team_id: usize, path: &str) -> Option<String> {
        let f = slot!(self.table(), ServerVtableV1, team_get_json)?;
        read_doc_two_call(|buf, cap, out_len| unsafe {
            f(
                self.state(),
                team_id,
                path.as_ptr(),
                path.len(),
                buf,
                cap,
                out_len,
            )
        })
    }

    pub fn team_set_json(&mut self, team_id: usize, path: &str, json: &str) -> bool {
        slot!(self.table(), ServerVtableV1, team_set_json).map_or(false, |f| unsafe {
            f(
                self.state(),
                team_id,
                path.as_ptr(),
                path.len(),
                json.as_ptr(),
                json.len(),
            )
        })
    }

    pub fn team_get_string(&self, team_id: usize, path: &str) -> Option<String> {
        json_help::parse_string(&self.team_get_json(team_id, path)?)
    }

    pub fn team_set_string(&mut self, team_id: usize, path: &str, value: &str) -> bool {
        self.team_set_json(team_id, path, &json_help::quote_string(value))
    }

    pub fn team_get_i64(&self, team_id: usize, path: &str) -> Option<i64> {
        json_help::parse_i64(&self.team_get_json(team_id, path)?)
    }

    pub fn team_set_i64(&mut self, team_id: usize, path: &str, value: i64) -> bool {
        self.team_set_json(team_id, path, &value.to_string())
    }

    pub fn athlete_get_json(&self, athlete_id: usize, path: &str) -> Option<String> {
        let f = slot!(self.table(), ServerVtableV1, athlete_get_json)?;
        read_doc_two_call(|buf, cap, out_len| unsafe {
            f(
                self.state(),
                athlete_id,
                path.as_ptr(),
                path.len(),
                buf,
                cap,
                out_len,
            )
        })
    }

    pub fn athlete_set_json(&mut self, athlete_id: usize, path: &str, json: &str) -> bool {
        slot!(self.table(), ServerVtableV1, athlete_set_json).map_or(false, |f| unsafe {
            f(
                self.state(),
                athlete_id,
                path.as_ptr(),
                path.len(),
                json.as_ptr(),
                json.len(),
            )
        })
    }

    pub fn athlete_get_string(&self, athlete_id: usize, path: &str) -> Option<String> {
        json_help::parse_string(&self.athlete_get_json(athlete_id, path)?)
    }

    pub fn athlete_set_string(&mut self, athlete_id: usize, path: &str, value: &str) -> bool {
        self.athlete_set_json(athlete_id, path, &json_help::quote_string(value))
    }

    pub fn athlete_get_i64(&self, athlete_id: usize, path: &str) -> Option<i64> {
        json_help::parse_i64(&self.athlete_get_json(athlete_id, path)?)
    }

    pub fn athlete_set_i64(&mut self, athlete_id: usize, path: &str, value: i64) -> bool {
        self.athlete_set_json(athlete_id, path, &value.to_string())
    }

    // -- generic record documents (any RecordKindV1; Match addresses the
    // whole server match table, the per-category Match* kinds are
    // client-only views) --

    pub fn record_ids(&self, kind: crate::RecordKindV1) -> Vec<usize> {
        let Some(f) = slot!(self.table(), ServerVtableV1, record_ids) else {
            return Vec::new();
        };
        let total = unsafe { f(self.state(), kind.code(), std::ptr::null_mut(), 0) };
        let mut ids = vec![0usize; total];
        let written =
            unsafe { f(self.state(), kind.code(), ids.as_mut_ptr(), ids.len()) }.min(total);
        ids.truncate(written);
        ids
    }

    pub fn record_get_json(
        &self,
        kind: crate::RecordKindV1,
        record_id: usize,
        path: &str,
    ) -> Option<String> {
        let f = slot!(self.table(), ServerVtableV1, record_get_json)?;
        read_doc_two_call(|buf, cap, out_len| unsafe {
            f(
                self.state(),
                kind.code(),
                record_id,
                path.as_ptr(),
                path.len(),
                buf,
                cap,
                out_len,
            )
        })
    }

    pub fn record_set_json(
        &mut self,
        kind: crate::RecordKindV1,
        record_id: usize,
        path: &str,
        json: &str,
    ) -> bool {
        slot!(self.table(), ServerVtableV1, record_set_json).map_or(false, |f| unsafe {
            f(
                self.state(),
                kind.code(),
                record_id,
                path.as_ptr(),
                path.len(),
                json.as_ptr(),
                json.len(),
            )
        })
    }

    pub fn record_get_string(
        &self,
        kind: crate::RecordKindV1,
        record_id: usize,
        path: &str,
    ) -> Option<String> {
        json_help::parse_string(&self.record_get_json(kind, record_id, path)?)
    }

    pub fn record_set_string(
        &mut self,
        kind: crate::RecordKindV1,
        record_id: usize,
        path: &str,
        value: &str,
    ) -> bool {
        self.record_set_json(kind, record_id, path, &json_help::quote_string(value))
    }

    /// Every management event with `seq > after_seq`, oldest first. Keep the
    /// last seen seq in your own state and poll from the management-tick
    /// hooks; the host retains a bounded backlog.
    pub fn management_events_after(&self, after_seq: u64) -> Vec<StableManagementEvent> {
        let Some(f) = slot!(self.table(), ServerVtableV1, management_event_next) else {
            return Vec::new();
        };
        let mut events = Vec::new();
        let mut cursor = after_seq;
        loop {
            let (mut seq, mut kind) = (0u64, u32::MAX);
            let mut len = 0usize;
            if !unsafe {
                f(
                    self.state(),
                    cursor,
                    &mut seq,
                    &mut kind,
                    std::ptr::null_mut(),
                    0,
                    &mut len,
                )
            } {
                break;
            }
            let mut buf = vec![0u8; len];
            let mut full = 0usize;
            if !unsafe {
                f(
                    self.state(),
                    cursor,
                    &mut seq,
                    &mut kind,
                    buf.as_mut_ptr(),
                    buf.len(),
                    &mut full,
                )
            } {
                break;
            }
            buf.truncate(full.min(len));
            let Ok(payload_json) = String::from_utf8(buf) else {
                break;
            };
            events.push(StableManagementEvent {
                seq,
                kind: crate::ManagementEventKindV1::from_code(kind),
                payload_json,
            });
            cursor = seq;
        }
        events
    }

    /// Forces a transfer: moves the athlete to `to_team_id` now with a
    /// lump-sum fee (budgets/finance reports move; buyer may go negative).
    /// Free agents are signed at fair salary and the fee is ignored.
    pub fn force_transfer(
        &mut self,
        athlete_id: usize,
        to_team_id: usize,
        transfer_fee: f64,
    ) -> bool {
        slot!(self.table(), ServerVtableV1, force_transfer).map_or(false, |f| unsafe {
            f(self.state(), athlete_id, to_team_id, transfer_fee)
        })
    }

    /// Pushes a simple news article onto the team's news feed, dated now.
    pub fn news_push(&mut self, team_id: usize, title: &str, content: &str, author: &str) -> bool {
        slot!(self.table(), ServerVtableV1, news_push).map_or(false, |f| unsafe {
            f(
                self.state(),
                team_id,
                title.as_ptr(),
                title.len(),
                content.as_ptr(),
                content.len(),
                author.as_ptr(),
                author.len(),
            )
        })
    }
}

fn read_doc_two_call(call: impl Fn(*mut u8, usize, *mut usize) -> bool) -> Option<String> {
    let mut len = 0usize;
    if !call(std::ptr::null_mut(), 0, &mut len) {
        return None;
    }
    let mut buf = vec![0u8; len];
    let mut full = 0usize;
    if !call(buf.as_mut_ptr(), buf.len(), &mut full) {
        return None;
    }
    buf.truncate(full.min(len));
    String::from_utf8(buf).ok()
}

/// Management event returned by [`StableServerCtx::management_events_after`].
/// `kind` is None for codes newer than this crate — skip those.
#[derive(Clone, Debug)]
pub struct StableManagementEvent {
    pub seq: u64,
    pub kind: Option<crate::ManagementEventKindV1>,
    pub payload_json: String,
}

/// Command delivered to [`crate::StableServerExtension::handle_command`].
#[derive(Clone, Copy, Debug)]
pub struct StableCommand<'a> {
    pub command: &'a str,
    pub payload: &'a [u8],
    pub sender_player_id: Option<usize>,
    pub sender_team_id: Option<usize>,
}

impl StableCommand<'_> {
    /// Where a reply event should go: the sending player if known, else the
    /// sending team, else broadcast — mirroring the legacy reply helper.
    pub fn reply_target(&self) -> StableEventTarget {
        if let Some(player) = self.sender_player_id {
            StableEventTarget::Player(player)
        } else if let Some(team) = self.sender_team_id {
            StableEventTarget::Team(team)
        } else {
            StableEventTarget::Broadcast
        }
    }
}
