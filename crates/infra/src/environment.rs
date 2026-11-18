use om_core::Environment;

/// Reads real process environment variables.
#[derive(Debug, Default, Clone, Copy)]
pub struct ProcessEnvironment;

impl Environment for ProcessEnvironment {
    fn var(&self, name: &str) -> Option<String> {
        // A non-unicode value cannot be a valid setting; treat it as unset.
        std::env::var(name).ok()
    }
}
