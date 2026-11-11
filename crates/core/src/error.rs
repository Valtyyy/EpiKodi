//! Common error shape shared by Tauri commands and the control API.

use serde::{Deserialize, Serialize};

/// Serialised as `{ code, message, field?, suggestion? }`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[error("{code}: {message}")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export, optional_fields))]
pub struct AppError {
    pub code: String,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
}

impl AppError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            field: None,
            suggestion: None,
        }
    }

    /// Validation failure attached to a specific input field.
    pub fn validation(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new("validation", message).with_field(field)
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::new("internal", message)
    }

    #[must_use]
    pub fn with_field(mut self, field: impl Into<String>) -> Self {
        self.field = Some(field.into());
        self
    }

    #[must_use]
    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestion = Some(suggestion.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn optional_fields_are_skipped() {
        let json = serde_json::to_string(&AppError::new("x", "msg")).unwrap();
        assert_eq!(json, r#"{"code":"x","message":"msg"}"#);
    }

    #[test]
    fn full_error_roundtrips() {
        let e = AppError::validation("name", "bad").with_suggestion("try again");
        let back: AppError = serde_json::from_str(&serde_json::to_string(&e).unwrap()).unwrap();
        assert_eq!(e, back);
        assert_eq!(e.field.as_deref(), Some("name"));
    }
}
