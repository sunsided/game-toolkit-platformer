//! [`Params`]: named controller inputs: bools, floats and triggers with a time to live.

use super::Name;

/// A parameter, by index into its [`Params`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ParamId(pub(crate) u16);

/// What kind of value a parameter holds.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ParamKind {
    /// On or off, set every update by the driver.
    Bool,
    /// A number, set every update by the driver.
    Float,
    /// A one-shot flag: fired, it stays pending until a transition consumes it or `ttl`
    /// seconds pass. A `ttl` of 0 lives for exactly one update.
    Trigger {
        /// Seconds a fired trigger waits to be consumed.
        ttl: f32,
    },
}

/// One declared parameter.
#[derive(Clone, Debug, PartialEq)]
struct Slot {
    /// Its name.
    name: Name,
    /// Its kind.
    kind: ParamKind,
    /// Bool as 0/1, float as is, trigger as seconds left (> 0: pending).
    value: f32,
}

/// The smallest time left that still counts as pending, so a `ttl` of 0 survives the one
/// update it was fired in.
const PENDING: f32 = 1e-6;

/// A set of named parameters.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Params {
    /// The parameters in declaration order; [`ParamId`] indexes this.
    slots: Vec<Slot>,
}

impl Params {
    /// Declare `name` as `kind` and return its id; declaring an existing name again
    /// returns the existing id (the first kind wins).
    pub fn declare(&mut self, name: impl Into<Name>, kind: ParamKind) -> ParamId {
        let name = name.into();
        if let Some(id) = self.id(&name) {
            return id;
        }
        self.slots.push(Slot {
            name,
            kind,
            value: 0.0,
        });
        ParamId((self.slots.len() - 1) as u16)
    }

    /// The id of `name`, if declared.
    pub fn id(&self, name: &str) -> Option<ParamId> {
        self.slots
            .iter()
            .position(|s| s.name == name)
            .map(|i| ParamId(i as u16))
    }

    /// The name of `id`.
    pub fn name(&self, id: ParamId) -> &str {
        &self.slots[id.0 as usize].name
    }

    /// The kind of `id`.
    pub fn kind(&self, id: ParamId) -> ParamKind {
        self.slots[id.0 as usize].kind
    }

    /// Set a bool parameter.
    pub fn set_bool(&mut self, id: ParamId, v: bool) {
        self.slots[id.0 as usize].value = if v { 1.0 } else { 0.0 };
    }

    /// Set a float parameter.
    pub fn set_float(&mut self, id: ParamId, v: f32) {
        self.slots[id.0 as usize].value = v;
    }

    /// Fire a trigger: pending for its time to live (re-firing restarts it).
    pub fn fire(&mut self, id: ParamId) {
        let slot = &mut self.slots[id.0 as usize];
        if let ParamKind::Trigger { ttl } = slot.kind {
            slot.value = ttl.max(PENDING);
        }
    }

    /// A bool parameter's value.
    pub fn get_bool(&self, id: ParamId) -> bool {
        self.slots[id.0 as usize].value != 0.0
    }

    /// A float parameter's value.
    pub fn get_float(&self, id: ParamId) -> f32 {
        self.slots[id.0 as usize].value
    }

    /// Whether a trigger is pending.
    pub fn pending(&self, id: ParamId) -> bool {
        self.slots[id.0 as usize].value > 0.0
    }

    /// Set every value back to `defaults`' (by position); parameters beyond those, which
    /// a driver declared on its own, go back to 0.
    pub(crate) fn reset_to(&mut self, defaults: &Params) {
        for (i, s) in self.slots.iter_mut().enumerate() {
            s.value = defaults.slots.get(i).map_or(0.0, |d| d.value);
        }
    }

    /// Clear a pending trigger: a transition used it.
    pub(crate) fn consume(&mut self, id: ParamId) {
        self.slots[id.0 as usize].value = 0.0;
    }

    /// Age pending triggers by `dt`; expired ones are dropped.
    pub(crate) fn decay(&mut self, dt: f32) {
        for s in &mut self.slots {
            if matches!(s.kind, ParamKind::Trigger { .. }) {
                s.value = (s.value - dt).max(0.0);
                if s.value < PENDING {
                    s.value = 0.0;
                }
            }
        }
    }

    /// Every parameter: id, name, kind and raw value (bools 0/1, triggers time left).
    pub fn iter(&self) -> impl Iterator<Item = (ParamId, &str, ParamKind, f32)> {
        self.slots
            .iter()
            .enumerate()
            .map(|(i, s)| (ParamId(i as u16), s.name.as_ref(), s.kind, s.value))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn triggers_wait_their_ttl_then_expire() {
        let mut p = Params::default();
        let t = p.declare("jump", ParamKind::Trigger { ttl: 0.1 });
        p.fire(t);
        p.decay(0.05);
        assert!(p.pending(t));
        p.decay(0.06);
        assert!(!p.pending(t));
    }

    #[test]
    fn a_zero_ttl_trigger_lives_one_update() {
        let mut p = Params::default();
        let t = p.declare("land", ParamKind::Trigger { ttl: 0.0 });
        p.fire(t);
        assert!(p.pending(t));
        p.decay(1.0 / 120.0);
        assert!(!p.pending(t));
    }

    #[test]
    fn redeclaring_returns_the_same_id() {
        let mut p = Params::default();
        let a = p.declare("x", ParamKind::Float);
        assert_eq!(p.declare("x", ParamKind::Bool), a);
        assert_eq!(p.kind(a), ParamKind::Float);
    }
}
