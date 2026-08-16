//! Generic JSON-document context: the host hands a mod an editable serde
//! document (map definition, and future document-shaped surfaces) behind two
//! slots. Same path semantics as the settings API.

use std::ffi::c_void;
use std::mem::size_of;

use crate::json_help;

#[repr(C)]
pub struct JsonDocVtableV1 {
    pub size: usize,
    /// Two-call JSON fragment read.
    pub get_json: Option<
        unsafe extern "C" fn(
            state: *const c_void,
            path: *const u8,
            path_len: usize,
            buf: *mut u8,
            buf_cap: usize,
            out_len: *mut usize,
        ) -> bool,
    >,
    /// Replaces the fragment at `path`. Schema validation happens when the
    /// host applies the document, so a bad edit voids the whole edit pass.
    pub set_json: Option<
        unsafe extern "C" fn(
            state: *mut c_void,
            path: *const u8,
            path_len: usize,
            json: *const u8,
            json_len: usize,
        ) -> bool,
    >,
}

/// Per-callback editable JSON document. Valid only inside the callback.
#[repr(C)]
pub struct JsonDocCtxV1 {
    pub size: usize,
    pub vtable: *const JsonDocVtableV1,
    pub state: *mut c_void,
}

/// Mod-side safe wrapper over [`JsonDocCtxV1`].
pub struct StableJsonDoc<'a> {
    raw: *mut JsonDocCtxV1,
    _marker: std::marker::PhantomData<&'a mut JsonDocCtxV1>,
}

impl StableJsonDoc<'_> {
    /// # Safety
    /// `raw` must point at a live `JsonDocCtxV1` for the callback duration.
    pub unsafe fn from_raw(raw: *mut JsonDocCtxV1) -> Option<Self> {
        if raw.is_null() || (*raw).size < size_of::<JsonDocCtxV1>() {
            return None;
        }
        Some(Self {
            raw,
            _marker: std::marker::PhantomData,
        })
    }

    fn table(&self) -> *const JsonDocVtableV1 {
        unsafe { (*self.raw).vtable }
    }

    fn state(&self) -> *mut c_void {
        unsafe { (*self.raw).state }
    }

    pub fn get_json(&self, path: &str) -> Option<String> {
        let f = slot!(self.table(), JsonDocVtableV1, get_json)?;
        let mut len = 0usize;
        if !unsafe {
            f(
                self.state(),
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

    pub fn set_json(&mut self, path: &str, json: &str) -> bool {
        slot!(self.table(), JsonDocVtableV1, set_json).map_or(false, |f| unsafe {
            f(
                self.state(),
                path.as_ptr(),
                path.len(),
                json.as_ptr(),
                json.len(),
            )
        })
    }

    pub fn get_i64(&self, path: &str) -> Option<i64> {
        json_help::parse_i64(&self.get_json(path)?)
    }

    pub fn get_f64(&self, path: &str) -> Option<f64> {
        json_help::parse_f64(&self.get_json(path)?)
    }

    pub fn get_bool(&self, path: &str) -> Option<bool> {
        json_help::parse_bool(&self.get_json(path)?)
    }

    pub fn get_string(&self, path: &str) -> Option<String> {
        json_help::parse_string(&self.get_json(path)?)
    }

    pub fn set_i64(&mut self, path: &str, value: i64) -> bool {
        self.set_json(path, &value.to_string())
    }

    pub fn set_bool(&mut self, path: &str, value: bool) -> bool {
        self.set_json(path, if value { "true" } else { "false" })
    }

    pub fn set_string(&mut self, path: &str, value: &str) -> bool {
        self.set_json(path, &json_help::quote_string(value))
    }
}

/// Mod-implemented map customizer: adjusts a match's map document right
/// before the game captures it.
#[repr(C)]
pub struct MapCustomizerVtableV1 {
    pub size: usize,
    pub destroy: Option<unsafe extern "C" fn(userdata: *mut c_void)>,
    /// `mode_kind` carries a [`crate::GameModeKindV1`] code.
    pub customize:
        Option<unsafe extern "C" fn(userdata: *mut c_void, mode_kind: u32, doc: *mut JsonDocCtxV1)>,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct MapCustomizerRegV1 {
    pub userdata: *mut c_void,
    pub vtable: *const MapCustomizerVtableV1,
}

impl MapCustomizerRegV1 {
    pub const NULL: Self = Self {
        userdata: std::ptr::null_mut(),
        vtable: std::ptr::null(),
    };

    pub fn is_null(&self) -> bool {
        self.vtable.is_null()
    }
}
