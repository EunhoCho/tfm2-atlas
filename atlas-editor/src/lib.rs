use mod_api_stable::{declare_stable_mod, StableHost, StableMod};

fn init(host: &StableHost) -> StableMod {
    tfm2_atlas_core::create_editor_mod(host)
}

declare_stable_mod!(init, requires = 3);
