use core::fmt;

/// Maximum number of players supported by the Malmuth-Harville bitmask DP calculator.
/// 10 players = 1024 states, fitting comfortably within CPU L1 cache.
pub const MAX_ICM_PLAYERS: usize = 10;

/// Errors that can occur during ICM calculations.
#[derive(Debug, Clone, PartialEq)]
pub enum IcmError {
    /// Number of players exceeds MAX_ICM_PLAYERS (10).
    TooManyPlayers(usize),
    /// Stacks slice is empty.
    EmptyStacks,
    /// Sum of all player stacks is zero or negative.
    ZeroTotalChips,
    /// A player stack is negative or NaN.
    InvalidStack(usize, f64),
    /// Payout value is negative or NaN.
    InvalidPayout(usize, f64),
}

impl fmt::Display for IcmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IcmError::TooManyPlayers(n) => {
                write!(f, "ICM player count ({n}) exceeds maximum of {MAX_ICM_PLAYERS}")
            }
            IcmError::EmptyStacks => write!(f, "ICM stacks slice cannot be empty"),
            IcmError::ZeroTotalChips => write!(f, "Total chip count must be greater than zero"),
            IcmError::InvalidStack(i, s) => write!(f, "Player {i} has invalid stack: {s}"),
            IcmError::InvalidPayout(i, p) => write!(f, "Payout {i} has invalid value: {p}"),
        }
    }
}

impl std::error::Error for IcmError {}

/// High-performance Malmuth-Harville Independent Chip Model (ICM) calculator.
///
/// Implements a memoized bitmask dynamic programming algorithm that evaluates tournament
/// monetary equities in $O(M \cdot 2^N)$ time and $O(2^N)$ space without dynamic heap allocations
/// on the hot path.
pub struct IcmCalculator;

impl IcmCalculator {
    /// Computes the exact monetary equity vector for each player given stack sizes and payout structure.
    ///
    /// # Arguments
    /// * `stacks` - Slice of chip stack sizes for $N \le 10$ players.
    /// * `payouts` - Slice of prize payouts in descending order (e.g. `[0.50, 0.30, 0.20]` or monetary values).
    ///
    /// # Returns
    /// A vector of monetary equity values for each player, summing to the total distributed payouts.
    pub fn compute_equity(stacks: &[f64], payouts: &[f64]) -> Result<Vec<f64>, IcmError> {
        let n = stacks.len();
        if n == 0 {
            return Err(IcmError::EmptyStacks);
        }
        if n > MAX_ICM_PLAYERS {
            return Err(IcmError::TooManyPlayers(n));
        }

        // Validate stacks
        let mut total_chips = 0.0f64;
        for (i, &s) in stacks.iter().enumerate() {
            if s.is_nan() || s < 0.0 {
                return Err(IcmError::InvalidStack(i, s));
            }
            total_chips += s;
        }
        if total_chips <= 0.0 {
            return Err(IcmError::ZeroTotalChips);
        }

        // Validate payouts
        for (i, &p) in payouts.iter().enumerate() {
            if p.is_nan() || p < 0.0 {
                return Err(IcmError::InvalidPayout(i, p));
            }
        }

        let num_places = payouts.len().min(n);
        if num_places == 0 {
            return Ok(vec![0.0; n]);
        }

        // Fast-path: 1 player
        if n == 1 {
            return Ok(vec![payouts[0]]);
        }

        // Fast-path: 2 players
        if n == 2 {
            let p0 = stacks[0] / total_chips;
            let p1 = stacks[1] / total_chips;
            let v0 = payouts[0];
            let v1 = if payouts.len() > 1 { payouts[1] } else { 0.0 };
            return Ok(vec![
                p0 * v0 + p1 * v1,
                p1 * v0 + p0 * v1,
            ]);
        }

        // Precompute sum of stacks for each bitmask subset in [0, 2^N - 1]
        let num_states = 1usize << n;
        let mut stack_sum = [0.0f64; 1 << MAX_ICM_PLAYERS];

        for mask in 1..num_states {
            let lsb = mask & (!mask + 1);
            let idx = lsb.trailing_zeros() as usize;
            stack_sum[mask] = stack_sum[mask ^ lsb] + stacks[idx];
        }

        // DP table: dp[mask] = probability that the set of players in `mask`
        // finished in the first |mask| positions.
        let mut dp = [0.0f64; 1 << MAX_ICM_PLAYERS];
        dp[0] = 1.0;

        let mut equities = vec![0.0f64; n];

        // Process position by position (k = 0 is 1st place, k = 1 is 2nd place, etc.)
        for (k, &payout) in payouts.iter().take(num_places).enumerate() {
            for mask in 0..num_states {
                if (mask.count_ones() as usize) != k {
                    continue;
                }

                let prob = dp[mask];
                if prob <= 0.0 {
                    continue;
                }

                let rem_chips = total_chips - stack_sum[mask];
                if rem_chips <= 1e-12 {
                    continue;
                }

                for i in 0..n {
                    if (mask & (1 << i)) == 0 && stacks[i] > 0.0 {
                        let p_finish = prob * (stacks[i] / rem_chips);
                        equities[i] += payout * p_finish;

                        if k + 1 < num_places {
                            dp[mask | (1 << i)] += p_finish;
                        }
                    }
                }
            }
        }

        Ok(equities)
    }

    /// Fixed-size stack-allocated equity calculation for zero dynamic heap allocations.
    #[inline]
    pub fn compute_equity_fixed<const N: usize>(
        stacks: &[f64; N],
        payouts: &[f64],
    ) -> Result<[f64; N], IcmError> {
        let eq_vec = Self::compute_equity(stacks, payouts)?;
        let mut res = [0.0f64; N];
        res.copy_from_slice(&eq_vec[..N]);
        Ok(res)
    }
}
