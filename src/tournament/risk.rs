use crate::tournament::icm::{IcmCalculator, IcmError, MAX_ICM_PLAYERS};

/// Detailed breakdown of an all-in confrontation under ICM.
#[derive(Debug, Clone, PartialEq)]
pub struct ConfrontationRisk {
    pub hero_idx: usize,
    pub villain_idx: usize,
    /// Monetary equity ($EV) before the confrontation.
    pub current_equity: f64,
    /// Monetary equity ($EV) if Hero wins the confrontation.
    pub win_equity: f64,
    /// Monetary equity ($EV) if Hero loses the confrontation.
    pub loss_equity: f64,
    /// Monetary gain if Hero wins: win_equity - current_equity.
    pub delta_win: f64,
    /// Monetary loss if Hero loses: current_equity - loss_equity.
    pub delta_loss: f64,
    /// Bubble Factor = delta_loss / delta_win (in chip-neutral confrontations, > 1.0 indicates ICM pressure).
    pub bubble_factor: f64,
    /// Pure chip EV pot odds required to call: call_cost / total_pot.
    pub pot_odds_chip_ev: f64,
    /// Tournament break-even required equity: delta_loss / (delta_win + delta_loss).
    pub required_tournament_equity: f64,
    /// Risk Premium: required_tournament_equity - pot_odds_chip_ev.
    pub risk_premium: f64,
}

/// Pairwise risk premium matrix for a tournament table.
#[derive(Debug, Clone, PartialEq)]
pub struct RiskPremiumMatrix {
    pub num_players: usize,
    /// Matrix of risk premiums where matrix[hero][villain] is Hero's risk premium against Villain.
    pub matrix: [[f64; MAX_ICM_PLAYERS]; MAX_ICM_PLAYERS],
}

impl RiskPremiumMatrix {
    /// Returns Hero's risk premium against Villain.
    #[inline]
    pub fn get(&self, hero: usize, villain: usize) -> f64 {
        self.matrix[hero][villain]
    }
}

/// Tournament risk premium and range tightening calculator.
pub struct RiskPremiumCalculator;

impl RiskPremiumCalculator {
    /// Evaluates an all-in confrontation between Hero and Villain.
    ///
    /// # Arguments
    /// * `stacks` - Current chip stacks of all players at the table.
    /// * `payouts` - Tournament payout structure.
    /// * `hero_idx` - Table seat index of Hero.
    /// * `villain_idx` - Table seat index of Villain.
    /// * `pot_before_call` - Chips already in the pot before Hero's call (e.g. blinds + villain shove).
    /// * `call_cost` - Chips Hero must put in to call (effective call amount).
    pub fn calculate_allin_confrontation(
        stacks: &[f64],
        payouts: &[f64],
        hero_idx: usize,
        villain_idx: usize,
        pot_before_call: f64,
        call_cost: f64,
    ) -> Result<ConfrontationRisk, IcmError> {
        let n = stacks.len();
        if hero_idx >= n || villain_idx >= n || hero_idx == villain_idx {
            return Err(IcmError::InvalidStack(hero_idx, stacks.get(hero_idx).copied().unwrap_or(0.0)));
        }

        // Current baseline equities
        let current_equities = IcmCalculator::compute_equity(stacks, payouts)?;
        let current_equity = current_equities[hero_idx];

        // Chips at stake
        let total_pot = pot_before_call + call_cost;
        let pot_odds_chip_ev = if total_pot > 0.0 {
            call_cost / total_pot
        } else {
            0.0
        };

        // Stack distribution if Hero WINS
        let mut stacks_win = stacks.to_vec();
        stacks_win[hero_idx] += pot_before_call;
        stacks_win[villain_idx] = (stacks_win[villain_idx] - call_cost).max(0.0);
        let win_equities = IcmCalculator::compute_equity(&stacks_win, payouts)?;
        let win_equity = win_equities[hero_idx];

        // Stack distribution if Hero LOSES
        let mut stacks_loss = stacks.to_vec();
        stacks_loss[hero_idx] = (stacks_loss[hero_idx] - call_cost).max(0.0);
        stacks_loss[villain_idx] += pot_before_call + call_cost;
        let loss_equities = IcmCalculator::compute_equity(&stacks_loss, payouts)?;
        let loss_equity = loss_equities[hero_idx];

        let delta_win = (win_equity - current_equity).max(0.0);
        let delta_loss = (current_equity - loss_equity).max(0.0);

        let bubble_factor = if delta_win > 1e-12 {
            delta_loss / delta_win
        } else {
            1.0
        };

        let denom = delta_win + delta_loss;
        let required_tournament_equity = if denom > 1e-12 {
            delta_loss / denom
        } else {
            pot_odds_chip_ev
        };

        let risk_premium = required_tournament_equity - pot_odds_chip_ev;

        Ok(ConfrontationRisk {
            hero_idx,
            villain_idx,
            current_equity,
            win_equity,
            loss_equity,
            delta_win,
            delta_loss,
            bubble_factor,
            pot_odds_chip_ev,
            required_tournament_equity,
            risk_premium,
        })
    }

    /// Computes the complete pairwise Risk Premium matrix for the table.
    ///
    /// Models an effective all-in confrontation between each pair of active players
    /// for the effective stack size $\min(s_i, s_j)$ with equal 1:1 pot odds (50% ChipEV).
    pub fn compute_pairwise_risk_matrix(
        stacks: &[f64],
        payouts: &[f64],
    ) -> Result<RiskPremiumMatrix, IcmError> {
        let n = stacks.len();
        if n == 0 {
            return Err(IcmError::EmptyStacks);
        }
        if n > MAX_ICM_PLAYERS {
            return Err(IcmError::TooManyPlayers(n));
        }

        let mut matrix = [[0.0f64; MAX_ICM_PLAYERS]; MAX_ICM_PLAYERS];

        for hero in 0..n {
            for villain in 0..n {
                if hero == villain || stacks[hero] <= 0.0 || stacks[villain] <= 0.0 {
                    matrix[hero][villain] = 0.0;
                    continue;
                }

                let eff_stack = stacks[hero].min(stacks[villain]);
                let confrontation = Self::calculate_allin_confrontation(
                    stacks,
                    payouts,
                    hero,
                    villain,
                    eff_stack, // pot before call = eff_stack
                    eff_stack, // call cost = eff_stack (1:1 pot odds => 50% ChipEV)
                )?;

                matrix[hero][villain] = confrontation.risk_premium;
            }
        }

        Ok(RiskPremiumMatrix {
            num_players: n,
            matrix,
        })
    }

    /// Adjusts a raw ChipEV equity threshold by the calculated Risk Premium.
    ///
    /// For example, if a baseline calling hand requires 35% equity in ChipEV,
    /// and the Risk Premium is +8.5%, the ICM-adjusted requirement is 43.5%.
    #[inline]
    pub fn adjust_equity_threshold(base_chip_ev_threshold: f64, risk_premium: f64) -> f64 {
        (base_chip_ev_threshold + risk_premium).clamp(0.0, 1.0)
    }
}
