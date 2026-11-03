//! [`Driver`]: the plug between a subject and an animator's parameters.

use super::Params;

/// Writes parameter values from a subject `S`, once per update before the animator runs.
/// Any `FnMut(&S, &mut Params)` closure is a driver.
pub trait Driver<S: ?Sized> {
    /// Set `params` from `subject`: bools and floats every time, triggers when something
    /// happened.
    fn drive(&mut self, subject: &S, params: &mut Params);
}

impl<S: ?Sized, F: FnMut(&S, &mut Params)> Driver<S> for F {
    fn drive(&mut self, subject: &S, params: &mut Params) {
        self(subject, params);
    }
}
