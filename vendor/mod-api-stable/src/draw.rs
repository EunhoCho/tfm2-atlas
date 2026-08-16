//! Render-hook draw surface: push draw commands onto the frame being built.
//!
//! Slots are live only inside the extension's `pre_render`/`post_render`
//! hooks — everywhere else the host has no frame under construction and
//! every slot fails with `false`. Commands target a named render map
//! ("UI" and "Default" are 1920x1080 screen space, "Game" is the 2048x2048
//! match world) in that map's local coordinates.
//!
//! Colors are packed `0xRRGGBBAA`. Text alignment codes are
//! [`crate::TextAlignXV1`]/[`crate::TextAlignYV1`].

use std::ffi::c_void;

/// Host-implemented draw table, reachable through `ClientCtxV1.draw` during
/// render hooks (null pointer outside them).
#[repr(C)]
pub struct DrawVtableV1 {
    pub size: usize,
    /// True only while a render hook is running — the one context where the
    /// command slots below are live.
    pub can_draw: Option<unsafe extern "C" fn(state: *const c_void) -> bool>,
    /// Size of a named render map, if that map has one this frame.
    pub map_size: Option<
        unsafe extern "C" fn(
            state: *const c_void,
            map: *const u8,
            map_len: usize,
            out_w: *mut f32,
            out_h: *mut f32,
        ) -> bool,
    >,
    /// Points the map's camera at a world-space rect (center + extent).
    pub set_camera: Option<
        unsafe extern "C" fn(
            state: *mut c_void,
            map: *const u8,
            map_len: usize,
            center_x: f32,
            center_y: f32,
            w: f32,
            h: f32,
        ) -> bool,
    >,
    /// Filled (optionally rounded) box.
    pub rect: Option<
        unsafe extern "C" fn(
            state: *mut c_void,
            map: *const u8,
            map_len: usize,
            x: f32,
            y: f32,
            w: f32,
            h: f32,
            z: i32,
            rounding: f32,
            color: u32,
        ) -> bool,
    >,
    /// Filled circle.
    pub circle: Option<
        unsafe extern "C" fn(
            state: *mut c_void,
            map: *const u8,
            map_len: usize,
            center_x: f32,
            center_y: f32,
            radius: f32,
            z: i32,
            color: u32,
        ) -> bool,
    >,
    /// Solid line segment.
    pub line: Option<
        unsafe extern "C" fn(
            state: *mut c_void,
            map: *const u8,
            map_len: usize,
            from_x: f32,
            from_y: f32,
            to_x: f32,
            to_y: f32,
            width: f32,
            z: i32,
            color: u32,
        ) -> bool,
    >,
    /// Text laid out inside a rect. `font` is a font asset path
    /// (e.g. "asset/base/font/set/regular").
    pub text: Option<
        unsafe extern "C" fn(
            state: *mut c_void,
            map: *const u8,
            map_len: usize,
            text: *const u8,
            text_len: usize,
            font: *const u8,
            font_len: usize,
            x: f32,
            y: f32,
            w: f32,
            h: f32,
            z: i32,
            font_size: f32,
            color: u32,
            align_x: u32,
            align_y: u32,
        ) -> bool,
    >,
    /// SVG asset scaled into a destination rect, tinted by `color`.
    pub svg: Option<
        unsafe extern "C" fn(
            state: *mut c_void,
            map: *const u8,
            map_len: usize,
            path: *const u8,
            path_len: usize,
            x: f32,
            y: f32,
            w: f32,
            h: f32,
            z: i32,
            color: u32,
        ) -> bool,
    >,
    /// Texture asset drawn at native pixel size. `uv_*` is the normalized
    /// source rect inside the texture (pass 0,0,1,1 for the whole image);
    /// `pivot_*` is the normalized anchor the x/y position refers to.
    pub sprite: Option<
        unsafe extern "C" fn(
            state: *mut c_void,
            map: *const u8,
            map_len: usize,
            texture: *const u8,
            texture_len: usize,
            x: f32,
            y: f32,
            z: i32,
            rot: f32,
            flip_x: bool,
            flip_y: bool,
            pivot_x: f32,
            pivot_y: f32,
            uv_x: f32,
            uv_y: f32,
            uv_w: f32,
            uv_h: f32,
            sample_nearest: bool,
        ) -> bool,
    >,
}
