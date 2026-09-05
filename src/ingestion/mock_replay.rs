use crate::card::Card;
use crate::ingestion::schema::ReplayHandScenario;
use crate::orchestrator::{GameEvent, HandStateMachine, Player, StateError, Street};
use crate::sim::ev::{DecisionReport, EvContext, EvEngine};

/// Result of playing through a mock tournament hand scenario.
#[derive(Debug, Clone)]
pub struct ReplayResult {
    pub hand_id: u64,
    pub events: Vec<GameEvent>,
    pub decision_reports: Vec<DecisionReport>,
    pub final_stacks: Vec<f64>,
}

/// Adapter that replays pre-recorded hands and drives real-time decision reporting.
pub struct MockReplayAdapter;

impl MockReplayAdapter {
    /// Executes a deterministic replay of a hand scenario step-by-step.
    pub fn replay(scenario: &ReplayHandScenario) -> Result<ReplayResult, StateError> {
        let players: Vec<Player> = scenario
            .players
            .iter()
            .enumerate()
            .map(|(i, p)| Player::new(i, &p.name, p.stack))
            .collect();

        let mut sm = HandStateMachine::new(
            scenario.hand_id,
            players,
            scenario.button_idx,
            scenario.sb_amount,
            scenario.bb_amount,
        )?;

        // Deal hole cards
        for (i, p) in scenario.players.iter().enumerate() {
            if let Some(cards_str) = &p.hole_cards {
                let c1 = Card::from_str_exact(&cards_str[0])
                    .map_err(|e| StateError::InvalidAction(e.to_string()))?;
                let c2 = Card::from_str_exact(&cards_str[1])
                    .map_err(|e| StateError::InvalidAction(e.to_string()))?;
                sm.set_hole_cards(i, [c1, c2])?;
            }
        }

        let mut decision_reports = Vec::new();
        let hero_idx = scenario.hero_idx.unwrap_or(0);

        // Parse community cards
        let board_cards: Result<Vec<Card>, _> = scenario
            .board_cards
            .iter()
            .map(|s| Card::from_str_exact(s))
            .collect();
        let board_cards = board_cards.map_err(|e| StateError::InvalidAction(e.to_string()))?;

        // Replay action script
        let mut board_cursor = 0;

        for step in &scenario.action_script {
            // Check if street advanced and we need to deal board cards
            if sm.street == Street::Flop && sm.board.is_empty() && board_cards.len() >= 3 {
                sm.deal_board_cards(&board_cards[0..3]);
                board_cursor = 3;
            } else if sm.street == Street::Turn && sm.board.len() == 3 && board_cards.len() >= 4 {
                sm.deal_board_cards(&board_cards[3..4]);
                board_cursor = 4;
            } else if sm.street == Street::River && sm.board.len() == 4 && board_cards.len() >= 5 {
                sm.deal_board_cards(&board_cards[4..5]);
                board_cursor = 5;
            }

            // If it's Hero's turn to act, generate an EV DecisionReport before applying the action
            if sm.action_idx == hero_idx && sm.street != Street::HandEnded && sm.street != Street::Showdown {
                if let Some(hero_cards) = sm.players[hero_idx].hole_cards {
                    let call_cost = (sm.current_bet - sm.players[hero_idx].street_bet).max(0.0);
                    let current_pot = sm.pot_total();
                    let hero_stack = sm.players[hero_idx].stack;

                    let ctx = EvContext {
                        street: format!("{:?}", sm.street),
                        current_pot,
                        call_cost,
                        hero_stack,
                        hero_cards,
                        board: sm.board.clone(),
                        hero_equity: 0.60, // Baseline equity estimate for reporting
                        hero_equity_when_called: 0.50,
                        opp_fold_freq: 0.35,
                        risk_premium: 0.05,
                    };

                    let mut report = EvEngine::evaluate_decision(&ctx);
                    report.hand_id = sm.hand_id;
                    decision_reports.push(report);
                }
            }

            // Apply recorded action
            sm.apply_action(step.player_idx, step.action.clone())?;
        }

        // Deal remaining board cards if showdown reached with cards left in scenario
        if sm.board.len() < 5 && board_cards.len() == 5 && sm.street == Street::Showdown {
            sm.deal_board_cards(&board_cards[board_cursor..5]);
        }

        let events = sm.event_bus.events().to_vec();
        let final_stacks = sm.players.iter().map(|p| p.stack).collect();

        Ok(ReplayResult {
            hand_id: scenario.hand_id,
            events,
            decision_reports,
            final_stacks,
        })
    }
}
