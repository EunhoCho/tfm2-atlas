//! Mod-side safe wrapper over [`ClientCtxV1`] — what extension hooks and UI
//! handlers use. Compiled into the mod; never crosses the boundary itself.

use std::ffi::c_void;
use std::mem::size_of;
use std::panic::{catch_unwind, AssertUnwindSafe};

use crate::{
    AssetVtableV1, ClientCtxV1, DataVtableV1, DrawVtableV1, NetVtableV1, RecordKindV1,
    SaveVtableV1, SceneKindV1, SceneVtableV1, TextAlignXV1, TextAlignYV1, UiEventKindV1,
    UiVtableV1,
};

/// The UI event a `ui_register_path_events` callback is currently handling.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StableUiEvent {
    pub kind: Option<UiEventKindV1>,
    pub path: String,
    /// Kind-specific payload as JSON (see [`UiEventKindV1`] docs).
    pub payload_json: String,
}

/// Event drained from the server for this mod.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StableModEvent {
    pub event: String,
    pub payload: Vec<u8>,
}

/// Raw keyboard event returned by [`StableClient::input_events`]. `kind` is
/// None for codes newer than this crate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StableInputEvent {
    pub kind: Option<crate::InputEventKindV1>,
    /// Engine key name (e.g. "Tab", "F5", "A").
    pub key: String,
}

/// Owned champion identity returned by [`StableClient::champion_brief`].
#[derive(Clone, Debug)]
pub struct StableChampionBrief {
    pub name: String,
    pub category: Option<crate::ChampionCategoryV1>,
    pub tags: Vec<crate::ChampionTagV1>,
    pub stat: crate::StatV1,
    pub growth: crate::StatV1,
}

/// Size-guarded view over the per-callback client context. Valid only inside
/// the callback that received it.
pub struct StableClient<'a> {
    raw: *mut ClientCtxV1,
    _marker: std::marker::PhantomData<&'a mut ClientCtxV1>,
}

