//! Safe Rust wrappers over the raw contract, compiled *into* the mod.
//!
//! Nothing here crosses the DLL boundary; wrappers may evolve freely because
//! each built DLL carries its own copy.

use std::mem::{offset_of, size_of};

use crate::{
    GameVersionV1, HostApiV1, ModExportV1, ServiceVtableV1, ABI_LEVEL, LOG_LEVEL_DEBUG,
    LOG_LEVEL_ERROR, LOG_LEVEL_INFO, LOG_LEVEL_WARN,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
}

impl LogLevel {
    fn code(self) -> u32 {
        match self {
            LogLevel::Error => LOG_LEVEL_ERROR,
            LogLevel::Warn => LOG_LEVEL_WARN,
            LogLevel::Info => LOG_LEVEL_INFO,
            LogLevel::Debug => LOG_LEVEL_DEBUG,
        }
    }
}

/// Size-guarded view over the host API. Valid only inside the callback that
/// received it — never store one.
pub struct StableHost {
    raw: *const HostApiV1,
    size: usize,
}

impl StableHost {
    /// # Safety
    /// `raw` must point at a live `HostApiV1` whose first field is its size.
    pub unsafe fn from_raw(raw: *const HostApiV1) -> Option<Self> {
        let size = (*raw).size;
        // The level-1 prefix (through `log`) must be present.
        if size
            < offset_of!(HostApiV1, log)
                + size_of::<Option<unsafe extern "C" fn(u32, *const u8, usize)>>()
        {
            return None;
        }
        Some(Self { raw, size })
    }

    fn has(&self, field_offset: usize, field_size: usize) -> bool {
        field_offset + field_size <= self.size
    }

    pub fn abi_level(&self) -> u32 {
        // SAFETY: within the verified level-1 prefix.
        unsafe { (*self.raw).host_abi_level }
    }

    pub fn game_version(&self) -> GameVersionV1 {
        // SAFETY: within the verified level-1 prefix.
        unsafe { (*self.raw).game_version }
    }

    pub fn log(&self, level: LogLevel, msg: &str) {
        let offset = offset_of!(HostApiV1, log);
        if !self.has(
            offset,
            size_of::<Option<unsafe extern "C" fn(u32, *const u8, usize)>>(),
        ) {
            return;
        }
        // SAFETY: slot verified inside the host's declared size; the host
        // guarantees slot fns stay valid for the process lifetime.
        unsafe {
            if let Some(f) = (*self.raw).log {
                f(level.code(), msg.as_ptr(), msg.len());
            }
        }
    }

    fn service_table(&self) -> *const crate::ServiceVtableV1 {
        let offset = offset_of!(HostApiV1, service);
        if !self.has(offset, size_of::<*const crate::ServiceVtableV1>()) {
            return std::ptr::null();
        }
        // SAFETY: field verified inside the host's declared size.
        unsafe { (*self.raw).service }
    }

    /// Publish a runtime service under this mod's id. Only valid while the
    /// mod's entry runs.
    pub fn register_service(
        &self,
        service_id: &str,
        version: crate::ServiceVersionV1,
        service: crate::ModServiceV1,
    ) -> bool {
        slot!(self.service_table(), ServiceVtableV1, register_service).map_or(false, |f| unsafe {
            f(service_id.as_ptr(), service_id.len(), version, service)
        })
    }

    /// Query a service published by another mod (loaded earlier via the
    /// dependency order). Only valid while this mod's entry runs.
    pub fn query_service(
        &self,
        provider_mod_id: &str,
        service_id: &str,
        version_req: &str,
    ) -> Option<crate::ModServiceV1> {
        let f = slot!(self.service_table(), ServiceVtableV1, query_service)?;
        let mut out = crate::ModServiceV1::NULL;
        unsafe {
            f(
                provider_mod_id.as_ptr(),
                provider_mod_id.len(),
                service_id.as_ptr(),
                service_id.len(),
                version_req.as_ptr(),
                version_req.len(),
                &mut out,
            )
        }
        .then_some(out)
    }
}

/// Builder for what the mod registers.
pub struct StableMod {
    mod_id: String,
    champions: Vec<crate::ChampionRegV1>,
    items: Vec<crate::ItemRegV1>,
    native_effects: Vec<(String, crate::EffectTypeRegV1)>,
    server_ext: crate::ServerExtRegV1,
    extension: crate::ExtensionRegV1,
    draft_hooks: Vec<crate::DraftHookRegV1>,
    item_build_hooks: Vec<crate::ItemBuildHookRegV1>,
    player_ai: Vec<crate::PlayerAiRegV1>,
    map_customizer: crate::MapCustomizerRegV1,
    match_hook: crate::MatchHookRegV1,
}

impl StableMod {
    pub fn new(mod_id: impl Into<String>) -> Self {
        Self {
            mod_id: mod_id.into(),
            champions: Vec::new(),
            items: Vec::new(),
            native_effects: Vec::new(),
            server_ext: crate::ServerExtRegV1::NULL,
            extension: crate::ExtensionRegV1::NULL,
            draft_hooks: Vec::new(),
            item_build_hooks: Vec::new(),
            player_ai: Vec::new(),
            map_customizer: crate::MapCustomizerRegV1::NULL,
            match_hook: crate::MatchHookRegV1::NULL,
        }
    }

    /// Register the mod's match hook (custom logic/win condition per tick).
    pub fn set_match_hook(&mut self, hook: impl crate::StableMatchHook) {
        self.match_hook = crate::mod_shim::match_hook_to_reg(Box::new(hook));
    }

    /// Register the mod's map customizer (adjusts match maps at creation).
    pub fn set_map_customizer(&mut self, customizer: impl crate::StableMapCustomizer) {
        self.map_customizer = crate::mod_shim::map_customizer_to_reg(Box::new(customizer));
    }

