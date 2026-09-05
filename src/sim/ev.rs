use crate::card::Card;
use serde::{Deserialize, Serialize};

/// Candidate actions available to a player at a decision node.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "amount")]
pub enum CandidateAction {
    Fold,
    Check,
    Call(f64),
    Bet(f64),
    Raise(f64),
    AllIn(f64),
}

/// Expected Value breakdown for a candidate action.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActionEv {
    pub action: CandidateAction,
    /// ChipEV: Expected chip change from baseline.
    pub chip_ev: f64,
    /// Tournament $EV (if ICM parameters are provided, otherwise mirrors chip_ev).
    pub dollar_ev: Option<f64>,
    /// Win/Chop equity required for this action to break even.
    pub break_even_equity: f64,
    /// Description of the action.
    pub description: String,
}

/// Structured JSON decision report output by the engine.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DecisionReport {
    /// Associated tournament hand ID.
    #[serde(default)]
    pub hand_id: u64,
    /// Street name (Preflop, Flop, Turn, River).
    pub street: String,
    /// Total pot size before Hero's decision.
    pub current_pot: f64,
    /// Cost in chips facing Hero to continue (0.0 if checking).
    pub call_cost: f64,
    /// Hero's current chip stack.
    #[serde(default)]
    pub hero_stack: f64,
    /// Hero's hole cards as strings.
    pub hero_cards: [String; 2],
    /// Community board cards as strings.
    pub board_cards: Vec<String>,
    /// Hero's estimated equity against opponent ranges [0.0, 1.0].
    pub hero_equity: f64,
    /// Pot odds facing Hero (call_cost / (current_pot + call_cost)).
    pub pot_odds: f64,
    /// Array of candidate action EV evaluations.
    pub action_evaluations: Vec<ActionEv>,
    /// The action with maximum expected value (argmax EV).
    pub recommended_action: CandidateAction,
    /// Analytical justification for the recommendation.
    pub reasoning: String,
}