impl StableClient<'_> {
    /// # Safety
    /// `raw` must point at a live `ClientCtxV1` for the callback duration.
    pub unsafe fn from_raw(raw: *mut ClientCtxV1) -> Option<Self> {
        // Require only the level-1 M3 prefix (through `state`); later fields
        // are size-guarded individually so older hosts keep working.
        let min = std::mem::offset_of!(ClientCtxV1, state) + size_of::<*mut std::ffi::c_void>();
        if raw.is_null() || (*raw).size < min {
            return None;
        }
        Some(Self {
            raw,
            _marker: std::marker::PhantomData,
        })
    }

    fn data_table(&self) -> *const DataVtableV1 {
        let end = std::mem::offset_of!(ClientCtxV1, data) + size_of::<*const DataVtableV1>();
        if unsafe { (*self.raw).size } < end {
            return std::ptr::null();
        }
        unsafe { (*self.raw).data }
    }

    fn state(&self) -> *mut c_void {
        unsafe { (*self.raw).state }
    }

    fn ui_table(&self) -> *const UiVtableV1 {
        unsafe { (*self.raw).ui }
    }

    fn scene_table(&self) -> *const SceneVtableV1 {
        unsafe { (*self.raw).scene }
    }

    fn asset_table(&self) -> *const AssetVtableV1 {
        unsafe { (*self.raw).asset }
    }

    fn save_table(&self) -> *const SaveVtableV1 {
        unsafe { (*self.raw).save }
    }

    fn net_table(&self) -> *const NetVtableV1 {
        unsafe { (*self.raw).net }
    }

    // -- scene --

    pub fn scene_kind(&self) -> Option<SceneKindV1> {
        let code = slot!(self.scene_table(), SceneVtableV1, kind)
            .map_or(u32::MAX, |f| unsafe { f(self.state()) });
        SceneKindV1::from_code(code)
    }

    pub fn is_in_game(&self) -> bool {
        self.scene_kind() == Some(SceneKindV1::InGame)
    }

    // -- NewGame scene (all fail gracefully outside it) --

    pub fn new_game_team_id(&self) -> Option<usize> {
        let f = slot!(self.scene_table(), SceneVtableV1, new_game_team_id)?;
        let mut out = 0usize;
        unsafe { f(self.state(), &mut out) }.then_some(out)
    }

    pub fn new_game_set_team_id(&mut self, team_id: usize) -> bool {
        slot!(self.scene_table(), SceneVtableV1, new_game_set_team_id)
            .map_or(false, |f| unsafe { f(self.state(), team_id) })
    }

    pub fn new_game_with_tutorial(&self) -> Option<bool> {
        let f = slot!(self.scene_table(), SceneVtableV1, new_game_with_tutorial)?;
        let mut out = false;
        unsafe { f(self.state(), &mut out) }.then_some(out)
    }

    pub fn new_game_set_with_tutorial(&mut self, with_tutorial: bool) -> bool {
        slot!(
            self.scene_table(),
            SceneVtableV1,
            new_game_set_with_tutorial
        )
        .map_or(false, |f| unsafe { f(self.state(), with_tutorial) })
    }

    pub fn new_game_manager_name(&self) -> Option<String> {
        let f = slot!(self.scene_table(), SceneVtableV1, new_game_manager_name)?;
        read_two_call_string(|buf, cap, out_len| unsafe { f(self.state(), buf, cap, out_len) })
    }

    pub fn new_game_set_manager_name(&mut self, name: &str) -> bool {
        slot!(self.scene_table(), SceneVtableV1, new_game_set_manager_name)
            .map_or(false, |f| unsafe {
                f(self.state(), name.as_ptr(), name.len())
            })
    }

    /// Reads a fragment of the new game's play-option document as JSON.
    pub fn new_game_option_get_json(&self, path: &str) -> Option<String> {
        let f = slot!(self.scene_table(), SceneVtableV1, new_game_option_get_json)?;
        read_two_call_string(|buf, cap, out_len| unsafe {
            f(self.state(), path.as_ptr(), path.len(), buf, cap, out_len)
        })
    }

    /// Replaces a fragment of the play-option document (atomic reject).
    pub fn new_game_option_set_json(&mut self, path: &str, json: &str) -> bool {
        slot!(self.scene_table(), SceneVtableV1, new_game_option_set_json).map_or(
            false,
            |f| unsafe {
                f(
                    self.state(),
                    path.as_ptr(),
                    path.len(),
                    json.as_ptr(),
                    json.len(),
                )
            },
        )
    }

    // -- raw keyboard events (per frame, hotkey surface) --

    /// Raw keyboard events recorded for the current frame.
    pub fn input_events(&self) -> Vec<StableInputEvent> {
        let Some(count_slot) = slot!(self.ui_table(), UiVtableV1, input_event_count) else {
            return Vec::new();
        };
        let Some(at_slot) = slot!(self.ui_table(), UiVtableV1, input_event_at) else {
            return Vec::new();
        };
        let count = unsafe { count_slot(self.state()) };
        (0..count)
            .filter_map(|index| {
                let kind = std::cell::Cell::new(u32::MAX);
                let key = read_two_call_string(|buf, cap, out_len| unsafe {
                    let mut k = u32::MAX;
                    let ok = at_slot(self.state(), index, &mut k, buf, cap, out_len);
                    kind.set(k);
                    ok
                })?;
                Some(StableInputEvent {
                    kind: crate::InputEventKindV1::from_code(kind.get()),
                    key,
                })
            })
            .collect()
    }

    /// True when the named key (engine key name, e.g. "F5") was pressed this
    /// frame.
    pub fn key_pressed(&self, key: &str) -> bool {
        self.input_events().iter().any(|event| {
            event.kind == Some(crate::InputEventKindV1::KeyPressed) && event.key == key
        })
    }

    // -- ui --

    pub fn ui_exists(&self, path: &str) -> bool {
        slot!(self.ui_table(), UiVtableV1, query_exists).map_or(false, |f| unsafe {
            f(self.state(), path.as_ptr(), path.len())
        })
    }

    pub fn ui_visible(&self, path: &str) -> Option<bool> {
        let f = slot!(self.ui_table(), UiVtableV1, get_visible)?;
        let mut visible = false;
        unsafe { f(self.state(), path.as_ptr(), path.len(), &mut visible) }.then_some(visible)
    }

    pub fn ui_set_visible(&mut self, path: &str, visible: bool) -> bool {
        slot!(self.ui_table(), UiVtableV1, set_visible).map_or(false, |f| unsafe {
            f(self.state(), path.as_ptr(), path.len(), visible)
        })
    }

    /// Loads a NodeTemplate asset and appends it as a child of `parent_path`.
    pub fn ui_spawn_template(
        &mut self,
        parent_path: &str,
        template_asset: &str,
        visible: bool,
    ) -> bool {
        slot!(self.ui_table(), UiVtableV1, spawn_template).map_or(false, |f| unsafe {
            f(
                self.state(),
                parent_path.as_ptr(),
                parent_path.len(),
                template_asset.as_ptr(),
                template_asset.len(),
                visible,
            )
        })
    }

    /// Registers a click handler. The closure is leaked (handlers live until
    /// process exit) — register once per UI build, guarded by the mod.
    pub fn ui_register_click(
        &mut self,
        path: &str,
        item: &str,
        handler: impl Fn(&mut StableClient<'_>) + Send + Sync + 'static,
    ) -> bool {
        let Some(f) = slot!(self.ui_table(), UiVtableV1, register_click) else {
            return false;
        };
        let userdata = Box::into_raw(Box::new(HandlerData {
            handler: Box::new(handler),
        }));
        unsafe {
            f(
                self.state(),
                path.as_ptr(),
                path.len(),
                item.as_ptr(),
                item.len(),
                Some(click_trampoline),
                userdata as *mut c_void,
            )
        }
    }

    /// Reads a label/text node's text.
    pub fn ui_text(&self, path: &str) -> Option<String> {
        let f = slot!(self.ui_table(), UiVtableV1, get_text)?;
        read_two_call_string(|buf, cap, out_len| unsafe {
            f(self.state(), path.as_ptr(), path.len(), buf, cap, out_len)
        })
    }

    /// Sets a label node's text.
    pub fn ui_set_text(&mut self, path: &str, text: &str) -> bool {
        slot!(self.ui_table(), UiVtableV1, set_text).map_or(false, |f| unsafe {
            f(
                self.state(),
                path.as_ptr(),
                path.len(),
                text.as_ptr(),
                text.len(),
            )
        })
    }

    /// Registers a handler for every UI event on `path`; inside the handler,
    /// [`Self::ui_current_event`] describes the firing event. The closure is
    /// leaked (handlers live until process exit).
    pub fn ui_register_path_events(
        &mut self,
        path: &str,
        handler: impl Fn(&mut StableClient<'_>) + Send + Sync + 'static,
    ) -> bool {
        let Some(f) = slot!(self.ui_table(), UiVtableV1, register_path_events) else {
            return false;
        };
        let userdata = Box::into_raw(Box::new(HandlerData {
            handler: Box::new(handler),
        }));
        unsafe {
            f(
                self.state(),
                path.as_ptr(),
                path.len(),
                Some(click_trampoline),
                userdata as *mut c_void,
            )
        }
    }

    /// Spawns a subtree from `.ui` template source under `parent_path`, e.g.
    /// `ui_spawn_source("body", "hint:label { text: \"hi\"; }")`. Any
    /// registered runner kind ("label", "checkbox", "slider", ...) works; the
    /// source grammar is exactly the one `.ui` asset files use, children and
    /// `@"asset#style"` refs included.
    pub fn ui_spawn_source(&mut self, parent_path: &str, source: &str) -> bool {
        slot!(self.ui_table(), UiVtableV1, spawn_source).map_or(false, |f| unsafe {
            f(
                self.state(),
                parent_path.as_ptr(),
                parent_path.len(),
                source.as_ptr(),
                source.len(),
            )
        })
    }

    /// Applies `.ui` property source (`key: value; ...`) to an existing node:
    /// layout keys, `visible`/`disable`, and the runner's own properties.
    pub fn ui_set_properties(&mut self, path: &str, source: &str) -> bool {
        slot!(self.ui_table(), UiVtableV1, set_properties).map_or(false, |f| unsafe {
            f(
                self.state(),
                path.as_ptr(),
                path.len(),
                source.as_ptr(),
                source.len(),
            )
        })
    }

    /// Removes the node at `path` together with its children.
    pub fn ui_remove_node(&mut self, path: &str) -> bool {
        slot!(self.ui_table(), UiVtableV1, remove_node).map_or(false, |f| unsafe {
            f(self.state(), path.as_ptr(), path.len())
        })
    }

    /// Registered runner name of the node ("label", "checkbox", ...). Empty
    /// string when the node exists but its kind is unknown to the host.
    pub fn ui_runner_name(&self, path: &str) -> Option<String> {
        let f = slot!(self.ui_table(), UiVtableV1, runner_name)?;
        read_two_call_string(|buf, cap, out_len| unsafe {
            f(self.state(), path.as_ptr(), path.len(), buf, cap, out_len)
        })
    }

    /// Runtime interaction state of the node's runner as JSON (see
    /// [`crate::UiVtableV1::state_get_json`] for the per-kind keys).
    pub fn ui_state_json(&self, path: &str) -> Option<String> {
        let f = slot!(self.ui_table(), UiVtableV1, state_get_json)?;
        read_two_call_string(|buf, cap, out_len| unsafe {
            f(self.state(), path.as_ptr(), path.len(), buf, cap, out_len)
        })
    }

    /// Writes runner runtime state from a JSON object (rejected atomically on
    /// unknown keys/kinds). See [`crate::UiVtableV1::state_set_json`].
    pub fn ui_set_state_json(&mut self, path: &str, json: &str) -> bool {
        slot!(self.ui_table(), UiVtableV1, state_set_json).map_or(false, |f| unsafe {
            f(
                self.state(),
                path.as_ptr(),
                path.len(),
                json.as_ptr(),
                json.len(),
            )
        })
    }

    // -- typed sugar over the state JSON --

    pub fn ui_checkbox_selected(&self, path: &str) -> Option<bool> {
        crate::json_help::parse_bool(&crate::json_help::object_field(
            &self.ui_state_json(path)?,
            "selected",
        )?)
    }

    pub fn ui_set_checkbox_selected(&mut self, path: &str, selected: bool) -> bool {
        self.ui_set_state_json(path, &format!("{{\"selected\":{}}}", selected))
    }

    pub fn ui_text_edit_text(&self, path: &str) -> Option<String> {
        crate::json_help::parse_string(&crate::json_help::object_field(
            &self.ui_state_json(path)?,
            "text",
        )?)
    }

    pub fn ui_set_text_edit_text(&mut self, path: &str, text: &str) -> bool {
        self.ui_set_state_json(
            path,
            &format!("{{\"text\":{}}}", crate::json_help::quote_string(text)),
        )
    }

    pub fn ui_slider_ratio(&self, path: &str) -> Option<f64> {
        crate::json_help::parse_f64(&crate::json_help::object_field(
            &self.ui_state_json(path)?,
            "ratio",
        )?)
    }

    pub fn ui_set_slider_ratio(&mut self, path: &str, ratio: f64) -> bool {
        self.ui_set_state_json(path, &format!("{{\"ratio\":{}}}", ratio))
    }

    pub fn ui_selectable_selected(&self, path: &str) -> Option<bool> {
        crate::json_help::parse_bool(&crate::json_help::object_field(
            &self.ui_state_json(path)?,
            "selected",
        )?)
    }

    pub fn ui_set_selectable_selected(&mut self, path: &str, selected: bool) -> bool {
        self.ui_set_state_json(path, &format!("{{\"selected\":{}}}", selected))
    }

    pub fn ui_dropdown_selected_item(&self, path: &str) -> Option<i64> {
        crate::json_help::parse_i64(&crate::json_help::object_field(
            &self.ui_state_json(path)?,
            "selected_item",
        )?)
    }

    /// Computed layout rect (x, y, w, h) of a node from the last layout pass.
    pub fn ui_node_rect(&self, path: &str) -> Option<(f32, f32, f32, f32)> {
        self.ui_rect_of(path, false)
    }

    /// Computed contents rect of a node (inner area after padding/scroll).
    pub fn ui_contents_rect(&self, path: &str) -> Option<(f32, f32, f32, f32)> {
        self.ui_rect_of(path, true)
    }

    fn ui_rect_of(&self, path: &str, contents: bool) -> Option<(f32, f32, f32, f32)> {
        let f = slot!(self.ui_table(), UiVtableV1, node_rect)?;
        let (mut x, mut y, mut w, mut h) = (0f32, 0f32, 0f32, 0f32);
        unsafe {
            f(
                self.state(),
                path.as_ptr(),
                path.len(),
                contents,
                &mut x,
                &mut y,
                &mut w,
                &mut h,
            )
        }
        .then_some((x, y, w, h))
    }

    /// Number of direct children. Empty path addresses the UI root.
    pub fn ui_child_count(&self, path: &str) -> Option<usize> {
        let f = slot!(self.ui_table(), UiVtableV1, child_count)?;
        let mut count = 0usize;
        unsafe { f(self.state(), path.as_ptr(), path.len(), &mut count) }.then_some(count)
    }

    /// Ids of all direct children. Empty path addresses the UI root.
    pub fn ui_child_names(&self, path: &str) -> Vec<String> {
        let Some(count) = self.ui_child_count(path) else {
            return Vec::new();
        };
        let Some(f) = slot!(self.ui_table(), UiVtableV1, child_name_at) else {
            return Vec::new();
        };
        (0..count)
            .filter_map(|index| {
                read_two_call_string(|buf, cap, out_len| unsafe {
                    f(
                        self.state(),
                        path.as_ptr(),
                        path.len(),
                        index,
                        buf,
                        cap,
                        out_len,
                    )
                })
            })
            .collect()
    }

    /// Inside a `ui_register_path_events` handler: the firing event.
    pub fn ui_current_event(&self) -> Option<StableUiEvent> {
        let f = slot!(self.ui_table(), UiVtableV1, current_event)?;
        let mut kind = u32::MAX;
        let (mut path_len, mut payload_len) = (0usize, 0usize);
        if !unsafe {
            f(
                self.state(),
                &mut kind,
                std::ptr::null_mut(),
                0,
                &mut path_len,
                std::ptr::null_mut(),
                0,
                &mut payload_len,
            )
        } {
            return None;
        }
        let mut path = vec![0u8; path_len];
        let mut payload = vec![0u8; payload_len];
        let (mut full_path, mut full_payload) = (0usize, 0usize);
        if !unsafe {
            f(
                self.state(),
                &mut kind,
                path.as_mut_ptr(),
                path.len(),
                &mut full_path,
                payload.as_mut_ptr(),
                payload.len(),
                &mut full_payload,
            )
        } {
            return None;
        }
        path.truncate(full_path.min(path_len));
        payload.truncate(full_payload.min(payload_len));
        Some(StableUiEvent {
            kind: UiEventKindV1::from_code(kind),
            path: String::from_utf8(path).ok()?,
            payload_json: String::from_utf8(payload).ok()?,
        })
    }

    // -- assets / audio --

    /// Plays a sound asset once at the given volume (0.0..=1.0). Queued and
    /// played by the game's frame loop, so it works from any hook.
    pub fn play_sound(&mut self, path: &str, volume: f32) -> bool {
        slot!(self.asset_table(), AssetVtableV1, play_sound).map_or(false, |f| unsafe {
            f(self.state(), path.as_ptr(), path.len(), volume)
        })
    }

    /// Replaces the background-music playlist with the given sound assets.
    pub fn play_bgm(&mut self, paths: &[&str]) -> bool {
        let Some(f) = slot!(self.asset_table(), AssetVtableV1, play_bgm) else {
            return false;
        };
        let strs: Vec<crate::StrV1> = paths.iter().map(|p| crate::StrV1::from_str(p)).collect();
        unsafe { f(self.state(), strs.as_ptr(), strs.len()) }
    }

    /// Stops the current background music.
    pub fn stop_bgm(&mut self) -> bool {
        slot!(self.asset_table(), AssetVtableV1, stop_bgm)
            .map_or(false, |f| unsafe { f(self.state()) })
    }

    pub fn i18n(&self, key: &str) -> Option<String> {
        let f = slot!(self.asset_table(), AssetVtableV1, i18n_get)?;
        read_two_call_string(|buf, cap, out_len| unsafe {
            f(self.state(), key.as_ptr(), key.len(), buf, cap, out_len)
        })
    }

    // -- save (this mod's namespace; no-ops outside an active game) --

    pub fn save_can_write(&self) -> bool {
        slot!(self.save_table(), SaveVtableV1, can_write)
            .map_or(false, |f| unsafe { f(self.state()) })
    }

    pub fn save_version(&self) -> usize {
        slot!(self.save_table(), SaveVtableV1, version).map_or(0, |f| unsafe { f(self.state()) })
    }

    pub fn save_set_version(&mut self, version: usize) -> bool {
        slot!(self.save_table(), SaveVtableV1, set_version)
            .map_or(false, |f| unsafe { f(self.state(), version) })
    }

    pub fn save_contains_key(&self, key: &str) -> bool {
        slot!(self.save_table(), SaveVtableV1, contains_key).map_or(false, |f| unsafe {
            f(self.state(), key.as_ptr(), key.len())
        })
    }

    pub fn save_get_bytes(&self, key: &str) -> Option<Vec<u8>> {
        let f = slot!(self.save_table(), SaveVtableV1, get)?;
        read_two_call_bytes(|buf, cap, out_len| unsafe {
            f(self.state(), key.as_ptr(), key.len(), buf, cap, out_len)
        })
    }

    pub fn save_get_string(&self, key: &str) -> Option<String> {
        String::from_utf8(self.save_get_bytes(key)?).ok()
    }

    pub fn save_set_bytes(&mut self, key: &str, value: &[u8]) -> bool {
        slot!(self.save_table(), SaveVtableV1, set).map_or(false, |f| unsafe {
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
        slot!(self.save_table(), SaveVtableV1, remove).map_or(false, |f| unsafe {
            f(self.state(), key.as_ptr(), key.len())
        })
    }

    pub fn save_keys(&self) -> Vec<String> {
        let Some(count_fn) = slot!(self.save_table(), SaveVtableV1, key_count) else {
            return Vec::new();
        };
        let Some(at_fn) = slot!(self.save_table(), SaveVtableV1, key_at) else {
            return Vec::new();
        };
        let count = unsafe { count_fn(self.state()) };
        (0..count)
            .filter_map(|index| {
                read_two_call_string(|buf, cap, out_len| unsafe {
                    at_fn(self.state(), index, buf, cap, out_len)
                })
            })
            .collect()
    }

    // -- management data reads (InGame only) --

    pub fn player_team_id(&self) -> Option<usize> {
        let f = slot!(self.data_table(), DataVtableV1, player_team_id)?;
        let mut out = 0usize;
        unsafe { f(self.state(), &mut out) }.then_some(out)
    }

    pub fn team_ids(&self) -> Vec<usize> {
        let Some(f) = slot!(self.data_table(), DataVtableV1, team_ids) else {
            return Vec::new();
        };
        let total = unsafe { f(self.state(), std::ptr::null_mut(), 0) };
        let mut ids = vec![0usize; total];
        let written = unsafe { f(self.state(), ids.as_mut_ptr(), ids.len()) }.min(total);
        ids.truncate(written);
        ids
    }

    pub fn team_name(&self, team_id: usize) -> Option<String> {
        let f = slot!(self.data_table(), DataVtableV1, team_name)?;
        read_two_call_string(|buf, cap, out_len| unsafe {
            f(self.state(), team_id, buf, cap, out_len)
        })
    }

    pub fn athlete_ids(&self) -> Vec<usize> {
        let Some(f) = slot!(self.data_table(), DataVtableV1, athlete_ids) else {
            return Vec::new();
        };
        let total = unsafe { f(self.state(), std::ptr::null_mut(), 0) };
        let mut ids = vec![0usize; total];
        let written = unsafe { f(self.state(), ids.as_mut_ptr(), ids.len()) }.min(total);
        ids.truncate(written);
        ids
    }

    pub fn athlete_name(&self, athlete_id: usize) -> Option<String> {
        let f = slot!(self.data_table(), DataVtableV1, athlete_name)?;
        read_two_call_string(|buf, cap, out_len| unsafe {
            f(self.state(), athlete_id, buf, cap, out_len)
        })
    }

    /// Read-only settings view (same path semantics as the server-side API).
    pub fn setting_get_json(&self, target: crate::SettingTargetV1, path: &str) -> Option<String> {
        let f = slot!(self.data_table(), DataVtableV1, setting_get_json)?;
        read_two_call_string(|buf, cap, out_len| unsafe {
            f(
                self.state(),
                target.code(),
                path.as_ptr(),
                path.len(),
                buf,
                cap,
                out_len,
            )
        })
    }

    pub fn setting_get_i64(&self, target: crate::SettingTargetV1, path: &str) -> Option<i64> {
        crate::json_help::parse_i64(&self.setting_get_json(target, path)?)
    }

    pub fn setting_get_string(&self, target: crate::SettingTargetV1, path: &str) -> Option<String> {
        crate::json_help::parse_string(&self.setting_get_json(target, path)?)
    }

    /// Ids of every record of the given kind (InGame only).
    pub fn record_ids(&self, kind: RecordKindV1) -> Vec<usize> {
        let Some(f) = slot!(self.data_table(), DataVtableV1, record_ids) else {
            return Vec::new();
        };
        let total = unsafe { f(self.state(), kind.code(), std::ptr::null_mut(), 0) };
        let mut ids = vec![0usize; total];
        let written =
            unsafe { f(self.state(), kind.code(), ids.as_mut_ptr(), ids.len()) }.min(total);
        ids.truncate(written);
        ids
    }

    /// Reads a management record fragment as JSON (read-only).
    pub fn record_get_json(
        &self,
        kind: RecordKindV1,
        record_id: usize,
        path: &str,
    ) -> Option<String> {
        let f = slot!(self.data_table(), DataVtableV1, record_get_json)?;
        read_two_call_string(|buf, cap, out_len| unsafe {
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

    pub fn record_get_string(
        &self,
        kind: RecordKindV1,
        record_id: usize,
        path: &str,
    ) -> Option<String> {
        crate::json_help::parse_string(&self.record_get_json(kind, record_id, path)?)
    }

    pub fn record_get_i64(&self, kind: RecordKindV1, record_id: usize, path: &str) -> Option<i64> {
        crate::json_help::parse_i64(&self.record_get_json(kind, record_id, path)?)
    }

    /// Current in-game date/time as (year, month, day, hour, minute).
    pub fn game_time(&self) -> Option<(i32, u32, u32, u32, u32)> {
        let f = slot!(self.data_table(), DataVtableV1, game_time)?;
        let (mut y, mut mo, mut d, mut h, mut mi) = (0i32, 0u32, 0u32, 0u32, 0u32);
        unsafe { f(self.state(), &mut y, &mut mo, &mut d, &mut h, &mut mi) }
            .then_some((y, mo, d, h, mi))
    }

    /// Current in-game date as (year, month, day).
    pub fn game_date(&self) -> Option<(i32, u32, u32)> {
        self.game_time().map(|(y, mo, d, _, _)| (y, mo, d))
    }

    /// Player team's all-time official (wins, losses) against the opponent.
    pub fn head_to_head(&self, opponent_team_id: usize) -> Option<(usize, usize)> {
        let f = slot!(self.data_table(), DataVtableV1, head_to_head)?;
        let (mut wins, mut losses) = (0usize, 0usize);
        unsafe { f(self.state(), opponent_team_id, &mut wins, &mut losses) }
            .then_some((wins, losses))
    }

    /// Final (champion, runner-up) team ids of a finished competition.
    pub fn competition_result(
        &self,
        is_tournament: bool,
        competition_id: usize,
    ) -> Option<(usize, usize)> {
        let f = slot!(self.data_table(), DataVtableV1, competition_result)?;
        let (mut champion, mut runner_up) = (0usize, 0usize);
        unsafe {
            f(
                self.state(),
                is_tournament,
                competition_id,
                &mut champion,
                &mut runner_up,
            )
        }
        .then_some((champion, runner_up))
    }

    /// Read-only JSON view of the season schedule document.
    pub fn schedule_get_json(&self, path: &str) -> Option<String> {
        let f = slot!(self.data_table(), DataVtableV1, schedule_get_json)?;
        read_two_call_string(|buf, cap, out_len| unsafe {
            f(self.state(), path.as_ptr(), path.len(), buf, cap, out_len)
        })
    }

    /// Names of every selectable champion in the active (patched) sheet.
    pub fn champion_names(&self) -> Vec<String> {
        let Some(count_slot) = slot!(self.data_table(), DataVtableV1, champion_count) else {
            return Vec::new();
        };
        let Some(name_slot) = slot!(self.data_table(), DataVtableV1, champion_name_at) else {
            return Vec::new();
        };
        let count = unsafe { count_slot(self.state()) };
        (0..count)
            .filter_map(|index| {
                read_two_call_string(|buf, cap, out_len| unsafe {
                    name_slot(self.state(), index, buf, cap, out_len)
                })
            })
            .collect()
    }

    /// Current in-client screen (InGame only; None elsewhere or on codes
    /// newer than this crate).
    pub fn client_scene_kind(&self) -> Option<crate::ClientSceneKindV1> {
        let f = slot!(self.data_table(), DataVtableV1, client_scene_kind)?;
        crate::ClientSceneKindV1::from_code(unsafe { f(self.state()) })
    }

    /// Variant name of the current main-screen tab (e.g. "Home", "Squad").
    /// None when the client is not on the Main screen.
    pub fn client_main_tab(&self) -> Option<String> {
        let f = slot!(self.data_table(), DataVtableV1, client_main_tab_name)?;
        read_two_call_string(|buf, cap, out_len| unsafe { f(self.state(), buf, cap, out_len) })
    }

    /// Static identity (category/tags/base stat/growth) of a champion.
    pub fn champion_brief(&self, name: &str) -> Option<StableChampionBrief> {
        let f = slot!(self.data_table(), DataVtableV1, champion_brief)?;
        let mut out = crate::ChampionBriefV1::default();
        if !unsafe { f(self.state(), name.as_ptr(), name.len(), &mut out) } {
            return None;
        }
        Some(StableChampionBrief {
            name: name.to_string(),
            category: crate::ChampionCategoryV1::from_code(out.category),
            tags: out.tags[..(out.tag_count as usize).min(out.tags.len())]
                .iter()
                .filter_map(|code| crate::ChampionTagV1::from_code(*code))
                .collect(),
            stat: out.stat,
            growth: out.growth,
        })
    }

    // -- net --

    pub fn send_command(&mut self, command: &str, payload: &[u8]) -> bool {
        slot!(self.net_table(), NetVtableV1, send_command).map_or(false, |f| unsafe {
            f(
                self.state(),
                command.as_ptr(),
                command.len(),
                payload.as_ptr(),
                payload.len(),
            )
        })
    }

    /// Drains and returns all pending server events for this mod.
    pub fn take_events(&mut self) -> Vec<StableModEvent> {
        let Some(take) = slot!(self.net_table(), NetVtableV1, take_events) else {
            return Vec::new();
        };
        let Some(at) = slot!(self.net_table(), NetVtableV1, taken_event_at) else {
            return Vec::new();
        };
        let count = unsafe { take(self.state()) };
        let mut events = Vec::with_capacity(count);
        for index in 0..count {
            let (mut name_len, mut payload_len) = (0usize, 0usize);
            if !unsafe {
                at(
                    self.state(),
                    index,
                    std::ptr::null_mut(),
                    0,
                    &mut name_len,
                    std::ptr::null_mut(),
                    0,
                    &mut payload_len,
                )
            } {
                continue;
            }
            let mut name = vec![0u8; name_len];
            let mut payload = vec![0u8; payload_len];
            let (mut full_name, mut full_payload) = (0usize, 0usize);
            if unsafe {
                at(
                    self.state(),
                    index,
                    name.as_mut_ptr(),
                    name.len(),
                    &mut full_name,
                    payload.as_mut_ptr(),
                    payload.len(),
                    &mut full_payload,
                )
            } {
                name.truncate(full_name.min(name_len));
                payload.truncate(full_payload.min(payload_len));
                if let Ok(event) = String::from_utf8(name) {
                    events.push(StableModEvent { event, payload });
                }
            }
        }
        events
    }

    // -- draw (live only inside pre_render/post_render) --

    fn draw_table(&self) -> *const DrawVtableV1 {
        let end = std::mem::offset_of!(ClientCtxV1, draw) + size_of::<*const DrawVtableV1>();
        if unsafe { (*self.raw).size } < end {
            return std::ptr::null();
        }
        unsafe { (*self.raw).draw }
    }

    /// True only while a render hook is running.
    pub fn can_draw(&self) -> bool {
        slot!(self.draw_table(), DrawVtableV1, can_draw)
            .map_or(false, |f| unsafe { f(self.state()) })
    }

    /// Size of a named render map ("UI"/"Default" are screen space, "Game"
    /// is the match world), if it has one this frame.
    pub fn draw_map_size(&self, map: &str) -> Option<(f32, f32)> {
        let f = slot!(self.draw_table(), DrawVtableV1, map_size)?;
        let (mut w, mut h) = (0f32, 0f32);
        unsafe { f(self.state(), map.as_ptr(), map.len(), &mut w, &mut h) }.then_some((w, h))
    }

    /// Points the map's camera at a rect (center + extent) in map units.
    pub fn draw_set_camera(
        &mut self,
        map: &str,
        center_x: f32,
        center_y: f32,
        w: f32,
        h: f32,
    ) -> bool {
        slot!(self.draw_table(), DrawVtableV1, set_camera).map_or(false, |f| unsafe {
            f(
                self.state(),
                map.as_ptr(),
                map.len(),
                center_x,
                center_y,
                w,
                h,
            )
        })
    }

    /// Filled box; `rounding` > 0 rounds the corners. Color is 0xRRGGBBAA.
    pub fn draw_rect(
        &mut self,
        map: &str,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        z: i32,
        rounding: f32,
        color: u32,
    ) -> bool {
        slot!(self.draw_table(), DrawVtableV1, rect).map_or(false, |f| unsafe {
            f(
                self.state(),
                map.as_ptr(),
                map.len(),
                x,
                y,
                w,
                h,
                z,
                rounding,
                color,
            )
        })
    }

    pub fn draw_circle(
        &mut self,
        map: &str,
        center_x: f32,
        center_y: f32,
        radius: f32,
        z: i32,
        color: u32,
    ) -> bool {
        slot!(self.draw_table(), DrawVtableV1, circle).map_or(false, |f| unsafe {
            f(
                self.state(),
                map.as_ptr(),
                map.len(),
                center_x,
                center_y,
                radius,
                z,
                color,
            )
        })
    }

    pub fn draw_line(
        &mut self,
        map: &str,
        from_x: f32,
        from_y: f32,
        to_x: f32,
        to_y: f32,
        width: f32,
        z: i32,
        color: u32,
    ) -> bool {
        slot!(self.draw_table(), DrawVtableV1, line).map_or(false, |f| unsafe {
            f(
                self.state(),
                map.as_ptr(),
                map.len(),
                from_x,
                from_y,
                to_x,
                to_y,
                width,
                z,
                color,
            )
        })
    }

    /// Text laid out inside the rect. `font` is a font asset path such as
    /// "asset/base/font/set/regular".
    #[allow(clippy::too_many_arguments)]
    pub fn draw_text(
        &mut self,
        map: &str,
        text: &str,
        font: &str,
        rect: (f32, f32, f32, f32),
        z: i32,
        font_size: f32,
        color: u32,
        align_x: TextAlignXV1,
        align_y: TextAlignYV1,
    ) -> bool {
        slot!(self.draw_table(), DrawVtableV1, text).map_or(false, |f| unsafe {
            f(
                self.state(),
                map.as_ptr(),
                map.len(),
                text.as_ptr(),
                text.len(),
                font.as_ptr(),
                font.len(),
                rect.0,
                rect.1,
                rect.2,
                rect.3,
                z,
                font_size,
                color,
                align_x.code(),
                align_y.code(),
            )
        })
    }

    /// SVG asset scaled into the destination rect, tinted by `color`.
    pub fn draw_svg(
        &mut self,
        map: &str,
        path: &str,
        rect: (f32, f32, f32, f32),
        z: i32,
        color: u32,
    ) -> bool {
        slot!(self.draw_table(), DrawVtableV1, svg).map_or(false, |f| unsafe {
            f(
                self.state(),
                map.as_ptr(),
                map.len(),
                path.as_ptr(),
                path.len(),
                rect.0,
                rect.1,
                rect.2,
                rect.3,
                z,
                color,
            )
        })
    }

    /// Texture asset drawn at native pixel size with the given placement.
    pub fn draw_sprite(&mut self, map: &str, texture: &str, params: &StableSpriteParams) -> bool {
        slot!(self.draw_table(), DrawVtableV1, sprite).map_or(false, |f| unsafe {
            f(
                self.state(),
                map.as_ptr(),
                map.len(),
                texture.as_ptr(),
                texture.len(),
                params.x,
                params.y,
                params.z,
                params.rot,
                params.flip_x,
                params.flip_y,
                params.pivot_x,
                params.pivot_y,
                params.uv.0,
                params.uv.1,
                params.uv.2,
                params.uv.3,
                params.sample_nearest,
            )
        })
    }
}

/// Placement of a [`StableClient::draw_sprite`] call. `uv` is the normalized
/// source rect inside the texture ((0,0,1,1) = whole image); `pivot_*` is the
/// normalized anchor that `x`/`y` refer to.
#[derive(Clone, Copy, Debug)]
pub struct StableSpriteParams {
    pub x: f32,
    pub y: f32,
    pub z: i32,
    pub rot: f32,
    pub flip_x: bool,
    pub flip_y: bool,
    pub pivot_x: f32,
    pub pivot_y: f32,
    pub uv: (f32, f32, f32, f32),
    pub sample_nearest: bool,
}

impl Default for StableSpriteParams {
    fn default() -> Self {
        Self {
            x: 0.,
            y: 0.,
            z: 0,
            rot: 0.,
            flip_x: false,
            flip_y: false,
            pivot_x: 0.,
            pivot_y: 0.,
            uv: (0., 0., 1., 1.),
            sample_nearest: true,
        }
    }
}

struct HandlerData {
    handler: Box<dyn Fn(&mut StableClient<'_>) + Send + Sync>,
}

unsafe extern "C" fn click_trampoline(userdata: *mut c_void, ctx: *mut ClientCtxV1) {
    let _ = catch_unwind(AssertUnwindSafe(|| {
        let Some(mut client) = (unsafe { StableClient::from_raw(ctx) }) else {
            return;
        };
        let data = unsafe { &*(userdata as *const HandlerData) };
        (data.handler)(&mut client);
    }));
}

fn read_two_call_bytes(call: impl Fn(*mut u8, usize, *mut usize) -> bool) -> Option<Vec<u8>> {
    let mut len = 0usize;
    if !call(std::ptr::null_mut(), 0, &mut len) {
        return None;
    }
    let mut buf = vec![0u8; len];
    let mut full_len = 0usize;
    if !call(buf.as_mut_ptr(), buf.len(), &mut full_len) {
        return None;
    }
    buf.truncate(full_len.min(len));
    Some(buf)
}

fn read_two_call_string(call: impl Fn(*mut u8, usize, *mut usize) -> bool) -> Option<String> {
    String::from_utf8(read_two_call_bytes(call)?).ok()
}
