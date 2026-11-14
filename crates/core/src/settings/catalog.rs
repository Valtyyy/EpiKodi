//! The single declaration of every setting. Adding a setting means adding one
//! entry to `settings_catalog!`: the typed struct, its defaults, the
//! descriptors and the key-based validation are all generated from it.

use super::model::{ApplyMoment, SettingDescriptor, SettingKind, SettingValue};
use crate::AppError;

/// Conversion between a typed field and its wire value.
pub trait FieldType: Sized {
    fn from_value(value: &SettingValue) -> Option<Self>;
    fn to_value(&self) -> SettingValue;
}

impl FieldType for bool {
    fn from_value(value: &SettingValue) -> Option<Self> {
        match value {
            SettingValue::Bool(b) => Some(*b),
            _ => None,
        }
    }
    fn to_value(&self) -> SettingValue {
        SettingValue::Bool(*self)
    }
}

impl FieldType for u32 {
    fn from_value(value: &SettingValue) -> Option<Self> {
        match value {
            SettingValue::Int(i) => u32::try_from(*i).ok(),
            _ => None,
        }
    }
    fn to_value(&self) -> SettingValue {
        SettingValue::Int(i64::from(*self))
    }
}

impl FieldType for String {
    fn from_value(value: &SettingValue) -> Option<Self> {
        match value {
            SettingValue::Text(s) | SettingValue::Choice(s) => Some(s.clone()),
            _ => None,
        }
    }
    fn to_value(&self) -> SettingValue {
        SettingValue::Text(self.clone())
    }
}

macro_rules! settings_catalog {
    ($(
        $field:ident : $ty:ty = $default:expr => {
            key: $key:literal,
            section: $section:literal,
            label: $label:literal,
            description: $desc:expr,
            kind: $kind:expr,
            unit: $unit:expr,
            apply: $apply:ident,
            visible: $visible:literal $(,)?
        }
    );+ $(;)?) => {
        /// Effective typed settings, read by services through a `SettingsView`.
        #[derive(Debug, Clone, PartialEq)]
        pub struct Settings {
            $(pub $field: $ty,)+
        }

        impl Default for Settings {
            fn default() -> Self {
                Self { $($field: $default,)+ }
            }
        }

        impl Settings {
            /// Every declared setting, in declaration order.
            pub fn descriptors() -> Vec<SettingDescriptor> {
                let defaults = Self::default();
                vec![$(SettingDescriptor {
                    key: $key.into(),
                    section: $section.into(),
                    label: $label.into(),
                    description: $desc.map(Into::into),
                    kind: $kind,
                    default: FieldType::to_value(&defaults.$field),
                    unit: $unit.map(Into::into),
                    apply: ApplyMoment::$apply,
                    visible: $visible,
                }),+]
            }

            /// Current value of `key` as a wire value, `None` for an unknown key.
            pub fn get(&self, key: &str) -> Option<SettingValue> {
                match key {
                    $($key => Some(FieldType::to_value(&self.$field)),)+
                    _ => None,
                }
            }

            /// Validates `value` against the descriptor of `key`, then stores it.
            pub fn set(&mut self, key: &str, value: &SettingValue) -> Result<(), AppError> {
                match key {
                    $($key => {
                        let kind: SettingKind = $kind;
                        kind.validate(key, value)?;
                        self.$field = <$ty as FieldType>::from_value(value).ok_or_else(|| {
                            AppError::validation(key, "Type de valeur incorrect.")
                        })?;
                        Ok(())
                    })+
                    _ => Err(AppError::validation(key, "Réglage inconnu.")),
                }
            }
        }
    };
}

settings_catalog! {
    window_decorations: bool = true => {
        key: "interface.window_decorations",
        section: "Interface",
        label: "Barre de titre de la fenêtre",
        description: Some("Affiche la barre de titre du gestionnaire de fenêtres."),
        kind: SettingKind::Bool,
        unit: None::<&str>,
        apply: Immediate,
        visible: true,
    };
    network_timeout_ms: u32 = 10_000 => {
        key: "network.timeout_ms",
        section: "Réseau",
        label: "Délai d'attente réseau",
        description: Some("Durée maximale d'attente d'une réponse réseau."),
        kind: SettingKind::Int { min: 1_000, max: 60_000, step: 500 },
        unit: Some("ms"),
        apply: NextOperation,
        visible: true,
    };
    scan_parallelism: u32 = 2 => {
        key: "library.scan_parallelism",
        section: "Bibliothèque",
        label: "Analyses en parallèle",
        description: Some("Nombre de dossiers analysés simultanément."),
        kind: SettingKind::Int { min: 1, max: 8, step: 1 },
        unit: None::<&str>,
        apply: NextOperation,
        visible: true,
    };
    hardware_decoding: bool = true => {
        key: "playback.hardware_decoding",
        section: "Lecture",
        label: "Décodage matériel",
        description: Some("Utilise le processeur graphique pour décoder la vidéo."),
        kind: SettingKind::Bool,
        unit: None::<&str>,
        apply: NextPlayback,
        visible: true,
    };
    first_run_completed: bool = false => {
        key: "setup.first_run_completed",
        section: "Configuration",
        label: "Premier lancement terminé",
        description: None::<&str>,
        kind: SettingKind::Bool,
        unit: None::<&str>,
        apply: Immediate,
        visible: false,
    };
}
