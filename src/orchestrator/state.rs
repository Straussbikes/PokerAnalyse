use crate::card::Card;
use crate::deck::DeckBitmask;
use crate::eval::eval_7hand;
use crate::orchestrator::event_bus::{EventBus, GameEvent, PlayerAction, Street};
use crate::orchestrator::pot::SidePotCalculator;
use core::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum StateError {
    InvalidAction(String),
    NotPlayerTurn { expected: usize, actual: usize },
    HandAlreadyFinished,
    InsufficientChips { needed: f64, available: f64 },
    PlayerNotFound(usize),
}

impl fmt::Display for StateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StateError::InvalidAction(msg) => write!(f, "Invalid action: {msg}"),
            StateError::NotPlayerTurn { expected, actual } => {
                write!(f, "Out of turn action: player {actual} acted, expected {expected}")
            }
            StateError::HandAlreadyFinished => write!(f, "Hand has already concluded"),
            StateError::InsufficientChips { needed, available } => {
                write!(f, "Insufficient chips: needed {needed}, available {available}")
            }
            StateError::PlayerNotFound(id) => write!(f, "Player with id {id} not found"),
        }
    }
}

impl std::error::Error for StateError {}

/// State of an individual player seated at the table.
#[derive(Debug, Clone, PartialEq)]
pub struct Player {
    pub id: usize,
    pub name: String,
    pub stack: f64,
    pub hole_cards: Option<[Card; 2]>,
    /// Chips bet on the current street.
    pub street_bet: f64,
    /// Total chips committed across the entire hand.
    pub total_committed: f64,
    /// Whether the player is still in the hand (has not folded).
    pub is_active: bool,
    /// Whether the player is all-in.
    pub is_all_in: bool,
    /// Whether the player has completed an action on this street.
    pub has_acted: bool,
}

impl Player {
    pub fn new(id: usize, name: impl Into<String>, stack: f64) -> Self {
        Player {
            id,
            name: name.into(),
            stack,
            hole_cards: None,
            street_bet: 0.0,
            total_committed: 0.0,
            is_active: true,
            is_all_in: stack <= 0.0,
            has_acted: false,
        }
    }
}

/// Comprehensive Game State Machine managing betting, pot tracking, street transitions, and showdowns.
pub struct HandStateMachine {
    pub hand_id: u64,
    pub players: Vec<Player>,
    pub button_idx: usize,
    pub sb_idx: usize,
    pub bb_idx: usize,
    pub street: Street,
    pub board: Vec<Card>,
    pub dead_cards: DeckBitmask,
    pub current_bet: f64,
    pub min_raise: f64,
    pub action_idx: usize,
    pub sb_amount: f64,
    pub bb_amount: f64,
    pub event_bus: EventBus,
}

impl HandStateMachine {
    /// Initializes a new hand state machine with players, positions, and blind structures.
    pub fn new(
        hand_id: u64,
        players: Vec<Player>,
        button_idx: usize,
        sb_amount: f64,
        bb_amount: f64,
    ) -> Result<Self, StateError> {
        let n = players.len();
        if n < 2 {
            return Err(StateError::InvalidAction("At least 2 players required".to_string()));
        }

        let sb_idx = if n == 2 { button_idx } else { (button_idx + 1) % n };
        let bb_idx = if n == 2 { (button_idx + 1) % n } else { (button_idx + 2) % n };
        let utg_idx = (bb_idx + 1) % n;

        let mut sm = HandStateMachine {
            hand_id,
            players,
            button_idx,
            sb_idx,
            bb_idx,
            street: Street::Preflop,
            board: Vec::with_capacity(5),
            dead_cards: DeckBitmask::empty(),
            current_bet: 0.0,
            min_raise: bb_amount,
            action_idx: utg_idx,
            sb_amount,
            bb_amount,
            event_bus: EventBus::new(),
        };

        sm.start_hand();
        Ok(sm)
    }

    /// Sets private hole cards for a player and marks them as dead cards.
    pub fn set_hole_cards(&mut self, player_idx: usize, cards: [Card; 2]) -> Result<(), StateError> {
        if player_idx >= self.players.len() {
            return Err(StateError::PlayerNotFound(player_idx));
        }
        self.players[player_idx].hole_cards = Some(cards);
        self.dead_cards.add_card(cards[0]);
        self.dead_cards.add_card(cards[1]);

        self.event_bus.publish(GameEvent::HoleCardsDealt {
            player_idx,
            cards,
        });
        Ok(())
    }

