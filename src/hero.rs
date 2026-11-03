//! The hero's binding to the animation controller: which sprites exist ([`sprite`]), the
//! state graph over them ([`graph`]), and the [`HeroDriver`] that turns the simulation into
//! the graph's parameters.
//!
//! Nothing here is required by [`crate::anim`]; another character is another binding.

pub mod driver;
pub mod graph;
pub mod sprite;

pub use driver::HeroDriver;
pub use graph::{graph, spec};
