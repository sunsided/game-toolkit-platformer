//! [`Contacts`]: what the body ran into in its last step, for an
//! [`Ability::after`](super::Ability::after) that reacts to collisions (a wall-hit stun, a
//! ceiling bonk that breaks a block, a ground pound).

/// What the body's own movement ran into in the last step (not what moving ground did).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Contacts {
    /// A wall stopped it sideways: -1 on the left, 1 on the right, 0 none.
    pub wall: i32,
    /// Its head hit a ceiling ([`Event::Bonked`](super::Event::Bonked)).
    pub ceiling: bool,
    /// It touched down ([`Event::Landed`](super::Event::Landed)).
    pub floor: bool,
}
