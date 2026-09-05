use crate::orchestrator::{PlayerAction, Street};
use crate::sim::ev::CandidateAction;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Classification of a decision leak severity based on Big Blind EV lost.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum LeakSeverity {
    /// Decision within 0.25 BB of optimal.
    Optimal,
    /// Minor imperfection losing between 0.25 and 1.0 BB.
    Inaccuracy,
    /// Noticeable mistake losing between 1.0 and 3.0 BB.
    Mistake,
    /// Severe leak / blunder losing 3.0+ BB.
    Blunder,
}

impl LeakSeverity {
    pub fn from_bb_loss(bb_loss: f64) -> Self {
        if bb_loss >= 3.0 {
            LeakSeverity::Blunder
        } else if bb_loss >= 1.0 {
            LeakSeverity::Mistake
        } else if bb_loss >= 0.25 {
            LeakSeverity::Inaccuracy
        } else {
            LeakSeverity::Optimal
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            LeakSeverity::Optimal => "OPTIMAL",
            LeakSeverity::Inaccuracy => "INACCURACY",
            LeakSeverity::Mistake => "MISTAKE",
            LeakSeverity::Blunder => "BLUNDER",
        }
    }
}

/// Audit record for an individual decision node faced by Hero.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DecisionAudit {
    pub hand_id: u64,
    pub street: Street,
    pub hero_cards: [String; 2],
    pub board_cards: Vec<String>,
    pub pot_before: f64,
    pub call_cost: f64,
    pub hero_stack: f64,
    pub action_taken: PlayerAction,
    pub recommended_action: CandidateAction,
    pub actual_chip_ev: f64,
    pub optimal_chip_ev: f64,
    pub chip_ev_loss: f64,
    pub ev_loss_bb: f64,
    pub dollar_ev_loss: Option<f64>,
    pub severity: LeakSeverity,
    pub reasoning: String,
}

/// Aggregated statistics broken down by betting street.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct StreetAuditStats {
    pub street: String,
    pub decisions_count: usize,
    pub chip_ev_loss: f64,
    pub bb_ev_loss: f64,
    pub blunder_count: usize,
    pub mistake_count: usize,
    pub inaccuracy_count: usize,
}

/// Comprehensive Post-Game Reviewer report for a tournament session.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionAuditReport {
    pub total_hands: usize,
    pub hero_hands: usize,
    pub total_decisions: usize,
    pub total_chip_ev_loss: f64,
    pub total_bb_ev_loss: f64,
    pub blunder_count: usize,
    pub mistake_count: usize,
    pub inaccuracy_count: usize,
    pub optimal_count: usize,
    pub leak_free_percentage: f64,
    pub street_stats: HashMap<String, StreetAuditStats>,
    pub top_leaks: Vec<DecisionAudit>,
    pub all_audits: Vec<DecisionAudit>,
}

impl SessionAuditReport {
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    pub fn from_json(s: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(s)
    }

    /// Formats an elegant, readable terminal summary table for the session review.
    pub fn format_terminal_table(&self) -> String {
        let mut out = String::new();
        out.push_str("================================================================================\n");
        out.push_str("                     TOURNAMENT SESSION AUDIT REPORT                            \n");
        out.push_str("================================================================================\n");
        out.push_str(&format!(
            "Total Hands Analyzed:   {:>6} | Hero Participated: {:>6}\n",
            self.total_hands, self.hero_hands
        ));
        out.push_str(&format!(
            "Total Decision Nodes:   {:>6} | Leak-Free Rate:    {:>6.1}%\n",
            self.total_decisions, self.leak_free_percentage
        ));
        out.push_str(&format!(
            "Cumulative EV Loss:     {:>9.2} chips ({:>6.2} BB)\n",
            self.total_chip_ev_loss, self.total_bb_ev_loss
        ));
        out.push_str("--------------------------------------------------------------------------------\n");
        out.push_str("LEAK CLASSIFICATION BREAKDOWN:\n");
        out.push_str(&format!(
            "  * [BLUNDER]    (>= 3.0 BB): {:>4} decisions\n",
            self.blunder_count
        ));
        out.push_str(&format!(
            "  * [MISTAKE]    (1.0-3.0 BB):{:>4} decisions\n",
            self.mistake_count
        ));
        out.push_str(&format!(
            "  * [INACCURACY] (0.25-1.0 BB):{:>3} decisions\n",
            self.inaccuracy_count
        ));
        out.push_str(&format!(
            "  * [OPTIMAL]    (< 0.25 BB): {:>4} decisions\n",
            self.optimal_count
        ));
        out.push_str("--------------------------------------------------------------------------------\n");
        out.push_str("STREET-BY-STREET ANALYSIS:\n");
        out.push_str(&format!(
            "  {:<10} | {:<10} | {:<14} | {:<12} | {:<8}\n",
            "Street", "Decisions", "Chip EV Loss", "BB EV Loss", "Blunders"
        ));
        out.push_str("  --------------------------------------------------------------------\n");

        for street_name in &["Preflop", "Flop", "Turn", "River"] {
            if let Some(stats) = self.street_stats.get(*street_name) {
                out.push_str(&format!(
                    "  {:<10} | {:>10} | {:>14.2} | {:>12.2} | {:>8}\n",
                    stats.street,
                    stats.decisions_count,
                    stats.chip_ev_loss,
                    stats.bb_ev_loss,
                    stats.blunder_count
                ));
            }
        }

        if !self.top_leaks.is_empty() {
            out.push_str("--------------------------------------------------------------------------------\n");
            out.push_str("TOP DETECTED LEAKS / BLUNDERS:\n");
            for (idx, leak) in self.top_leaks.iter().take(5).enumerate() {
                let board_str = if leak.board_cards.is_empty() {
                    "-".to_string()
                } else {
                    leak.board_cards.join(" ")
                };
                out.push_str(&format!(
                    "  #{}. Hand #{:<10} [{}] Street: {:?} | Hole: [{} {}] | Board: [{}]\n",
                    idx + 1,
                    leak.hand_id,
                    leak.severity.label(),
                    leak.street,
                    leak.hero_cards[0],
                    leak.hero_cards[1],
                    board_str
                ));
                out.push_str(&format!(
                    "      Action Taken: {:?} (EV: {:+.2})\n",
                    leak.action_taken, leak.actual_chip_ev
                ));
                out.push_str(&format!(
                    "      Recommended:  {:?} (EV: {:+.2}) | EV Loss: -{:.2} chips (-{:.2} BB)\n",
                    leak.recommended_action, leak.optimal_chip_ev, leak.chip_ev_loss, leak.ev_loss_bb
                ));
                out.push_str(&format!("      Analytical Note: {}\n\n", leak.reasoning));
            }
        }
        out.push_str("================================================================================\n");
        out
    }
}
