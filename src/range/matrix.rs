use crate::card::{Card, Rank};
use crate::deck::DeckBitmask;
use crate::range::combo::{
    cards_to_combo, combo_to_cards, CanonicalHand, COMBO_TABLES, NUM_HOLE_COMBOS,
};
use core::fmt;
use std::str::FromStr;

/// Errors that can occur when parsing a poker range string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RangeParseError {
    InvalidToken(String),
    InvalidRank(char),
    InvalidWeight(String),
    InvalidRankOrder(String),
}

impl fmt::Display for RangeParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RangeParseError::InvalidToken(tok) => write!(f, "Invalid range token: '{tok}'"),
            RangeParseError::InvalidRank(c) => write!(f, "Invalid card rank: '{c}'"),
            RangeParseError::InvalidWeight(w) => write!(f, "Invalid weight specification: '{w}'"),
            RangeParseError::InvalidRankOrder(s) => write!(f, "Invalid rank order in token: '{s}'"),
        }
    }
}

impl std::error::Error for RangeParseError {}

/// Represents a probability distribution or selection weight across all 1326 hole-card combinations.
///
/// Backed by a zero-allocation flat array `[f32; 1326]` resident in CPU L1 cache.
#[derive(Clone, PartialEq)]
pub struct Range {
    pub weights: [f32; NUM_HOLE_COMBOS],
}

impl Range {
    /// Creates an empty range with all weights set to 0.0.
    #[inline]
    pub const fn empty() -> Self {
        Range {
            weights: [0.0; NUM_HOLE_COMBOS],
        }
    }

    /// Creates a uniform range with all weights set to 1.0 (100% of hands).
    #[inline]
    pub const fn uniform() -> Self {
        Range {
            weights: [1.0; NUM_HOLE_COMBOS],
        }
    }

    /// Creates a Range from a custom array of 1326 weights.
    #[inline]
    pub const fn from_weights(weights: [f32; NUM_HOLE_COMBOS]) -> Self {
        Range { weights }
    }

    /// Returns the weight of a specific pair of cards.
    #[inline(always)]
    pub fn get_weight(&self, c1: Card, c2: Card) -> f32 {
        let idx = cards_to_combo(c1, c2);
        self.weights[idx]
    }

    /// Sets the weight of a specific pair of cards.
    #[inline(always)]
    pub fn set_weight(&mut self, c1: Card, c2: Card, weight: f32) {
        let idx = cards_to_combo(c1, c2);
        self.weights[idx] = weight;
    }

    /// Returns the sum of all combination weights in the range.
    #[inline]
    pub fn total_weight(&self) -> f32 {
        self.weights.iter().sum()
    }

    /// Normalizes the range weights so that the sum of all weights equals 1.0.
    /// If total weight is 0.0, leaves the range unchanged.
    pub fn normalize(&mut self) {
        let sum = self.total_weight();
        if sum > 0.0 {
            let inv_sum = 1.0 / sum;
            for w in self.weights.iter_mut() {
                *w *= inv_sum;
            }
        }
    }

    /// Eliminates all blocked combinations overlapping with dead cards by setting their weights to 0.0.
    ///
    /// Executes in sub-microsecond time with zero allocations.
    #[inline]
    pub fn apply_blockers(&mut self, dead_cards: DeckBitmask) {
        let dead_mask = dead_cards.as_u64();
        if dead_mask == 0 {
            return;
        }

        for i in 0..NUM_HOLE_COMBOS {
            let combo_mask = COMBO_TABLES.combo_to_mask[i].as_u64();
            if (combo_mask & dead_mask) != 0 {
                self.weights[i] = 0.0;
            }
        }
    }

