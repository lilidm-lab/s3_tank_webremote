use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
    Stop,
}

#[derive(Serialize, Deserialize)]
pub struct MoveCmd {
    pub dir: Direction,
}

#[derive(Serialize)]
#[serde(tag = "evt", rename_all = "lowercase")]
pub enum UiEvent {
    Device { online: bool },
    Telemetry { data: serde_json::Value },
}