    fn start_hand(&mut self) {
        self.event_bus.publish(GameEvent::HandStarted {
            hand_id: self.hand_id,
            button_idx: self.button_idx,
            sb_amount: self.sb_amount,
            bb_amount: self.bb_amount,
        });

        // Post SB
        let sb_actual = self.post_blind(self.sb_idx, self.sb_amount);
        // Post BB
        let bb_actual = self.post_blind(self.bb_idx, self.bb_amount);

        self.current_bet = bb_actual.max(sb_actual);
        self.min_raise = self.bb_amount;

        self.event_bus.publish(GameEvent::BlindsPosted {
            sb_player: self.sb_idx,
            sb_amount: sb_actual,
            bb_player: self.bb_idx,
            bb_amount: bb_actual,
        });

        self.emit_hero_to_act_if_applicable();
    }

    fn post_blind(&mut self, player_idx: usize, amount: f64) -> f64 {
        let p = &mut self.players[player_idx];
        let actual = amount.min(p.stack);
        p.stack -= actual;
        p.street_bet += actual;
        p.total_committed += actual;
        if p.stack <= 1e-6 {
            p.is_all_in = true;
        }
        actual
    }

    /// Total chips in the pot (sum of all players' total committed chips).
    #[inline]
    pub fn pot_total(&self) -> f64 {
        self.players.iter().map(|p| p.total_committed).sum()
    }

    /// Count of active players who have not folded.
    #[inline]
    pub fn active_player_count(&self) -> usize {
        self.players.iter().filter(|p| p.is_active).count()
    }

