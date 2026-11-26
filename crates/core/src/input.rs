//! Single input source: the living-room keyboard and the phone produce the same
//! `InputAction` (ARCHITECTURE.md section 6).

use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::{UiBridge, UiEvent};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub enum InputAction {
    Up,
    Down,
    Left,
    Right,
    Ok,
    Back,
}

/// Relays actions from other devices to the TV interface, which applies them exactly
/// like the matching living-room key.
#[derive(Clone)]
pub struct InputService {
    bridge: Arc<dyn UiBridge>,
}

impl InputService {
    pub fn new(bridge: Arc<dyn UiBridge>) -> Self {
        Self { bridge }
    }

    pub fn send(&self, action: InputAction) {
        self.bridge.send(UiEvent::Input { action });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::RecordingUiBridge;

    #[test]
    fn actions_reach_the_interface_in_order() {
        let bridge = Arc::new(RecordingUiBridge::default());
        let input = InputService::new(bridge.clone());
        input.send(InputAction::Down);
        input.send(InputAction::Ok);
        assert_eq!(
            bridge.events(),
            vec![
                UiEvent::Input {
                    action: InputAction::Down
                },
                UiEvent::Input {
                    action: InputAction::Ok
                },
            ]
        );
    }

    #[test]
    fn wire_format_matches_the_control_api() {
        let json = serde_json::to_value(UiEvent::Input {
            action: InputAction::Back,
        })
        .unwrap();
        assert_eq!(
            json,
            serde_json::json!({ "type": "input", "action": "back" })
        );
        let parsed: InputAction = serde_json::from_str("\"left\"").unwrap();
        assert_eq!(parsed, InputAction::Left);
    }
}
