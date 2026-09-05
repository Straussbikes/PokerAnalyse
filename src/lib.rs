pub mod api_bridge;
pub mod audit;
pub mod card;
pub mod deck;
pub mod eval;
pub mod ingestion;
pub mod orchestrator;
pub mod range;
pub mod sim;
pub mod tables;
pub mod tournament;

pub use api_bridge::{
    split_hand_blocks, HandHistoryParser, HandHistoryWatcher, ParsedHandHistory, ServerMessage,
    WatcherConfig, WsEventServer,
};
pub use audit::{
    DecisionAudit, LeakSeverity, SessionAuditReport, SessionAuditor, StreetAuditStats,
};
pub use card::{Card, CardParseError, Rank, Suit, RANK_PRIMES};
pub use deck::{DeckBitmask, DeckIterator, FULL_DECK_MASK};
pub use eval::{
    eval_5hand, eval_5hand_array, eval_7hand, eval_holdem, HandRankCategory, COMBOS_7_CHOOSE_5,
};
pub use ingestion::{
    MockReplayAdapter, ReplayActionStep, ReplayHandScenario, ReplayPlayer, ReplayResult,
};
pub use orchestrator::{
    EventBus, GameEvent, HandStateMachine, Player, PlayerAction, Pot, SidePotCalculator,
    SidePotResult, StateError, Street,
};
pub use range::{
    cards_to_combo, classify_combo, combo_to_bitmask, combo_to_cards, CanonicalHand,
    HandStrengthBucket, OpponentAction, OpponentArchetype, Range, RangeParseError, TendencyMatrix,
    ThreeTierBucket, NUM_HOLE_COMBOS,
};
pub use sim::{
    ActionEv, CandidateAction, DecisionReport, EquityResult, EvContext, EvEngine,
    MonteCarloSimulator,
};
pub use tournament::{
    ConfrontationRisk, IcmCalculator, IcmError, RiskPremiumCalculator, RiskPremiumMatrix,
    MAX_ICM_PLAYERS,
};
