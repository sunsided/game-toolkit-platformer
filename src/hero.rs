//! The hero's binding to the animation controller: which sprites exist ([`sprite`]), the
//! state graph over them ([`graph`]), and the [`HeroDriver`] that turns the simulation into
//! the graph's parameters.
//!
//! Nothing here is required by [`crate::anim`]; another character is another binding.
//!
//! The loop, each fixed step: the simulation, then the driver, then the animator.
//!
//! ```
//! use game_toolkit_platformer::anim::{Animator, Driver};
//! use game_toolkit_platformer::hero::{HeroDriver, graph};
//! use game_toolkit_platformer::sim::{DT, Intent, Level, Tuning, World};
//!
//! let mut level = Level::demo();
//! let mut world = World::on(&level, Tuning::default());
//! let mut animator = Animator::new(graph());
//! let mut driver = HeroDriver::new(&animator.params);
//! for _ in 0..30 {
//!     world.step(&mut level, &Intent { move_x: 1.0, ..Intent::default() });
//!     driver.drive(&world, &mut animator.params);
//!     animator.update(DT);
//! }
//! assert_eq!(animator.state_name(), "run");
//! let _sprite = animator.sprite(); // what to draw
//! ```

pub mod driver;
pub mod graph;
pub mod sprite;

pub use driver::HeroDriver;
pub use graph::{graph, spec};