    /// Register a draft (ban/pick) score hook.
    pub fn add_draft_score_hook(&mut self, hook: impl crate::StableDraftHook) {
        self.draft_hooks
            .push(crate::mod_shim::draft_hook_to_reg(Box::new(hook)));
    }

    /// Register an item-build hook (steers which final items the AI targets).
    pub fn add_item_build_hook(&mut self, hook: impl crate::StableItemBuildHook) {
        self.item_build_hooks
            .push(crate::mod_shim::item_build_hook_to_reg(Box::new(hook)));
    }

    /// Register a per-tick player input AI.
    pub fn add_player_input_ai(&mut self, ai: impl crate::StablePlayerAi) {
        self.player_ai
            .push(crate::mod_shim::player_ai_to_reg(Box::new(ai)));
    }

    /// Register the mod's authoritative server extension.
    pub fn set_server_extension(&mut self, ext: impl crate::StableServerExtension) {
        self.server_ext = crate::mod_shim::server_ext_to_reg(Box::new(ext));
    }

    /// Register the mod's client extension (title/game lifecycle hooks).
    pub fn set_extension(&mut self, ext: impl crate::StableExtension) {
        self.extension = crate::mod_shim::extension_to_reg(Box::new(ext));
    }

    /// Register a champion (or rework an existing one by reusing its id).
    pub fn add_champion(&mut self, champion: impl crate::StableChampion) {
        self.champions
            .push(crate::mod_shim::champion_to_reg(Box::new(champion)));
    }

    pub fn add_item(&mut self, item: impl crate::StableItem) {
        self.items
            .push(crate::mod_shim::item_to_reg(Box::new(item)));
    }

    /// Register a named native effect referenced from `.data_champion`
    /// actions via `DataEffectDef::Native { effect_ref: name }`.
    pub fn add_native_effect(
        &mut self,
        name: impl Into<String>,
        effect: impl crate::StableEffectType,
    ) {
        self.native_effects.push((
            name.into(),
            crate::mod_shim::effect_type_to_reg(Box::new(effect)),
        ));
    }

    /// Materialize the boundary export. The container is freed by `destroy`,
    /// which runs in this DLL so allocator ownership never crosses sides.
    /// Content userdata is NOT freed by `destroy` — the host owns it after
    /// this call and releases it through each entry vtable's `destroy`.
    pub fn into_export_raw(self) -> *mut ModExportV1 {
        let (effect_names, effect_regs): (Vec<String>, Vec<crate::EffectTypeRegV1>) =
            self.native_effects.into_iter().unzip();

        let mut container = Box::new(ExportContainer {
            export: ModExportV1 {
                size: size_of::<ModExportV1>(),
                required_abi_level: ABI_LEVEL,
                mod_id_ptr: std::ptr::null(),
                mod_id_len: 0,
                destroy: Some(destroy_export),
                champions_ptr: std::ptr::null(),
                champions_len: 0,
                items_ptr: std::ptr::null(),
                items_len: 0,
                native_effects_ptr: std::ptr::null(),
                native_effects_len: 0,
                server_ext: self.server_ext,
                extension: self.extension,
                draft_hooks_ptr: std::ptr::null(),
                draft_hooks_len: 0,
                player_ai_ptr: std::ptr::null(),
                player_ai_len: 0,
                map_customizer: self.map_customizer,
                match_hook: self.match_hook,
                item_build_hooks_ptr: std::ptr::null(),
                item_build_hooks_len: 0,
            },
            mod_id: self.mod_id,
            champions: self.champions,
            items: self.items,
            native_effect_names: effect_names,
            native_effects: Vec::new(),
            draft_hooks: self.draft_hooks,
            item_build_hooks: self.item_build_hooks,
            player_ai: self.player_ai,
        });

        // Heap storage is address-stable now; wire the boundary pointers.
        container.native_effects = container
            .native_effect_names
            .iter()
            .zip(effect_regs)
            .map(|(name, effect)| crate::NativeEffectRegV1 {
                name: crate::StrV1::from_str(name),
                effect,
            })
            .collect();

        container.export.mod_id_ptr = container.mod_id.as_ptr();
        container.export.mod_id_len = container.mod_id.len();
        container.export.champions_ptr = container.champions.as_ptr();
        container.export.champions_len = container.champions.len();
        container.export.items_ptr = container.items.as_ptr();
        container.export.items_len = container.items.len();
        container.export.native_effects_ptr = container.native_effects.as_ptr();
        container.export.native_effects_len = container.native_effects.len();
        container.export.draft_hooks_ptr = container.draft_hooks.as_ptr();
        container.export.draft_hooks_len = container.draft_hooks.len();
        container.export.player_ai_ptr = container.player_ai.as_ptr();
        container.export.player_ai_len = container.player_ai.len();
        Box::into_raw(container) as *mut ModExportV1
    }
}

/// `export` must stay the first field: the host only knows `*mut ModExportV1`
/// and `destroy` casts it back to the container.
#[repr(C)]
struct ExportContainer {
    export: ModExportV1,
    mod_id: String,
    champions: Vec<crate::ChampionRegV1>,
    items: Vec<crate::ItemRegV1>,
    native_effect_names: Vec<String>,
    native_effects: Vec<crate::NativeEffectRegV1>,
    draft_hooks: Vec<crate::DraftHookRegV1>,
    item_build_hooks: Vec<crate::ItemBuildHookRegV1>,
    player_ai: Vec<crate::PlayerAiRegV1>,
}

unsafe extern "C" fn destroy_export(export: *mut ModExportV1) {
    if export.is_null() {
        return;
    }
    drop(Box::from_raw(export as *mut ExportContainer));
}
