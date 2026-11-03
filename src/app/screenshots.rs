//! [`Screenshots`]: saving what the window shows, through the toolkit's
//! `Graphics::request_screenshot` (a read-back of the presented frame by miniscreenshot-wgpu).

use std::path::{Path, PathBuf};

/// Where F12 saves screenshots, relative to the working directory.
pub const DIR: &str = "screenshots";

/// Screenshot bookkeeping: the one asked for on the command line, and the last one taken
/// (to report it a frame later, so the message isn't in the picture).
#[derive(Default)]
pub struct Screenshots {
    /// From the command line: save the window to this path once the app has run this many
    /// seconds, then quit.
    pub auto: Option<(PathBuf, f32)>,
    /// A screenshot requested this frame, to report next frame.
    pub taken: Option<PathBuf>,
    /// The automatic screenshot was taken: quit next frame (after it was presented and
    /// saved).
    pub quit: bool,
}

impl Screenshots {
    /// The automatic screenshot's path, if it is due at app time `clock` (once).
    pub fn due(&mut self, clock: f32) -> Option<PathBuf> {
        match &self.auto {
            Some((_, at)) if clock >= *at => {
                let (path, _) = self.auto.take()?;
                self.quit = true;
                Some(path)
            }
            _ => None,
        }
    }

    /// The first free `screenshots/<name>-NNN.png`, creating the folder if needed.
    pub fn next_path(name: &str) -> PathBuf {
        let _ = std::fs::create_dir_all(DIR);
        (1..)
            .map(|n| Path::new(DIR).join(format!("{name}-{n:03}.png")))
            .find(|p| !p.exists())
            .expect("a free file name")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_automatic_shot_is_due_once() {
        let mut s = Screenshots {
            auto: Some(("x.png".into(), 2.0)),
            ..Screenshots::default()
        };
        assert_eq!(s.due(1.0), None);
        assert_eq!(s.due(2.5), Some(PathBuf::from("x.png")));
        assert!(s.quit);
        assert_eq!(s.due(3.0), None);
    }
}