    /// Returns an iterator yielding active combinations with weight > 0.
    /// Yields `(combo_index, (card1, card2), weight)`.
    pub fn active_combos(&self) -> impl Iterator<Item = (usize, (Card, Card), f32)> + '_ {
        self.weights
            .iter()
            .enumerate()
            .filter(|(_, &w)| w > 0.0)
            .map(|(idx, &w)| (idx, combo_to_cards(idx), w))
    }

    /// Sets all combinations of a canonical hand category to a specified weight.
    pub fn set_canonical_weight(&mut self, hand: CanonicalHand, weight: f32) {
        for idx in hand.combo_indices() {
            self.weights[idx] = weight;
        }
    }

    /// Parses a standard poker range string (e.g. `"AA, KK, AKs, AKo, TT+, A5s-A2s, 0.5*87s"`).
    pub fn parse(input: &str) -> Result<Self, RangeParseError> {
        let mut range = Range::empty();
        if input.trim().is_empty() {
            return Ok(range);
        }

        let tokens = input.split([',', ';', ' ']).filter(|s| !s.trim().is_empty());
        for raw_token in tokens {
            let token = raw_token.trim();
            if token.is_empty() {
                continue;
            }

            // Parse weight prefix/suffix: "0.5*AA" or "AA:0.5"
            let (weight, hand_spec) = if let Some((w_str, spec)) = token.split_once('*') {
                let w = w_str.parse::<f32>().map_err(|_| RangeParseError::InvalidWeight(w_str.to_string()))?;
                (w, spec.trim())
            } else if let Some((spec, w_str)) = token.split_once(':') {
                let w = w_str.parse::<f32>().map_err(|_| RangeParseError::InvalidWeight(w_str.to_string()))?;
                (w, spec.trim())
            } else {
                (1.0f32, token)
            };

            parse_hand_group(&mut range, hand_spec, weight)?;
        }

        Ok(range)
    }
}

impl Default for Range {
    fn default() -> Self {
        Range::empty()
    }
}

impl FromStr for Range {
    type Err = RangeParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Range::parse(s)
    }
}

fn parse_rank(c: char) -> Result<Rank, RangeParseError> {
    Rank::from_char(c).map_err(|_| RangeParseError::InvalidRank(c))
}

fn parse_hand_group(range: &mut Range, spec: &str, weight: f32) -> Result<(), RangeParseError> {
    // 1. Specific 2-card combo: e.g. "AhKh" or "AsKd"
    if spec.len() == 4 {
        let chars: Vec<char> = spec.chars().collect();
        if let (Ok(c1), Ok(c2)) = (
            Card::from_chars(chars[0], chars[1]),
            Card::from_chars(chars[2], chars[3]),
        ) {
            range.set_weight(c1, c2, weight);
            return Ok(());
        }
    }

    // 2. Dash range notation: "99-66", "A5s-A2s", "KJo-K9o"
    if let Some((start_spec, end_spec)) = spec.split_once('-') {
        return parse_dash_range(range, start_spec.trim(), end_spec.trim(), weight);
    }

    // 3. Plus notation: "TT+", "AJs+", "KQo+"
    if let Some(base) = spec.strip_suffix('+') {
        return parse_plus_notation(range, base, weight);
    }

    // 4. Single canonical category: "AA", "AKs", "AKo"
    parse_single_canonical(range, spec, weight)
}

fn parse_single_canonical(range: &mut Range, spec: &str, weight: f32) -> Result<(), RangeParseError> {
    let chars: Vec<char> = spec.chars().collect();
    if chars.len() == 2 {
        let r1 = parse_rank(chars[0])?;
        let r2 = parse_rank(chars[1])?;
        if r1 == r2 {
            range.set_canonical_weight(CanonicalHand::Pair(r1), weight);
            return Ok(());
        } else {
            // Unspecified suitedness defaults to both suited and offsuit (e.g. "AK" = AKs + AKo)
            let (high, low) = if r1 > r2 { (r1, r2) } else { (r2, r1) };
            range.set_canonical_weight(CanonicalHand::Suited(high, low), weight);
            range.set_canonical_weight(CanonicalHand::Offsuit(high, low), weight);
            return Ok(());
        }
    } else if chars.len() == 3 {
        let r1 = parse_rank(chars[0])?;
        let r2 = parse_rank(chars[1])?;
        let s_flag = chars[2];

        let (high, low) = if r1 > r2 { (r1, r2) } else { (r2, r1) };
        match s_flag {
            's' | 'S' => {
                range.set_canonical_weight(CanonicalHand::Suited(high, low), weight);
                return Ok(());
            }
            'o' | 'O' => {
                range.set_canonical_weight(CanonicalHand::Offsuit(high, low), weight);
                return Ok(());
            }
            _ => return Err(RangeParseError::InvalidToken(spec.to_string())),
        }
    }

    Err(RangeParseError::InvalidToken(spec.to_string()))
}

