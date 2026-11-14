//! `Announcer` port for the Orca spike (spike 2.5, issue #14).
//!
//! The web page is expected to announce through its `aria-live` regions and
//! Orca reads them through AT-SPI. `NoopAnnouncer` is that default path.
//! `SpeechDispatcherAnnouncer` is the fallback: it speaks through the
//! `spd-say` command (speech-dispatcher client), one subprocess per message.
//! The `speech-dispatcher` crate was not used because `libspeechd` headers
//! (`pkg-config speech-dispatcher`) are not installed on the spike host.

use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

/// How urgently a message should be spoken.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Priority {
    /// Equivalent of `aria-live="polite"`: waits for the current speech.
    Polite,
    /// Equivalent of `aria-live="assertive"`: should be heard first.
    Assertive,
}

/// Port: anything that can tell the user something.
pub trait Announcer {
    /// Announce `text`. Failures are not reported to the caller (the page
    /// must not break because speech is unavailable); implementations may
    /// count them instead.
    fn announce(&self, text: &str, priority: Priority);
}

/// Announces nothing: the page's `aria-live` regions are the channel.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoopAnnouncer;

impl Announcer for NoopAnnouncer {
    fn announce(&self, _text: &str, _priority: Priority) {}
}

/// Speaks through `spd-say` (speech-dispatcher), waiting for the spawned
/// process to return. Without `-w`, `spd-say` returns as soon as the server
/// has queued the message, so the measured latency is the queueing cost.
#[derive(Debug)]
pub struct SpeechDispatcherAnnouncer {
    program: String,
    failures: AtomicU64,
}

impl Default for SpeechDispatcherAnnouncer {
    fn default() -> Self {
        Self::new()
    }
}

impl SpeechDispatcherAnnouncer {
    /// Uses `spd-say` from `PATH`.
    pub fn new() -> Self {
        Self::with_program("spd-say")
    }

    /// Uses another program with the same argument convention as `spd-say`.
    /// Benchmarks use this to measure the spawn cost alone (`/bin/true`).
    pub fn with_program(program: impl Into<String>) -> Self {
        Self {
            program: program.into(),
            failures: AtomicU64::new(0),
        }
    }

    /// Number of `announce` calls whose subprocess failed to start or exited non-zero.
    pub fn failures(&self) -> u64 {
        self.failures.load(Ordering::Relaxed)
    }
}

/// Maps a `Priority` to the speech-dispatcher priority name used by `spd-say -P`.
pub fn spd_priority_name(priority: Priority) -> &'static str {
    match priority {
        Priority::Polite => "message",
        Priority::Assertive => "important",
    }
}

impl Announcer for SpeechDispatcherAnnouncer {
    fn announce(&self, text: &str, priority: Priority) {
        let status = Command::new(&self.program)
            .args(["-N", "orca-spike", "-P", spd_priority_name(priority), "--", text])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        match status {
            Ok(s) if s.success() => {}
            _ => {
                self.failures.fetch_add(1, Ordering::Relaxed);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn priorities_map_to_speech_dispatcher_names() {
        assert_eq!(spd_priority_name(Priority::Polite), "message");
        assert_eq!(spd_priority_name(Priority::Assertive), "important");
    }

    #[test]
    fn noop_announcer_accepts_anything() {
        NoopAnnouncer.announce("", Priority::Polite);
        NoopAnnouncer.announce("x", Priority::Assertive);
    }

    #[test]
    fn failing_program_is_counted_not_raised() {
        let a = SpeechDispatcherAnnouncer::with_program("/nonexistent/spd-say-spike");
        a.announce("hello", Priority::Polite);
        assert_eq!(a.failures(), 1);
    }

    #[test]
    fn successful_program_is_not_counted() {
        let a = SpeechDispatcherAnnouncer::with_program("/bin/true");
        a.announce("hello", Priority::Assertive);
        assert_eq!(a.failures(), 0);
    }
}
