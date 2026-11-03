//! [`Animator`]: runs a [`Graph`]: plays the current state's clip and takes transitions.

use std::collections::VecDeque;
use std::sync::Arc;

use super::{Graph, Params, Rate, StateId};

/// How many past states the animator remembers, for debugging.
const HISTORY: usize = 8;

/// Something the animator did in an update, for sounds and effects.
#[derive(Clone, Debug, PartialEq)]
pub enum AnimEvent {
    /// Entered a state.
    Entered(StateId),
    /// A frame with this marker started.
    Marker(Arc<str>),
    /// A once-clip reached its end.
    Finished(StateId),
}

/// One running instance of a graph.
#[derive(Clone, Debug)]
pub struct Animator {
    /// The graph, shared between animators.
    graph: Arc<Graph>,
    /// This animator's parameter values. Drivers write here.
    pub params: Params,
    /// The current state.
    state: StateId,
    /// Seconds in the current state.
    time: f32,
    /// Clip time (seconds scaled by the rate).
    clip_t: f32,
    /// The frame shown.
    frame: usize,
    /// The once-clip has finished.
    finished: bool,
    /// What happened since the last drain.
    events: Vec<AnimEvent>,
    /// Recently left states, newest last, with seconds spent in each.
    history: VecDeque<(StateId, f32)>,
}

impl Animator {
    /// An animator in the graph's entry state.
    pub fn new(graph: Arc<Graph>) -> Self {
        let mut a = Self {
            params: graph.params.clone(),
            state: graph.entry,
            graph,
            time: 0.0,
            clip_t: 0.0,
            frame: 0,
            finished: false,
            events: Vec::new(),
            history: VecDeque::new(),
        };
        a.enter(a.state);
        a
    }

    /// Start over: the graph's entry state, its parameters at their defaults, no pending
    /// triggers, no history. For a character that respawns.
    pub fn reset(&mut self) {
        self.params = self.graph.params.clone();
        self.history.clear();
        // Entering the current state with no time in it leaves no history.
        self.time = 0.0;
        self.state = self.graph.entry;
        self.enter(self.state);
    }

    /// Advance `dt` seconds: play the clip, then take the first transition (by priority)
    /// whose condition holds, consuming the triggers it tested; failing that, follow the
    /// state's `next` if its clip finished. Pending triggers then age.
    pub fn update(&mut self, dt: f32) {
        self.advance(dt);
        if let Some(i) = self.pick() {
            let mut used = Vec::new();
            let tr = &self.graph.transitions[i];
            tr.when.used_triggers(&self.params, &mut used);
            let to = tr.to;
            used.into_iter().for_each(|id| self.params.consume(id));
            self.enter(to);
        } else if self.finished
            && let Some(next) = self.graph.state(self.state).next
        {
            self.enter(next);
        }
        self.params.decay(dt);
    }

    /// Play the clip on by `dt` at the state's rate; report new frames' markers and the
    /// end of once-clips.
    fn advance(&mut self, dt: f32) {
        let st = self.graph.state(self.state);
        let rate = match st.rate {
            Rate::Fixed(r) => r,
            Rate::Param {
                param,
                per,
                min,
                max,
            } => (self.params.get_float(param).abs() / per).clamp(min, max),
        };
        self.time += dt;
        self.clip_t += dt * rate;
        let (frame, finished) = st.clip.frame_at(self.clip_t);
        if frame != self.frame {
            self.frame = frame;
            self.markers();
        }
        if finished && !self.finished {
            self.finished = true;
            self.events.push(AnimEvent::Finished(self.state));
        }
    }

    /// The index of the transition to take now, if any.
    fn pick(&self) -> Option<usize> {
        let locked = self.locked();
        self.graph.transitions.iter().position(|t| {
            t.from.applies(self.state)
                && (!locked || t.breaks_lock)
                && (t.to != self.state || t.restart)
                && t.when.eval(&self.params, self.finished, self.time)
        })
    }

    /// Switch to `to`, from the start of its clip.
    fn enter(&mut self, to: StateId) {
        if self.time > 0.0 || to != self.state {
            self.history.push_back((self.state, self.time));
            if self.history.len() > HISTORY {
                self.history.pop_front();
            }
        }
        self.state = to;
        self.time = 0.0;
        self.clip_t = 0.0;
        self.frame = 0;
        self.finished = false;
        self.events.push(AnimEvent::Entered(to));
        self.markers();
    }

    /// Report the markers on the current frame.
    fn markers(&mut self) {
        let clip = &self.graph.state(self.state).clip;
        for (f, name) in &clip.markers {
            if *f == self.frame {
                self.events.push(AnimEvent::Marker(name.clone()));
            }
        }
    }

    /// The graph.
    pub fn graph(&self) -> &Graph {
        &self.graph
    }

    /// The current state.
    pub fn state(&self) -> StateId {
        self.state
    }

    /// The current state's name.
    pub fn state_name(&self) -> &str {
        self.graph.state(self.state).name.as_ref()
    }

    /// The sprite to draw now.
    pub fn sprite(&self) -> u16 {
        self.graph.state(self.state).clip.sprite(self.frame)
    }

    /// The frame index within the clip.
    pub fn frame(&self) -> usize {
        self.frame
    }

    /// Seconds in the current state.
    pub fn time_in_state(&self) -> f32 {
        self.time
    }

