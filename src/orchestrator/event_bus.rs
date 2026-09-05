use crate::card::Card;
use crate::orchestrator::pot::Pot;
use serde::{Deserialize, Serialize};

/// Betting street in Texas Hold'em.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Street {
    Preflop,
    Flop,
    Turn,
    River,
    Showdown,
    HandEnded,
}

impl Street {
    pub const fn next(self) -> Self {
        match self {
            Street::Preflop => Street::Flop,
            Street::Flop => Street::Turn,
            Street::Turn => Street::River,
            Street::River => Street::Showdown,
            Street::Showdown | Street::HandEnded => Street::HandEnded,
        }
    }
}

/// Standard poker actions executed by a player.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "amount")]
pub enum PlayerAction {
    Fold,
    Check,
    Call(f64),
    Bet(f64),
    Raise(f64),
    AllIn(f64),
}

/// Standardized domain events emitted by the game state machine.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "event", content = "data")]
pub enum GameEvent {
    HandStarted {
        hand_id: u64,
        button_idx: usize,
        sb_amount: f64,
        bb_amount: f64,
    },
    BlindsPosted {
        sb_player: usize,
        sb_amount: f64,
        bb_player: usize,
        bb_amount: f64,
    },
    HoleCardsDealt {
        player_idx: usize,
        cards: [Card; 2],
    },
    StreetAdvanced {
        street: Street,
        board: Vec<Card>,
        pot_size: f64,
    },
    HeroToAct {
        player_idx: usize,
        call_cost: f64,
        current_pot: f64,
        min_raise: f64,
    },
    PlayerActed {
        player_idx: usize,
        action: PlayerAction,
        amount_committed: f64,
        remaining_stack: f64,
    },
    PotsAwarded {
        pots: Vec<Pot>,
        payouts: Vec<(usize, f64)>,
    },
    HandComplete {
        final_stacks: Vec<f64>,
    },
}

/// Simple pub/sub event bus dispatcher for decoupling engine mutations from subscribers.
#[derive(Default)]
pub struct EventBus {
    history: Vec<GameEvent>,
}

impl EventBus {
    pub fn new() -> Self {
        EventBus {
            history: Vec::new(),
        }
    }

    /// Publishes an event and logs it in the hand event history.
    pub fn publish(&mut self, event: GameEvent) {
        self.history.push(event);
    }

    /// Returns a slice of all events emitted during this hand.
    pub fn events(&self) -> &[GameEvent] {
        &self.history
    }

    /// Clears recorded history for the start of a new hand.
    pub fn clear(&mut self) {
        self.history.clear();
    }
}
