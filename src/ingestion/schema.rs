use crate::orchestrator::PlayerAction;
use serde::{Deserialize, Serialize};

/// Player definition for a recorded replay scenario.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReplayPlayer {
    pub name: String,
    pub stack: f64,
    pub hole_cards: Option<[String; 2]>,
}

/// Recorded player action step in a replay log.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReplayActionStep {
    pub street: String,
    pub player_idx: usize,
    pub action: PlayerAction,
}

/// Schema for a pre-recorded tournament hand scenario.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReplayHandScenario {
    pub hand_id: u64,
    pub button_idx: usize,
    pub sb_amount: f64,
    pub bb_amount: f64,
    pub players: Vec<ReplayPlayer>,
    pub board_cards: Vec<String>,
    pub action_script: Vec<ReplayActionStep>,
    pub payouts: Option<Vec<f64>>,
    pub hero_idx: Option<usize>,
}

impl ReplayHandScenario {
    /// Deserializes a scenario from a JSON string.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// Serializes the scenario into a formatted JSON string.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}