    /// Whether only lock-breaking transitions can leave the state right now.
    pub fn locked(&self) -> bool {
        self.time < self.graph.state(self.state).lock
    }

    /// Recently left states, oldest first, with seconds spent in each.
    pub fn history(&self) -> impl Iterator<Item = (StateId, f32)> + '_ {
        self.history.iter().copied()
    }

    /// Take what happened since the last call.
    pub fn drain_events(&mut self) -> Vec<AnimEvent> {
        std::mem::take(&mut self.events)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::anim::{Clip, Cond, GraphSpec, Playback, StateSpec, TransitionSpec};

    /// idle <-> run by speed; any -> jump on a trigger; jump is a locked once-clip that
    /// returns to idle.
    fn graph() -> Arc<Graph> {
        let c = |s: &[u16]| Clip::uniform(Playback::Loop, s, 0.1);
        Arc::new(
            GraphSpec::new()
                .float("speed")
                .trigger("jump", 0.2)
                .trigger("hit", 0.0)
                .state(StateSpec::new("idle", c(&[0])))
                .state(StateSpec::new("run", c(&[1, 2]).marker(1, "step")))
                .state(
                    StateSpec::new("jump", Clip::uniform(Playback::Once, &[3, 4], 0.1))
                        .lock(0.15)
                        .then("idle"),
                )
                .state(StateSpec::new("hurt", c(&[5])))
                .transition(TransitionSpec::new(
                    ["idle"],
                    "run",
                    Cond::Above("speed".into(), 1.0),
                ))
                .transition(TransitionSpec::new(
                    ["run"],
                    "idle",
                    Cond::Below("speed".into(), 1.0),
                ))
                .transition(TransitionSpec::any("jump", Cond::Trigger("jump".into())).priority(10))
                .transition(
                    TransitionSpec::any("hurt", Cond::Trigger("hit".into()))
                        .priority(20)
                        .breaks_lock(),
                )
                .compile()
                .unwrap(),
        )
    }

    /// The animator's state name after an update with `speed`.
    fn step(a: &mut Animator, speed: f32) -> String {
        let id = a.params.id("speed").unwrap();
        a.params.set_float(id, speed);
        a.update(0.01);
        a.state_name().to_string()
    }

    #[test]
    fn follows_parameters_and_reports_markers() {
        let mut a = Animator::new(graph());
        assert_eq!(step(&mut a, 0.0), "idle");
        assert_eq!(step(&mut a, 5.0), "run");
        a.drain_events();
        for _ in 0..12 {
            step(&mut a, 5.0);
        }
        assert!(a.drain_events().contains(&AnimEvent::Marker("step".into())));
        assert_eq!(step(&mut a, 0.0), "idle");
    }

    #[test]
    fn a_trigger_fired_during_a_lock_waits_for_it() {
        let mut a = Animator::new(graph());
        let jump = a.params.id("jump").unwrap();
        a.params.fire(jump);
        assert_eq!(step(&mut a, 0.0), "jump");
        assert!(!a.params.pending(jump), "the transition consumed it");
        // Fired again while locked: no restart (and no restart flag), but it doesn't block
        // the clip's own way out either.
        a.params.fire(jump);
        assert!(a.locked());
        for _ in 0..25 {
            step(&mut a, 0.0);
        }
        assert_eq!(a.state_name(), "idle");
    }

    #[test]
    fn lock_breakers_cut_in_and_zero_ttl_triggers_expire() {
        let mut a = Animator::new(graph());
        a.params.fire(a.params.id("jump").unwrap());
        step(&mut a, 0.0);
        a.params.fire(a.params.id("hit").unwrap());
        assert_eq!(step(&mut a, 0.0), "hurt");

        let mut b = Animator::new(graph());
        b.params.fire(b.params.id("hit").unwrap());
        b.update(0.0);
        assert_eq!(b.state_name(), "hurt");
        let hit = b.params.id("hit").unwrap();
        b.params.fire(hit);
        b.update(0.01);
        assert!(
            !b.params.pending(hit),
            "unconsumed zero-ttl trigger expired"
        );
    }

    #[test]
    fn once_clips_finish_and_follow_next() {
        let mut a = Animator::new(graph());
        a.params.fire(a.params.id("jump").unwrap());
        step(&mut a, 0.0);
        assert_eq!(a.sprite(), 3);
        for _ in 0..11 {
            step(&mut a, 0.0);
        }
        assert_eq!(a.sprite(), 4);
        for _ in 0..10 {
            step(&mut a, 0.0);
        }
        assert_eq!(a.state_name(), "idle");
    }

    #[test]
    fn reset_starts_over_from_the_entry_state() {
        let mut a = Animator::new(graph());
        let jump = a.params.id("jump").unwrap();
        a.params.fire(jump);
        step(&mut a, 5.0);
        assert!(a.locked());
        a.params.fire(jump);
        a.reset();
        assert_eq!(a.state_name(), "idle");
        assert!(!a.locked());
        assert!(!a.params.pending(jump), "pending triggers are gone");
        assert_eq!(a.history().count(), 0);
        assert!(a.drain_events().contains(&AnimEvent::Entered(a.state())));
        // The speed went back to its default, so it stays idle.
        a.update(0.01);
        assert_eq!(a.state_name(), "idle");
    }
}
