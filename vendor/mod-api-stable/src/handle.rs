//! Opaque `u64` handles.
//!
//! Handles are ids, never pointers: low 32 bits index/id, high 32 bits a
//! generation stamp so stale handles are detected instead of dereferencing
//! freed memory. `0` is always the null handle. Every contract function must
//! be safe to call with an invalid handle (queries fail, mutations no-op).
//!
//! Handles are only guaranteed valid inside the callback that produced them.
//! Long-lived references use domain string/integer ids re-resolved per call.

macro_rules! stable_handle {
    ($(#[$doc:meta])* $name:ident) => {
        $(#[$doc])*
        #[repr(transparent)]
        #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
        pub struct $name(pub u64);

        impl $name {
            pub const NULL: Self = Self(0);

            pub const fn from_raw(raw: u64) -> Self {
                Self(raw)
            }

            pub const fn raw(self) -> u64 {
                self.0
            }

            pub const fn is_null(self) -> bool {
                self.0 == 0
            }

            /// Compose a handle from index and generation. The host side is
            /// the only producer; mods treat handles as opaque.
            pub const fn from_parts(index: u32, generation: u32) -> Self {
                Self(((generation as u64) << 32) | index as u64)
            }

            pub const fn index(self) -> u32 {
                self.0 as u32
            }

            pub const fn generation(self) -> u32 {
                (self.0 >> 32) as u32
            }

            /// Encode a plain id/index as a handle (`id + 1`; 0 stays null).
            /// Sim handles use this scheme in v1; the generation field is
            /// reserved for future domains.
            pub const fn from_id(id: usize) -> Self {
                Self(id as u64 + 1)
            }

            /// Decode the id/index encoded by [`Self::from_id`].
            pub fn id(self) -> Option<usize> {
                if self.0 == 0 {
                    None
                } else {
                    Some((self.0 - 1) as usize)
                }
            }
        }
    };
}

stable_handle!(
    /// In-game simulation entity (champion, minion, tower, ...).
    EntityHandleV1
);
stable_handle!(
    /// Player (the controller behind a champion) in a running simulation.
    PlayerHandleV1
);
stable_handle!(
    /// Active projectile in a running simulation.
    ProjectileHandleV1
);