    /// Applies a player's action, validates turn rules, and handles round/street completion.
    pub fn apply_action(&mut self, player_idx: usize, action: PlayerAction) -> Result<(), StateError> {
        if self.street == Street::HandEnded || self.street == Street::Showdown {
            return Err(StateError::HandAlreadyFinished);
        }
        if player_idx != self.action_idx {
            return Err(StateError::NotPlayerTurn {
                expected: self.action_idx,
                actual: player_idx,
            });
        }

        let p = &self.players[player_idx];
        if !p.is_active || p.is_all_in {
            return Err(StateError::InvalidAction("Player is not eligible to act".to_string()));
        }

        let call_cost = (self.current_bet - p.street_bet).max(0.0);

        match action {
            PlayerAction::Fold => {
                let p_mut = &mut self.players[player_idx];
                p_mut.is_active = false;
                p_mut.has_acted = true;
                if let Some(cards) = p_mut.hole_cards {
                    self.dead_cards.add_card(cards[0]);
                    self.dead_cards.add_card(cards[1]);
                }

                self.event_bus.publish(GameEvent::PlayerActed {
                    player_idx,
                    action: PlayerAction::Fold,
                    amount_committed: 0.0,
                    remaining_stack: p_mut.stack,
                });
            }
            PlayerAction::Check => {
                if call_cost > 1e-6 {
                    return Err(StateError::InvalidAction(format!(
                        "Cannot check when facing a bet of {call_cost}"
                    )));
                }
                let p_mut = &mut self.players[player_idx];
                p_mut.has_acted = true;

                self.event_bus.publish(GameEvent::PlayerActed {
                    player_idx,
                    action: PlayerAction::Check,
                    amount_committed: 0.0,
                    remaining_stack: p_mut.stack,
                });
            }
            PlayerAction::Call(amount) => {
                let p_mut = &mut self.players[player_idx];
                let to_call = call_cost.min(p_mut.stack);
                p_mut.stack -= to_call;
                p_mut.street_bet += to_call;
                p_mut.total_committed += to_call;
                if p_mut.stack <= 1e-6 {
                    p_mut.is_all_in = true;
                }
                p_mut.has_acted = true;

                self.event_bus.publish(GameEvent::PlayerActed {
                    player_idx,
                    action: PlayerAction::Call(amount),
                    amount_committed: to_call,
                    remaining_stack: p_mut.stack,
                });
            }
            PlayerAction::Bet(amount) | PlayerAction::Raise(amount) => {
                if amount < call_cost {
                    return Err(StateError::InvalidAction(format!(
                        "Bet/Raise amount {amount} must at least match current bet {call_cost}"
                    )));
                }

                let (total_put_in, remaining_stack, raise_diff, new_street_bet) = {
                    let p = &mut self.players[player_idx];
                    let total_put_in = amount.min(p.stack);
                    p.stack -= total_put_in;
                    p.street_bet += total_put_in;
                    p.total_committed += total_put_in;
                    if p.stack <= 1e-6 {
                        p.is_all_in = true;
                    }
                    p.has_acted = true;
                    let raise_diff = p.street_bet - self.current_bet;
                    (total_put_in, p.stack, raise_diff, p.street_bet)
                };

                if raise_diff > 1e-6 {
                    self.min_raise = raise_diff;
                    self.current_bet = new_street_bet;
                    // Reset has_acted for other active non-allin players
                    for (i, other) in self.players.iter_mut().enumerate() {
                        if i != player_idx && other.is_active && !other.is_all_in {
                            other.has_acted = false;
                        }
                    }
                }

                self.event_bus.publish(GameEvent::PlayerActed {
                    player_idx,
                    action: if matches!(action, PlayerAction::Bet(_)) {
                        PlayerAction::Bet(total_put_in)
                    } else {
                        PlayerAction::Raise(total_put_in)
                    },
                    amount_committed: total_put_in,
                    remaining_stack,
                });
            }
            PlayerAction::AllIn(_) => {
                let (all_in_chips, is_raise, diff) = {
                    let p = &mut self.players[player_idx];
                    let all_in_chips = p.stack;
                    p.stack = 0.0;
                    p.street_bet += all_in_chips;
                    p.total_committed += all_in_chips;
                    p.is_all_in = true;
                    p.has_acted = true;
                    let is_raise = p.street_bet > self.current_bet;
                    let diff = p.street_bet - self.current_bet;
                    (all_in_chips, is_raise, diff)
                };

                if is_raise {
                    if diff >= self.min_raise {
                        self.min_raise = diff;
                    }
                    self.current_bet = self.players[player_idx].street_bet;
                    for (i, other) in self.players.iter_mut().enumerate() {
                        if i != player_idx && other.is_active && !other.is_all_in {
                            other.has_acted = false;
                        }
                    }
                }

                self.event_bus.publish(GameEvent::PlayerActed {
                    player_idx,
                    action: PlayerAction::AllIn(all_in_chips),
                    amount_committed: all_in_chips,
                    remaining_stack: 0.0,
                });
            }
        }

        self.advance_after_action()
    }

    fn advance_after_action(&mut self) -> Result<(), StateError> {
        // Check if only 1 player remains active (all others folded)
        if self.active_player_count() <= 1 {
            self.conclude_hand_uncontested();
            return Ok(());
        }

        // Check if betting round is complete:
        // All active non-all-in players have acted and have street_bet == current_bet
        let round_complete = self.players.iter().all(|p| {
            !p.is_active
                || p.is_all_in
                || (p.has_acted && (p.street_bet - self.current_bet).abs() < 1e-6)
        });

        if round_complete {
            self.advance_street();
        } else {
            self.advance_actor();
            self.emit_hero_to_act_if_applicable();
        }

        Ok(())
    }

    fn advance_actor(&mut self) {
        let n = self.players.len();
        let mut next = (self.action_idx + 1) % n;
        while !self.players[next].is_active || self.players[next].is_all_in {
            next = (next + 1) % n;
            if next == self.action_idx {
                break;
            }
        }
        self.action_idx = next;
    }

