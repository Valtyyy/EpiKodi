//! Value, descriptor and patch types shared by the service, the UI and the control API.

use serde::{Deserialize, Serialize};

use crate::AppError;

/// Wire and storage representation of one setting value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub enum SettingValue {
    Bool(bool),
    Int(#[cfg_attr(feature = "ts", ts(type = "number"))] i64),
    Text(String),
    Choice(String),
}

/// Ordered list of `(key, value)` changes, applied atomically.
pub type SettingsPatch = Vec<(String, SettingValue)>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub enum SettingKind {
    Bool,
    Int {
        #[cfg_attr(feature = "ts", ts(type = "number"))]
        min: i64,
        #[cfg_attr(feature = "ts", ts(type = "number"))]
        max: i64,
        #[cfg_attr(feature = "ts", ts(type = "number"))]
        step: i64,
    },
    Choice {
        options: Vec<String>,
    },
    Text {
        #[cfg_attr(feature = "ts", ts(type = "number"))]
        max_len: u32,
    },
}

/// When a changed value starts to matter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub enum ApplyMoment {
    Immediate,
    NextOperation,
    NextPlayback,
    VerifiedRollback,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export, optional_fields))]
pub struct SettingDescriptor {
    pub key: String,
    pub section: String,
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub kind: SettingKind,
    pub default: SettingValue,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    pub apply: ApplyMoment,
    /// Hidden settings are internal state, not shown on the settings screen.
    pub visible: bool,
}

/// Where the effective value of a setting comes from. Priority: env > stored > default.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub enum SettingSource {
    Env,
    Stored,
    Default,
}

/// A value found in the environment or the database that was ignored.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SettingWarning {
    pub key: String,
    pub message: String,
}

impl SettingKind {
    /// Parses an environment string (not JSON). The result still needs `validate`.
    pub fn parse_plain(&self, raw: &str) -> Option<SettingValue> {
        let raw = raw.trim();
        match self {
            Self::Bool => match raw.to_ascii_lowercase().as_str() {
                "true" | "1" => Some(SettingValue::Bool(true)),
                "false" | "0" => Some(SettingValue::Bool(false)),
                _ => None,
            },
            Self::Int { .. } => raw.parse().ok().map(SettingValue::Int),
            Self::Choice { .. } => Some(SettingValue::Choice(raw.to_owned())),
            Self::Text { .. } => Some(SettingValue::Text(raw.to_owned())),
        }
    }

    /// Checks type and bounds. The `suggestion` tells the user what is accepted.
    pub fn validate(&self, key: &str, value: &SettingValue) -> Result<(), AppError> {
        match (self, value) {
            (Self::Bool, SettingValue::Bool(_)) => Ok(()),
            (Self::Int { min, max, step }, SettingValue::Int(v)) => {
                let step = (*step).max(1);
                if v < min || v > max || (v - min) % step != 0 {
                    let hint = if step > 1 {
                        format!("Saisissez un entier entre {min} et {max}, par pas de {step}.")
                    } else {
                        format!("Saisissez un entier entre {min} et {max}.")
                    };
                    Err(AppError::validation(key, "Valeur hors limites.").with_suggestion(hint))
                } else {
                    Ok(())
                }
            }
            (Self::Choice { options }, SettingValue::Choice(v)) => {
                if options.contains(v) {
                    Ok(())
                } else {
                    Err(AppError::validation(key, "Choix inconnu.")
                        .with_suggestion(format!("Valeurs possibles : {}.", options.join(", "))))
                }
            }
            (Self::Text { max_len }, SettingValue::Text(v)) => {
                if v.chars().count() <= *max_len as usize {
                    Ok(())
                } else {
                    Err(AppError::validation(key, "Texte trop long.")
                        .with_suggestion(format!("{max_len} caractères au maximum.")))
                }
            }
            _ => Err(AppError::validation(key, "Type de valeur incorrect.")
                .with_suggestion(self.expected_hint())),
        }
    }

    fn expected_hint(&self) -> String {
        match self {
            Self::Bool => "Une valeur booléenne est attendue.".into(),
            Self::Int { .. } => "Un entier est attendu.".into(),
            Self::Choice { .. } => "Un choix parmi la liste est attendu.".into(),
            Self::Text { .. } => "Un texte est attendu.".into(),
        }
    }
}
