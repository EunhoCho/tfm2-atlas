//! Client-extension boundary: title/game-loop lifecycle hooks.
//!
//! A mod's extension receives a per-callback [`ClientCtxV1`] bundling the
//! ui/scene/asset/save/net tables plus opaque host state. Inside UI event
//! handlers the host provides a reduced context (no scene), so save/net
//! calls fail gracefully there — mods should mutate UI in handlers and do
//! data work in the update hooks, mirroring legacy usage.

use std::ffi::c_void;

use crate::{
    AssetVtableV1, DataVtableV1, DrawVtableV1, NetVtableV1, SaveVtableV1, SceneVtableV1, UiVtableV1,
};

/// Per-callback client context. Valid only for the duration of the callback;
/// never store it.
#[repr(C)]
pub struct ClientCtxV1 {
    pub size: usize,
    pub ui: *const UiVtableV1,
    pub scene: *const SceneVtableV1,
    pub asset: *const AssetVtableV1,
    pub save: *const SaveVtableV1,
    pub net: *const NetVtableV1,
    /// Opaque host state passed back to every slot.
    pub state: *mut c_void,
    pub data: *const DataVtableV1,
    /// Draw table — live only inside `pre_render`/`post_render`.
    pub draw: *const DrawVtableV1,
}

/// Callback registered for a UI event. Receives a fresh reduced context per
/// firing; the userdata is owned by the mod and lives until process exit.
pub type UiClickHandlerFn = unsafe extern "C" fn(userdata: *mut c_void, ctx: *mut ClientCtxV1);

/// Mod-implemented client extension hooks. `dt_micros` is the frame delta in
/// microseconds.
#[repr(C)]
pub struct ExtensionVtableV1 {
    pub size: usize,
    pub destroy: Option<unsafe extern "C" fn(userdata: *mut c_void)>,
    pub on_init: Option<unsafe extern "C" fn(userdata: *mut c_void, ctx: *mut ClientCtxV1)>,
    pub pre_update:
        Option<unsafe extern "C" fn(userdata: *mut c_void, ctx: *mut ClientCtxV1, dt_micros: u64)>,
    pub post_update:
        Option<unsafe extern "C" fn(userdata: *mut c_void, ctx: *mut ClientCtxV1, dt_micros: u64)>,
    pub on_end: Option<unsafe extern "C" fn(userdata: *mut c_void)>,
    /// Render hooks: run before/after the frame's own draw pass. The ctx is
    /// read-only (UI/save/net mutation slots fail) and `ctx.draw` is live.
    pub pre_render: Option<unsafe extern "C" fn(userdata: *mut c_void, ctx: *mut ClientCtxV1)>,
    pub post_render: Option<unsafe extern "C" fn(userdata: *mut c_void, ctx: *mut ClientCtxV1)>,
}

/// A registered client extension: opaque mod-side state plus its table.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct ExtensionRegV1 {
    pub userdata: *mut c_void,
    pub vtable: *const ExtensionVtableV1,
}

impl ExtensionRegV1 {
    pub const NULL: Self = Self {
        userdata: std::ptr::null_mut(),
        vtable: std::ptr::null(),
    };

    pub fn is_null(&self) -> bool {
        self.vtable.is_null()
    }
}
