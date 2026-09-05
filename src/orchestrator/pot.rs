use serde::{Deserialize, Serialize};
 
/// A pot (either main pot or side pot) and the players eligible to win it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Pot {
    pub amount: f64,
    /// Indices of players who are eligible to contest this pot.
    pub eligible_players: Vec<usize>,
}

/// Result of multiway pot resolution with side pots and uncalled chips.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SidePotResult {
    pub main_pot: Pot,
    pub side_pots: Vec<Pot>,
    /// Any uncalled chips returned directly to players (e.g. uncalled bet/shove).
    pub uncalled_chips: Vec<(usize, f64)>,
    /// Total chips collected across all pots and uncalled returns.
    pub total_collected: f64,
}

/// Accurately computes main pot, side pots, and uncalled chip returns for multiway all-in scenarios.
pub struct SidePotCalculator;

impl SidePotCalculator {
    /// Computes pots from player total contributions and active (non-folded) statuses.
    ///
    /// # Arguments
    /// * `contributions` - Total chips contributed by each player throughout the hand.
    /// * `is_active` - Boolean flag per player indicating if they are still in the hand (not folded).
    pub fn calculate_pots(contributions: &[f64], is_active: &[bool]) -> SidePotResult {
        let n = contributions.len();
        let total_collected: f64 = contributions.iter().sum();

        if total_collected <= 0.0 {
            return SidePotResult {
                main_pot: Pot { amount: 0.0, eligible_players: Vec::new() },
                side_pots: Vec::new(),
                uncalled_chips: Vec::new(),
                total_collected: 0.0,
            };
        }

        // Active players who contributed > 0
        let mut active_contributors: Vec<(usize, f64)> = contributions
            .iter()
            .enumerate()
            .filter(|&(idx, &c)| is_active[idx] && c > 1e-6)
            .map(|(idx, &c)| (idx, c))
            .collect();

        // Edge case: all active players contributed 0 or only 1 player contributed
        if active_contributors.is_empty() {
            // Folded players or uncontested: return remaining to first non-zero contributor
            return SidePotResult {
                main_pot: Pot { amount: total_collected, eligible_players: (0..n).filter(|&i| is_active[i]).collect() },
                side_pots: Vec::new(),
                uncalled_chips: Vec::new(),
                total_collected,
            };
        }

        // Sort active contributors by contribution amount ascending
        active_contributors.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

        let mut pots = Vec::new();
        let mut prev_level = 0.0f64;
        let mut uncalled_chips = Vec::new();

        // Distinct commitment levels among active contributors
        let mut levels: Vec<f64> = active_contributors.iter().map(|&(_, c)| c).collect();
        levels.dedup_by(|a, b| (*a - *b).abs() < 1e-6);

        let mut rem_contribs = contributions.to_vec();

        for &level in &levels {
            let slice_size = level - prev_level;
            if slice_size <= 1e-6 {
                continue;
            }

            let mut pot_amount = 0.0f64;
            let mut eligible = Vec::new();

            for i in 0..n {
                if rem_contribs[i] > 1e-6 {
                    let take = rem_contribs[i].min(slice_size);
                    pot_amount += take;
                    rem_contribs[i] -= take;
                }
                if is_active[i] && contributions[i] >= level - 1e-6 {
                    eligible.push(i);
                }
            }

            if eligible.len() == 1 {
                // Only 1 player contributed at this tier -> uncalled chips returned!
                uncalled_chips.push((eligible[0], pot_amount));
            } else if pot_amount > 1e-6 {
                pots.push(Pot {
                    amount: pot_amount,
                    eligible_players: eligible,
                });
            }

            prev_level = level;
        }

        // Add any remaining chips from folded players who contributed more than max active
        let leftover_folded: f64 = rem_contribs.iter().sum();
        if leftover_folded > 1e-6 && !pots.is_empty() {
            let last_idx = pots.len() - 1;
            pots[last_idx].amount += leftover_folded;
        }

        let main_pot = if !pots.is_empty() {
            pots.remove(0)
        } else {
            Pot {
                amount: 0.0,
                eligible_players: (0..n).filter(|&i| is_active[i]).collect(),
            }
        };

        SidePotResult {
            main_pot,
            side_pots: pots,
            uncalled_chips,
            total_collected,
        }
    }
}
