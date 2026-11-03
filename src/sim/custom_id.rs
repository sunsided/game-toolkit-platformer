//! [`CustomId`]: the name of a move or event of your own, for [`Mode::Custom`] and
//! [`Event::Custom`].
//!
//! An id is a hash of a name you choose, worked out at compile time, so two abilities from
//! different crates never have to agree on numbers: name them after their crate and move
//! (`"my_crate::lunge"`) and they don't collide. The same name always gives the same id,
//! in every build and on every machine, so replays and cloned worlds stay deterministic.
//!
//! ```
//! use game_toolkit_platformer::sim::CustomId;
//!
//! const LUNGE: CustomId = CustomId::new("my_game::lunge");
//! const POGO: CustomId = CustomId::new("pogo_crate::pogo");
//! assert_ne!(LUNGE, POGO);
//! assert_eq!(LUNGE, CustomId::new("my_game::lunge"));
//! ```
//!
//! [`Mode::Custom`]: super::Mode::Custom
//! [`Event::Custom`]: super::Event::Custom

/// The id of a move or event of your own: a 64-bit hash of its name (see the module docs).
/// Usable in patterns: `Mode::Custom { id: LUNGE, .. }`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CustomId(pub u64);

impl CustomId {
    /// The id named `name` (FNV-1a over its bytes). Prefix it with your crate's name.
    pub const fn new(name: &str) -> Self {
        let bytes = name.as_bytes();
        let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
        let mut i = 0;
        while i < bytes.len() {
            hash ^= bytes[i] as u64;
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
            i += 1;
        }
        Self(hash)
    }
}
