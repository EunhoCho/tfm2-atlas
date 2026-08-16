//! Entry symbols and the shim behind `declare_stable_mod!`.

use std::panic::{catch_unwind, AssertUnwindSafe};

use crate::{HostApiV1, ModExportV1, StableHost, StableMod};

/// Symbol returning the level the mod requires. Checked before entry runs.
pub const MOD_REQUIRED_ABI_LEVEL_SYMBOL: &str = "tfm2_mod_required_abi_level";
/// Stable entry symbol. Present => the DLL is a stable-ABI mod and the
/// loader must not use the legacy exact-match path.
pub const MOD_STABLE_ENTRY_SYMBOL: &str = "tfm2_mod_entry_stable";

pub type ModRequiredAbiLevelFn = unsafe extern "C" fn() -> u32;
pub type ModStableEntryFn = unsafe extern "C" fn(host: *const HostApiV1) -> *mut ModExportV1;

/// Runs a mod init function behind the boundary rules (null check, host size
/// probe, panic isolation). Used by [`crate::declare_stable_mod!`]; not
/// called by mods directly.
pub fn entry_shim(host: *const HostApiV1, init: fn(&StableHost) -> StableMod) -> *mut ModExportV1 {
    if host.is_null() {
        return std::ptr::null_mut();
    }
    // SAFETY: non-null host pointer handed in by the loader; from_raw probes
    // the declared size before any field beyond the prefix is trusted.
    let Some(host) = (unsafe { StableHost::from_raw(host) }) else {
        return std::ptr::null_mut();
    };
    match catch_unwind(AssertUnwindSafe(|| init(&host))) {
        Ok(decl) => decl.into_export_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

/// The level every stable host supports. A mod declares this by default, so
/// building against a newer SDK never costs backward compatibility: calls
/// into slots the running game lacks already degrade per slot (`None` /
/// `false` / no callback), which is the whole point of the size-prefixed
/// tables.
pub const BASELINE_ABI_LEVEL: u32 = 1;

/// Resolves what a mod declares as its minimum. A mod cannot require more
/// than the SDK it was built against describes, so a typo degrades to "needs
/// this SDK's level" instead of locking the mod out of every existing game.
pub const fn resolve_required_level(requested: u32) -> u32 {
    if requested < BASELINE_ABI_LEVEL {
        BASELINE_ABI_LEVEL
    } else if requested > crate::ABI_LEVEL {
        crate::ABI_LEVEL
    } else {
        requested
    }
}

/// Declares the stable-ABI DLL entry points.
///
/// ```ignore
/// fn init(host: &StableHost) -> StableMod {
///     host.log(LogLevel::Info, "hello");
///     StableMod::new("my_mod")
/// }
///
/// mod_api_stable::declare_stable_mod!(init);
/// ```
///
/// The DLL loads on every game version that has the stable loader, and
/// individual calls into newer host features fail gracefully on older games.
///
/// If your mod is *pointless* without a feature the host may not have — it
/// would load and then do nothing — declare the minimum instead, and players
/// on an older game get an "update the game" diagnostic rather than an inert
/// mod:
///
/// ```ignore
/// mod_api_stable::declare_stable_mod!(init, requires = 2);
/// ```
#[macro_export]
macro_rules! declare_stable_mod {
    ($init_fn:ident) => {
        $crate::declare_stable_mod!($init_fn, requires = $crate::BASELINE_ABI_LEVEL);
    };
    ($init_fn:ident, requires = $level:expr) => {
        #[no_mangle]
        pub extern "C" fn tfm2_mod_required_abi_level() -> u32 {
            $crate::resolve_required_level($level)
        }

        #[no_mangle]
        pub unsafe extern "C" fn tfm2_mod_entry_stable(
            host: *const $crate::HostApiV1,
        ) -> *mut $crate::ModExportV1 {
            $crate::entry_shim(host, $init_fn)
        }
    };
}