    /// Advances the hand to the next street (Flop, Turn, River, or Showdown).
    pub fn advance_street(&mut self) {
        // Reset street bets
        for p in self.players.iter_mut() {
            p.street_bet = 0.0;
            p.has_acted = false;
        }
        self.current_bet = 0.0;
        self.min_raise = self.bb_amount;

        self.street = self.street.next();

        if self.street == Street::Showdown || self.street == Street::HandEnded {
            self.resolve_showdown();
            return;
        }

        self.event_bus.publish(GameEvent::StreetAdvanced {
            street: self.street,
            board: self.board.clone(),
            pot_size: self.pot_total(),
        });

        // Next actor is first active player left of the button
        let n = self.players.len();
        let mut next = (self.button_idx + 1) % n;
        while !self.players[next].is_active || self.players[next].is_all_in {
            next = (next + 1) % n;
            if next == self.button_idx {
                break;
            }
        }
        self.action_idx = next;

        // If at most 1 active player can act (others all-in), advance streets automatically
        let players_can_act = self.players.iter().filter(|p| p.is_active && !p.is_all_in).count();
        if players_can_act <= 1 {
            self.advance_street();
        } else {
            self.emit_hero_to_act_if_applicable();
        }
    }

    /// Adds community cards to the board (e.g. Flop 3 cards, Turn 1 card, River 1 card).
    pub fn deal_board_cards(&mut self, cards: &[Card]) {
        for &c in cards {
            self.board.push(c);
            self.dead_cards.add_card(c);
        }
    }

    fn conclude_hand_uncontested(&mut self) {
        let winner_idx = self.players.iter().position(|p| p.is_active).unwrap_or(0);
        let total_pot = self.pot_total();
        self.players[winner_idx].stack += total_pot;

        let pot = crate::orchestrator::pot::Pot {
            amount: total_pot,
            eligible_players: vec![winner_idx],
        };

        self.event_bus.publish(GameEvent::PotsAwarded {
            pots: vec![pot],
            payouts: vec![(winner_idx, total_pot)],
        });

        self.street = Street::HandEnded;
        self.event_bus.publish(GameEvent::HandComplete {
            final_stacks: self.players.iter().map(|p| p.stack).collect(),
        });
    }

    fn resolve_showdown(&mut self) {
        let contribs: Vec<f64> = self.players.iter().map(|p| p.total_committed).collect();
        let is_active: Vec<bool> = self.players.iter().map(|p| p.is_active).collect();
        let side_pot_res = SidePotCalculator::calculate_pots(&contribs, &is_active);

        // Return uncalled chips first
        for &(uncalled_player, return_amount) in &side_pot_res.uncalled_chips {
            self.players[uncalled_player].stack += return_amount;
        }

        let mut all_pots = Vec::new();
        if side_pot_res.main_pot.amount > 0.0 {
            all_pots.push(side_pot_res.main_pot);
        }
        all_pots.extend(side_pot_res.side_pots);

        let mut payouts = Vec::new();

        // Evaluate each pot
        for pot in &all_pots {
            let mut best_score = 9999u16;
            let mut pot_winners = Vec::new();

            for &p_idx in &pot.eligible_players {
                if let Some(hole) = self.players[p_idx].hole_cards {
                    let seven = [
                        hole[0], hole[1],
                        self.board[0], self.board[1], self.board[2], self.board[3], self.board[4],
                    ];
                    let score = eval_7hand(&seven);
                    if score < best_score {
                        best_score = score;
                        pot_winners.clear();
                        pot_winners.push(p_idx);
                    } else if score == best_score {
                        pot_winners.push(p_idx);
                    }
                }
            }

            let split_share = pot.amount / (pot_winners.len() as f64);
            for &w in &pot_winners {
                self.players[w].stack += split_share;
                payouts.push((w, split_share));
            }
        }

        self.event_bus.publish(GameEvent::PotsAwarded {
            pots: all_pots,
            payouts,
        });

        self.street = Street::HandEnded;
        self.event_bus.publish(GameEvent::HandComplete {
            final_stacks: self.players.iter().map(|p| p.stack).collect(),
        });
    }

    fn emit_hero_to_act_if_applicable(&mut self) {
        if self.street != Street::HandEnded && self.street != Street::Showdown {
            let p = &self.players[self.action_idx];
            if p.is_active && !p.is_all_in {
                let call_cost = (self.current_bet - p.street_bet).max(0.0);
                let current_pot = self.pot_total();
                let min_raise = self.min_raise;

                self.event_bus.publish(GameEvent::HeroToAct {
                    player_idx: self.action_idx,
                    call_cost,
                    current_pot,
                    min_raise,
                });
            }
        }
    }
}
