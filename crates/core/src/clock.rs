//! Time port.

/// Source of time, injected so tests can control it.
pub trait Clock: Send + Sync {
    /// Milliseconds since the Unix epoch.
    fn now_ms(&self) -> u64;
}
