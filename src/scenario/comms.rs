//! The intercepted-comms feed: queued lines, the typewriter, hold + fade.

use std::collections::VecDeque;

/// Typewriter speed of the comms feed, characters per second.
pub const COMMS_CHARS_PER_SEC: f32 = 38.0;
/// Pause between the end of one line's typing and the next line starting.
pub const COMMS_LINE_GAP: f32 = 0.35;
/// How long a fully shown line stays before it starts to fade.
pub const COMMS_HOLD_SECS: f32 = 9.0;
/// Fade-out duration after the hold.
pub const COMMS_FADE_SECS: f32 = 1.5;
/// Maximum number of lines kept on screen.
pub const COMMS_MAX_VISIBLE: usize = 4;

/// A comms line waiting for its turn.
#[derive(Debug, Clone, PartialEq)]
struct QueuedLine {
    who: &'static str,
    text: &'static str,
    /// Absolute scenario time before which the line must not start.
    not_before: f32,
}

/// A comms line on screen (typing, holding, or fading).
#[derive(Debug, Clone, PartialEq)]
pub struct CommsLine {
    pub who: &'static str,
    pub text: &'static str,
    /// Seconds since the line started playing.
    pub age: f32,
}

impl CommsLine {
    /// Number of characters revealed by the typewriter so far.
    pub fn chars_shown(&self) -> usize {
        let n = (self.age * COMMS_CHARS_PER_SEC) as usize;
        n.min(self.text.chars().count())
    }

    /// Seconds needed to type the whole line.
    pub fn typing_time(&self) -> f32 {
        self.text.chars().count() as f32 / COMMS_CHARS_PER_SEC
    }

    pub fn fully_typed(&self) -> bool {
        self.age >= self.typing_time()
    }

    /// Opacity 0..1 (1 while typing/holding, fading to 0 afterwards).
    pub fn alpha(&self) -> f32 {
        let hold_end = self.typing_time() + COMMS_HOLD_SECS;
        if self.age <= hold_end {
            1.0
        } else {
            (1.0 - (self.age - hold_end) / COMMS_FADE_SECS).clamp(0.0, 1.0)
        }
    }

    pub fn expired(&self) -> bool {
        self.age > self.typing_time() + COMMS_HOLD_SECS + COMMS_FADE_SECS
    }
}

/// The intercepted-comms feed: a queue of pending lines that play strictly one
/// after another (each waits for its own delay *and* for the previous line to
/// finish typing), plus the lines currently on screen.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct CommsFeed {
    queue: VecDeque<QueuedLine>,
    visible: Vec<CommsLine>,
    /// Scenario time at which the currently playing line finishes typing.
    busy_until: f32,
}

impl CommsFeed {
    pub(super) fn enqueue(&mut self, who: &'static str, text: &'static str, not_before: f32) {
        self.queue.push_back(QueuedLine {
            who,
            text,
            not_before,
        });
    }

    pub(super) fn update(&mut self, now: f32, dt: f32) {
        for line in &mut self.visible {
            line.age += dt;
        }
        self.visible.retain(|l| !l.expired());

        // Start the next line once its delay has elapsed and the feed is idle.
        while let Some(head) = self.queue.front() {
            if now < head.not_before || now < self.busy_until {
                break;
            }
            let head = self.queue.pop_front().unwrap();
            let line = CommsLine {
                who: head.who,
                text: head.text,
                age: 0.0,
            };
            self.busy_until = now + line.typing_time() + COMMS_LINE_GAP;
            self.visible.push(line);
            if self.visible.len() > COMMS_MAX_VISIBLE {
                self.visible.remove(0);
            }
        }
    }

    /// Lines currently on screen, oldest first.
    pub fn visible(&self) -> &[CommsLine] {
        &self.visible
    }

    /// Lines still waiting to play.
    pub fn pending(&self) -> usize {
        self.queue.len()
    }

    /// Whether anything is queued or still typing.
    pub fn is_active(&self, now: f32) -> bool {
        !self.queue.is_empty() || now < self.busy_until
    }
}