impl DecisionReport {
    /// Serializes the DecisionReport into a formatted JSON string.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}

/// Configuration context for EV solving.
#[derive(Debug, Clone)]
pub struct EvContext {
    pub street: String,
    pub current_pot: f64,
    pub call_cost: f64,
    pub hero_stack: f64,
    pub hero_cards: [Card; 2],
    pub board: Vec<Card>,
    pub hero_equity: f64,
    /// Estimated equity when opponent calls Hero's bet.
    pub hero_equity_when_called: f64,
    /// Opponent fold probability when facing a bet.
    pub opp_fold_freq: f64,
    /// Optional Risk Premium (from Milestone 2 ICM) to adjust calling/betting thresholds.
    pub risk_premium: f64,
}

/// Solver engine that computes argmax EV across candidate poker actions.
pub struct EvEngine;

impl EvEngine {
    /// Computes EV for all candidate actions and formats a structured DecisionReport.
    pub fn evaluate_decision(ctx: &EvContext) -> DecisionReport {
        let mut evaluations = Vec::new();

        // 1. Fold: EV is strictly 0.0 relative to current decision point
        evaluations.push(ActionEv {
            action: CandidateAction::Fold,
            chip_ev: 0.0,
            dollar_ev: Some(0.0),
            break_even_equity: 0.0,
            description: "Surrender hand without committing additional chips".to_string(),
        });

        // 2. Check / Call
        let pot_odds = if ctx.current_pot + ctx.call_cost > 0.0 {
            ctx.call_cost / (ctx.current_pot + ctx.call_cost)
        } else {
            0.0
        };

        if ctx.call_cost <= 1e-6 {
            // Check: EV = Equity * CurrentPot
            let check_ev = ctx.hero_equity * ctx.current_pot;
            evaluations.push(ActionEv {
                action: CandidateAction::Check,
                chip_ev: check_ev,
                dollar_ev: Some(check_ev),
                break_even_equity: 0.0,
                description: "Pass action and see next card/showdown for free".to_string(),
            });
        } else {
            // Call: EV = (Equity * FinalPot) - CallCost
            let final_pot = ctx.current_pot + ctx.call_cost;
            let call_ev = (ctx.hero_equity * final_pot) - ctx.call_cost;
            let break_even = pot_odds + ctx.risk_premium;

            evaluations.push(ActionEv {
                action: CandidateAction::Call(ctx.call_cost),
                chip_ev: call_ev,
                dollar_ev: Some(call_ev),
                break_even_equity: break_even,
                description: format!("Match current bet of {:.1} chips (Pot odds: {:.1}%)", ctx.call_cost, pot_odds * 100.0),
            });
        }

        // 3. Raise sizing if facing a bet
        if ctx.call_cost > 1e-6 && ctx.hero_stack > ctx.call_cost {
            let raise_amount = (ctx.call_cost * 2.5 + ctx.current_pot * 0.25).min(ctx.hero_stack);
            if raise_amount > ctx.call_cost && raise_amount < ctx.hero_stack {
                let p_fold = ctx.opp_fold_freq;
                let p_call = 1.0 - p_fold;
                let pot_when_called = ctx.current_pot + ctx.call_cost + (2.0 * raise_amount);
                let ev_called = (ctx.hero_equity_when_called * pot_when_called) - raise_amount;
                let raise_ev = (p_fold * (ctx.current_pot + ctx.call_cost)) + (p_call * ev_called);

                evaluations.push(ActionEv {
                    action: CandidateAction::Raise(raise_amount),
                    chip_ev: raise_ev,
                    dollar_ev: Some(raise_ev),
                    break_even_equity: raise_amount / (ctx.current_pot + ctx.call_cost + raise_amount),
                    description: format!("Standard raise to {:.1} chips", raise_amount),
                });
            }
        }

        // 4. Sizing Bets: 33% Pot and 75% Pot (if Hero has enough chips and not facing a large bet)
        if ctx.call_cost <= 1e-6 && ctx.hero_stack > 0.0 {
            let sizes = [
                (0.33f64, "Bet 33% Pot (Small continuation / block)"),
                (0.75f64, "Bet 75% Pot (Standard value / protection)"),
            ];

            for (fraction, desc) in sizes {
                let bet_amount = (ctx.current_pot * fraction).min(ctx.hero_stack);
                if bet_amount > 0.0 {
                    // EV(Bet) = P(Fold) * Pot + P(Call) * [EquityCalled * (Pot + 2*Bet) - Bet]
                    let p_fold = ctx.opp_fold_freq;
                    let p_call = 1.0 - p_fold;
                    let pot_when_called = ctx.current_pot + (2.0 * bet_amount);
                    let ev_when_called = (ctx.hero_equity_when_called * pot_when_called) - bet_amount;
                    let bet_ev = (p_fold * ctx.current_pot) + (p_call * ev_when_called);

                    evaluations.push(ActionEv {
                        action: CandidateAction::Bet(bet_amount),
                        chip_ev: bet_ev,
                        dollar_ev: Some(bet_ev),
                        break_even_equity: bet_amount / (ctx.current_pot + bet_amount),
                        description: format!("{desc}: {:.1} chips", bet_amount),
                    });
                }
            }
        }

        // 5. All-In with realistic overbet diminishing returns
        if ctx.hero_stack > 0.0 {
            let shove_amount = ctx.hero_stack;
            let total_facing = ctx.current_pot + ctx.call_cost;
            let overbet_ratio = if total_facing > 0.0 {
                shove_amount / total_facing
            } else {
                1.0
            };

            let (p_fold, called_equity) = if overbet_ratio > 1.5 {
                let pf = (1.0 - (1.0 - ctx.opp_fold_freq) / (1.0 + 0.3 * (overbet_ratio - 1.0)))
                    .clamp(ctx.opp_fold_freq, 0.98);
                let eq = (ctx.hero_equity_when_called
                    * (1.0 / (1.0 + 0.15 * (overbet_ratio - 1.0))))
                    .clamp(0.10, 0.95);
                (pf, eq)
            } else {
                (
                    (ctx.opp_fold_freq * 1.2).clamp(0.0, 0.95),
                    ctx.hero_equity_when_called,
                )
            };

            let p_call = 1.0 - p_fold;
            let final_pot = ctx.current_pot + ctx.call_cost + (2.0 * shove_amount);
            let ev_called = (called_equity * final_pot) - shove_amount;
            let allin_ev = (p_fold * ctx.current_pot) + (p_call * ev_called);

            evaluations.push(ActionEv {
                action: CandidateAction::AllIn(shove_amount),
                chip_ev: allin_ev,
                dollar_ev: Some(allin_ev),
                break_even_equity: shove_amount / (ctx.current_pot + shove_amount),
                description: format!("Commit entire remaining stack of {:.1} chips", shove_amount),
            });
        }

        // Select recommended action: argmax EV
        let best = evaluations
            .iter()
            .max_by(|a, b| a.chip_ev.partial_cmp(&b.chip_ev).unwrap())
            .unwrap();

        let recommended_action = best.action.clone();
        let reasoning = format!(
            "Action {:?} yields optimal expected value of +{:.2} chips (Hero Equity: {:.1}%, Required: {:.1}%)",
            recommended_action,
            best.chip_ev,
            ctx.hero_equity * 100.0,
            best.break_even_equity * 100.0,
        );

        DecisionReport {
            hand_id: 0,
            street: ctx.street.clone(),
            current_pot: ctx.current_pot,
            call_cost: ctx.call_cost,
            hero_stack: ctx.hero_stack,
            hero_cards: [ctx.hero_cards[0].to_string(), ctx.hero_cards[1].to_string()],
            board_cards: ctx.board.iter().map(|c| c.to_string()).collect(),
            hero_equity: ctx.hero_equity,
            pot_odds,
            action_evaluations: evaluations,
            recommended_action,
            reasoning,
        }
    }
}
