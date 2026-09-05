use crate::api_bridge::hand_parser::HandHistoryParser;
use crate::api_bridge::watcher::split_hand_blocks;
use crate::audit::report::{DecisionAudit, LeakSeverity, SessionAuditReport, StreetAuditStats};
use crate::deck::DeckBitmask;
use crate::orchestrator::{HandStateMachine, Player, PlayerAction, Street};
use crate::range::Range;
use crate::sim::ev::{EvContext, EvEngine};
use crate::sim::MonteCarloSimulator;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// End-to-End Session Auditor / Post-Game Reviewer.
///
/// Ingests multi-hand tournament logs, plays through each hand using the state machine,
/// runs game-theoretic EV solvers at each Hero decision node, and calculates cumulative EV loss / leaks.
pub struct SessionAuditor;

impl SessionAuditor {
    /// Reads and audits a tournament hand history file from disk.
    pub fn audit_file<P: AsRef<Path>>(
        path: P,
        default_payouts: Option<Vec<f64>>,
    ) -> Result<SessionAuditReport, String> {
        let text = fs::read_to_string(path.as_ref())
            .map_err(|e| format!("Failed to read hand history file {:?}: {}", path.as_ref(), e))?;
        Self::audit_text(&text, default_payouts)
    }

    /// Audits a tournament session from raw text containing one or multiple hand history blocks.
    pub fn audit_text(
        text: &str,
        _default_payouts: Option<Vec<f64>>,
    ) -> Result<SessionAuditReport, String> {
        let hand_blocks = split_hand_blocks(text);
        if hand_blocks.is_empty() {
            return Err("No valid hand history blocks found in input".to_string());
        }

        let mut total_hands = 0;
        let mut hero_hands = 0;
        let mut total_decisions = 0;
        let mut total_chip_ev_loss = 0.0;
        let mut total_bb_ev_loss = 0.0;
        let mut blunder_count = 0;
        let mut mistake_count = 0;
        let mut inaccuracy_count = 0;
        let mut optimal_count = 0;

        let mut street_stats: HashMap<String, StreetAuditStats> = HashMap::new();
        for s in &["Preflop", "Flop", "Turn", "River"] {
            street_stats.insert(
                s.to_string(),
                StreetAuditStats {
                    street: s.to_string(),
                    decisions_count: 0,
                    chip_ev_loss: 0.0,
                    bb_ev_loss: 0.0,
                    blunder_count: 0,
                    mistake_count: 0,
                    inaccuracy_count: 0,
                },
            );
        }

        let mut all_audits: Vec<DecisionAudit> = Vec::new();

        // Reusable opponent range for equity simulation
        let default_villain_range = Range::uniform();

        for block in &hand_blocks {
            let parsed = match HandHistoryParser::parse(block) {
                Ok(p) => p,
                Err(_) => continue,
            };

            total_hands += 1;
            let hero_name = match &parsed.hero_name {
                Some(name) => name.clone(),
                None => continue,
            };

            let hero_cards = match parsed.hero_cards {
                Some(cards) => cards,
                None => continue,
            };

            hero_hands += 1;

            let players: Vec<Player> = parsed
                .players
                .iter()
                .enumerate()
                .map(|(i, p)| Player::new(i, &p.name, p.stack))
                .collect();

            let hero_idx = match players.iter().position(|p| p.name == hero_name) {
                Some(idx) => idx,
                None => continue,
            };

            let button_idx = parsed.button_seat.saturating_sub(1);
            let mut sm = match HandStateMachine::new(
                parsed.hand_id,
                players,
                button_idx,
                parsed.sb_amount,
                parsed.bb_amount,
            ) {
                Ok(m) => m,
                Err(_) => continue,
            };

            let _ = sm.set_hole_cards(hero_idx, hero_cards);

            let board_cards = parsed.board_cards.clone();

            for step in &parsed.actions {
                let step_street = match step.street.as_str() {
                    "Flop" => Street::Flop,
                    "Turn" => Street::Turn,
                    "River" => Street::River,
                    _ => Street::Preflop,
                };

                // Ensure street is synchronized with the recorded action step
                if step_street != sm.street {
                    sm.street = step_street;
                    sm.current_bet = 0.0;
                    for p in &mut sm.players {
                        p.street_bet = 0.0;
                        p.has_acted = false;
                    }
                }

                // Deal board cards when streets advance
                if (sm.street == Street::Flop || sm.street == Street::Turn || sm.street == Street::River)
                    && sm.board.is_empty()
                    && board_cards.len() >= 3
                {
                    sm.deal_board_cards(&board_cards[0..3]);
                }
                if (sm.street == Street::Turn || sm.street == Street::River)
                    && sm.board.len() == 3
                    && board_cards.len() >= 4
                {
                    sm.deal_board_cards(&board_cards[3..4]);
                }
                if sm.street == Street::River
                    && sm.board.len() == 4
                    && board_cards.len() >= 5
                {
                    sm.deal_board_cards(&board_cards[4..5]);
                }

                // Check if it is Hero's turn to act
                if step.player_idx == hero_idx
                    && sm.street != Street::HandEnded
                    && sm.street != Street::Showdown
                {
                    sm.action_idx = hero_idx;
                    let current_pot = sm.pot_total().max(parsed.sb_amount + parsed.bb_amount);
                    let call_cost = (sm.current_bet - sm.players[hero_idx].street_bet).max(0.0);
                    let hero_stack = sm.players[hero_idx].stack;

                    // Calculate Hero's equity using Monte Carlo simulator
                    let mut dead = DeckBitmask::empty();
                    dead.add_card(hero_cards[0]);
                    dead.add_card(hero_cards[1]);
                    for &b in &sm.board {
                        dead.add_card(b);
                    }

                    let eq_res = MonteCarloSimulator::simulate_equity(
                        hero_cards,
                        &sm.board,
                        &[default_villain_range.clone()],
                        dead,
                        1_000,
                        Some(0x1234567890ABCDEF),
                    );

                    let hero_equity = eq_res.equity;
                    let opp_fold_freq = match sm.street {
                        Street::Preflop => 0.40,
                        Street::Flop => 0.35,
                        Street::Turn => 0.28,
                        Street::River => 0.20,
                        _ => 0.30,
                    };

                    let hero_equity_when_called = (hero_equity * 0.85).clamp(0.10, 0.95);

                    let ctx = EvContext {
                        street: format!("{:?}", sm.street),
                        current_pot,
                        call_cost,
                        hero_stack,
                        hero_cards,
                        board: sm.board.clone(),
                        hero_equity,
                        hero_equity_when_called,
                        opp_fold_freq,
                        risk_premium: 0.03,
                    };

                    let report = EvEngine::evaluate_decision(&ctx);

                    // Determine EV of the optimal recommendation
                    let optimal_action = report.recommended_action.clone();
                    let optimal_eval = report
                        .action_evaluations
                        .iter()
                        .find(|e| e.action == optimal_action)
                        .unwrap();
                    let optimal_chip_ev = optimal_eval.chip_ev;

                    // Calculate EV of actual action taken by Hero
                    let actual_chip_ev = match &step.action {
                        PlayerAction::Fold => 0.0,
                        PlayerAction::Check => hero_equity * current_pot,
                        PlayerAction::Call(_) => {
                            let final_pot = current_pot + call_cost;
                            (hero_equity * final_pot) - call_cost
                        }
                        PlayerAction::Bet(amt) | PlayerAction::Raise(amt) | PlayerAction::AllIn(amt) => {
                            let bet_amt = (*amt).min(hero_stack);
                            let p_fold = opp_fold_freq;
                            let p_call = 1.0 - p_fold;
                            let pot_called = current_pot + (2.0 * bet_amt);
                            let ev_called = (hero_equity_when_called * pot_called) - bet_amt;
                            (p_fold * current_pot) + (p_call * ev_called)
                        }
                    };

                    let chip_ev_loss = (optimal_chip_ev - actual_chip_ev).max(0.0);
                    let bb = parsed.bb_amount.max(1.0);
                    let ev_loss_bb = chip_ev_loss / bb;
                    let severity = LeakSeverity::from_bb_loss(ev_loss_bb);

                    total_decisions += 1;
                    total_chip_ev_loss += chip_ev_loss;
                    total_bb_ev_loss += ev_loss_bb;

                    match severity {
                        LeakSeverity::Blunder => blunder_count += 1,
                        LeakSeverity::Mistake => mistake_count += 1,
                        LeakSeverity::Inaccuracy => inaccuracy_count += 1,
                        LeakSeverity::Optimal => optimal_count += 1,
                    }

                    // Update street stats
                    let street_key = match sm.street {
                        Street::Preflop => "Preflop",
                        Street::Flop => "Flop",
                        Street::Turn => "Turn",
                        Street::River => "River",
                        _ => "Preflop",
                    };

                    if let Some(st) = street_stats.get_mut(street_key) {
                        st.decisions_count += 1;
                        st.chip_ev_loss += chip_ev_loss;
                        st.bb_ev_loss += ev_loss_bb;
                        match severity {
                            LeakSeverity::Blunder => st.blunder_count += 1,
                            LeakSeverity::Mistake => st.mistake_count += 1,
                            LeakSeverity::Inaccuracy => st.inaccuracy_count += 1,
                            LeakSeverity::Optimal => {}
                        }
                    }

                    let reasoning = format!(
                        "Optimal {:?} (+{:.2} chips) vs actual {:?} ({:+.2} chips). Equity: {:.1}%, Pot Odds: {:.1}%.",
                        optimal_action, optimal_chip_ev, step.action, actual_chip_ev, hero_equity * 100.0, report.pot_odds * 100.0
                    );

                    all_audits.push(DecisionAudit {
                        hand_id: parsed.hand_id,
                        street: sm.street,
                        hero_cards: [hero_cards[0].to_string(), hero_cards[1].to_string()],
                        board_cards: sm.board.iter().map(|c| c.to_string()).collect(),
                        pot_before: current_pot,
                        call_cost,
                        hero_stack,
                        action_taken: step.action.clone(),
                        recommended_action: optimal_action,
                        actual_chip_ev,
                        optimal_chip_ev,
                        chip_ev_loss,
                        ev_loss_bb,
                        dollar_ev_loss: Some(chip_ev_loss),
                        severity,
                        reasoning,
                    });
                }

                // Apply recorded action to state machine
                sm.action_idx = step.player_idx;
                let _ = sm.apply_action(step.player_idx, step.action.clone());
            }
        }

        let leak_free_percentage = if total_decisions > 0 {
            (optimal_count as f64 / total_decisions as f64) * 100.0
        } else {
            100.0
        };

        // Extract top leaks sorted by chip EV loss descending
        let mut top_leaks: Vec<DecisionAudit> = all_audits
            .iter()
            .filter(|a| a.severity != LeakSeverity::Optimal)
            .cloned()
            .collect();
        top_leaks.sort_by(|a, b| b.chip_ev_loss.partial_cmp(&a.chip_ev_loss).unwrap());

        Ok(SessionAuditReport {
            total_hands,
            hero_hands,
            total_decisions,
            total_chip_ev_loss,
            total_bb_ev_loss,
            blunder_count,
            mistake_count,
            inaccuracy_count,
            optimal_count,
            leak_free_percentage,
            street_stats,
            top_leaks,
            all_audits,
        })
    }
}
