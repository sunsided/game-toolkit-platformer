//! [`Clip`]: a sequence of frames with durations, a playback mode and named markers.

use std::sync::Arc;

/// How a clip continues past its last frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Playback {
    /// Start over.
    Loop,
    /// Hold the last frame and report the clip finished.
    Once,
    /// Play backwards to the start, then forwards again (end frames are not doubled).
    PingPong,
}

/// One frame: which sprite to show, and for how long at rate 1.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Frame {
    /// Sprite index; what it means is up to the renderer (an atlas cell, a pose).
    pub sprite: u16,
    /// Seconds.
    pub secs: f32,
}

/// An animation clip.
#[derive(Clone, Debug, PartialEq)]
pub struct Clip {
    /// The frames in order. Never empty.
    pub frames: Vec<Frame>,
    /// What happens after the last frame.
    pub playback: Playback,
    /// Named markers, reported when their frame starts: `(frame index, name)`.
    pub markers: Vec<(usize, Arc<str>)>,
}

impl Clip {
    /// A clip of `(sprite, seconds)` frames.
    pub fn new(playback: Playback, frames: &[(u16, f32)]) -> Self {
        assert!(!frames.is_empty(), "a clip needs at least one frame");
        Self {
            frames: frames
                .iter()
                .map(|&(sprite, secs)| Frame { sprite, secs })
                .collect(),
            playback,
            markers: Vec::new(),
        }
    }

    /// A clip of `sprites`, each shown for `secs`.
    pub fn uniform(playback: Playback, sprites: &[u16], secs: f32) -> Self {
        let frames: Vec<_> = sprites.iter().map(|&s| (s, secs)).collect();
        Self::new(playback, &frames)
    }

    /// The same clip with a marker `name` on frame `frame`.
    pub fn marker(mut self, frame: usize, name: &str) -> Self {
        self.markers.push((frame, name.into()));
        self
    }

    /// Seconds for one pass through the frames.
    pub fn duration(&self) -> f32 {
        self.frames.iter().map(|f| f.secs).sum()
    }

    /// The frame index shown at clip time `t`, and whether a [`Playback::Once`] clip has
    /// finished.
    pub fn frame_at(&self, t: f32) -> (usize, bool) {
        let n = self.frames.len();
        let total = self.duration();
        if n == 1 || total <= 0.0 {
            return (0, self.playback == Playback::Once && t >= total);
        }
        match self.playback {
            Playback::Once if t >= total => (n - 1, true),
            Playback::Once => (self.index_in(&(0..n).collect::<Vec<_>>(), t), false),
            Playback::Loop => (self.index_in(&(0..n).collect::<Vec<_>>(), t % total), false),
            Playback::PingPong => {
                let order: Vec<usize> = (0..n).chain((1..n - 1).rev()).collect();
                let cycle: f32 = order.iter().map(|&i| self.frames[i].secs).sum();
                (self.index_in(&order, t % cycle), false)
            }
        }
    }

    /// The frame of `order` (frame indices, played in turn) shown `t` seconds in.
    fn index_in(&self, order: &[usize], mut t: f32) -> usize {
        for &i in order {
            t -= self.frames[i].secs;
            if t < 0.0 {
                return i;
            }
        }
        order[order.len() - 1]
    }

    /// The sprite of frame `i`.
    pub fn sprite(&self, i: usize) -> u16 {
        self.frames[i.min(self.frames.len() - 1)].sprite
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn once_holds_the_last_frame_and_finishes() {
        let c = Clip::uniform(Playback::Once, &[1, 2, 3], 0.1);
        assert_eq!(c.frame_at(0.15), (1, false));
        assert_eq!(c.frame_at(0.31), (2, true));
    }

    #[test]
    fn loop_wraps() {
        let c = Clip::uniform(Playback::Loop, &[1, 2], 0.1);
        assert_eq!(c.frame_at(0.25).0, 0);
    }

    #[test]
    fn ping_pong_turns_without_doubling_the_ends() {
        let c = Clip::uniform(Playback::PingPong, &[1, 2, 3], 0.1);
        let seq: Vec<usize> = (0..6)
            .map(|k| c.frame_at(k as f32 * 0.1 + 0.05).0)
            .collect();
        assert_eq!(seq, [0, 1, 2, 1, 0, 1]);
    }
}