fn parse_plus_notation(range: &mut Range, base: &str, weight: f32) -> Result<(), RangeParseError> {
    let chars: Vec<char> = base.chars().collect();
    if chars.len() == 2 && chars[0] == chars[1] {
        // Pair plus: "TT+" -> TT, JJ, QQ, KK, AA
        let start_rank = parse_rank(chars[0])? as usize;
        for r_idx in start_rank..=12 {
            range.set_canonical_weight(CanonicalHand::Pair(Rank::ALL[r_idx]), weight);
        }
        return Ok(());
    } else if chars.len() == 3 {
        let r1 = parse_rank(chars[0])?;
        let r2 = parse_rank(chars[1])?;
        let s_flag = chars[2];

        if r1 <= r2 {
            return Err(RangeParseError::InvalidRankOrder(base.to_string()));
        }

        let kicker_start = r2 as usize;
        let kicker_end = (r1 as usize) - 1;

        match s_flag {
            's' | 'S' => {
                for k in kicker_start..=kicker_end {
                    range.set_canonical_weight(CanonicalHand::Suited(r1, Rank::ALL[k]), weight);
                }
                return Ok(());
            }
            'o' | 'O' => {
                for k in kicker_start..=kicker_end {
                    range.set_canonical_weight(CanonicalHand::Offsuit(r1, Rank::ALL[k]), weight);
                }
                return Ok(());
            }
            _ => return Err(RangeParseError::InvalidToken(base.to_string())),
        }
    }

    Err(RangeParseError::InvalidToken(base.to_string()))
}

fn parse_dash_range(
    range: &mut Range,
    start: &str,
    end: &str,
    weight: f32,
) -> Result<(), RangeParseError> {
    let c_start: Vec<char> = start.chars().collect();
    let c_end: Vec<char> = end.chars().collect();

    // Pair range: "99-66"
    if c_start.len() == 2 && c_end.len() == 2 && c_start[0] == c_start[1] && c_end[0] == c_end[1] {
        let r_high = parse_rank(c_start[0])? as usize;
        let r_low = parse_rank(c_end[0])? as usize;
        let (min_r, max_r) = if r_high > r_low {
            (r_low, r_high)
        } else {
            (r_high, r_low)
        };
        for r in min_r..=max_r {
            range.set_canonical_weight(CanonicalHand::Pair(Rank::ALL[r]), weight);
        }
        return Ok(());
    }

    // Suited / Offsuit range with fixed high card: "A5s-A2s" or "KJo-K9o"
    if c_start.len() == 3 && c_end.len() == 3 && c_start[0] == c_end[0] && c_start[2] == c_end[2] {
        let fixed_rank = parse_rank(c_start[0])?;
        let k1 = parse_rank(c_start[1])? as usize;
        let k2 = parse_rank(c_end[1])? as usize;
        let (min_k, max_k) = if k1 > k2 { (k2, k1) } else { (k1, k2) };
        let s_flag = c_start[2];

        match s_flag {
            's' | 'S' => {
                for k in min_k..=max_k {
                    range.set_canonical_weight(CanonicalHand::Suited(fixed_rank, Rank::ALL[k]), weight);
                }
                return Ok(());
            }
            'o' | 'O' => {
                for k in min_k..=max_k {
                    range.set_canonical_weight(CanonicalHand::Offsuit(fixed_rank, Rank::ALL[k]), weight);
                }
                return Ok(());
            }
            _ => return Err(RangeParseError::InvalidToken(start.to_string())),
        }
    }

    Err(RangeParseError::InvalidToken(format!("{start}-{end}")))
}
