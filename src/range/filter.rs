use crate::card::Card;
use crate::eval::{eval_5hand, eval_7hand};
use crate::range::combo::combo_to_cards;
use crate::range::matrix::Range;

/// Hand strength classification bucket for a given board texture.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum HandStrengthBucket {
    /// Weak high card, missed draws, no pair.
    Air = 0,
    /// Significant draw (Flush draw, Open-Ended Straight Draw, Gutshot).
    Draw = 1,
    /// Weak/middle pair, underpair, pocket pair below top card.
    Marginal = 2,
    /// Top pair with strong kicker, Overpair, Two Pair.
    Strong = 3,
    /// Sets, Full House, Flush, Straight, Quads, Straight Flush.
    Monster = 4,
}

/// Simplified 3-tier categorization matching the Roadmap requirements (Air, Marginal, Strong).
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum ThreeTierBucket {
    Air,
    Marginal,
    Strong,
}

impl HandStrengthBucket {
    /// Maps 5-tier classification into the 3-tier roadmap model (Air, Marginal, Strong).
    #[inline(always)]
    pub const fn to_three_tier(self) -> ThreeTierBucket {
        match self {
            HandStrengthBucket::Air => ThreeTierBucket::Air,
            HandStrengthBucket::Draw | HandStrengthBucket::Marginal => ThreeTierBucket::Marginal,
            HandStrengthBucket::Strong | HandStrengthBucket::Monster => ThreeTierBucket::Strong,
        }
    }
}

/// Actions an opponent can take on a betting street.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum OpponentAction {
    Check = 0,
    Call = 1,
    Bet = 2,
    Raise = 3,
    Fold = 4,
}

/// Standard player archetypes with characteristic tendency deviations.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum OpponentArchetype {
    /// Extremely tight, rarely bluffs, raises only with Strong/Monster.
    Nit,
    /// Extremely sticky, calls down with Marginal/Draws, rarely raises.
    CallingStation,
    /// Ultra-aggressive, bluffs frequently with Air/Draws, raises wide.
    Maniac,
    /// Balanced standard approximation.
    Balanced,
}

/// Empirical conditional probability matrix: P(Action | Bucket).
#[derive(Debug, Clone, PartialEq)]
pub struct TendencyMatrix {
    /// Probabilities: table[action_idx][bucket_idx] in [0.0, 1.0].
    pub table: [[f32; 5]; 5],
}

impl TendencyMatrix {
    /// Creates a custom TendencyMatrix from a 5x5 array [Action][Bucket].
    #[inline]
    pub const fn new(table: [[f32; 5]; 5]) -> Self {
        TendencyMatrix { table }
    }

    /// Returns the likelihood P(Action | Bucket).
    #[inline(always)]
    pub fn likelihood(&self, action: OpponentAction, bucket: HandStrengthBucket) -> f32 {
        self.table[action as usize][bucket as usize]
    }

    /// Returns the pre-calibrated TendencyMatrix for a given archetype.
    pub fn from_archetype(archetype: OpponentArchetype) -> Self {
        match archetype {
            OpponentArchetype::Nit => {
                // Actions: Check, Call, Bet, Raise, Fold
                // Buckets: Air, Draw, Marginal, Strong, Monster
                TendencyMatrix::new([
                    // Check: [Air, Draw, Marginal, Strong, Monster]
                    [0.60, 0.40, 0.70, 0.15, 0.05],
                    // Call:
                    [0.05, 0.45, 0.65, 0.35, 0.10],
                    // Bet:
                    [0.02, 0.10, 0.20, 0.70, 0.85],
                    // Raise: (under-bluffs: almost 0% with Air, raises pure value)
                    [0.01, 0.05, 0.05, 0.50, 0.90],
                    // Fold:
                    [0.95, 0.50, 0.25, 0.02, 0.00],
                ])
            }
            OpponentArchetype::CallingStation => {
                TendencyMatrix::new([
                    // Check:
                    [0.50, 0.25, 0.50, 0.20, 0.10],
                    // Call: (extremely high calling frequency with marginal & draws)
                    [0.20, 0.70, 0.85, 0.75, 0.20],
                    // Bet:
                    [0.05, 0.10, 0.15, 0.50, 0.80],
                    // Raise: (rarely raises without the absolute nuts)
                    [0.01, 0.02, 0.05, 0.20, 0.85],
                    // Fold: (under-folds across the board)
                    [0.70, 0.20, 0.10, 0.01, 0.00],
                ])
            }
            OpponentArchetype::Maniac => {
                TendencyMatrix::new([
                    // Check: (rarely checks)
                    [0.20, 0.15, 0.20, 0.10, 0.05],
                    // Call:
                    [0.25, 0.35, 0.40, 0.30, 0.15],
                    // Bet: (bluffs air and draws heavily)
                    [0.45, 0.50, 0.40, 0.65, 0.75],
                    // Raise: (high bluff-raise frequency)
                    [0.35, 0.45, 0.25, 0.60, 0.85],
                    // Fold:
                    [0.40, 0.15, 0.10, 0.02, 0.00],
                ])
            }
            OpponentArchetype::Balanced => {
                TendencyMatrix::new([
                    // Check:
                    [0.60, 0.35, 0.65, 0.25, 0.10],
                    // Call:
                    [0.10, 0.45, 0.55, 0.45, 0.15],
                    // Bet:
                    [0.25, 0.30, 0.25, 0.65, 0.80],
                    // Raise:
                    [0.15, 0.25, 0.10, 0.45, 0.85],
                    // Fold:
                    [0.80, 0.40, 0.20, 0.02, 0.00],
                ])
            }
        }
    }
}

/// Classifies a 2-card combination against a community board into a HandStrengthBucket.
pub fn classify_combo(c1: Card, c2: Card, board: &[Card]) -> HandStrengthBucket {
    if board.is_empty() {
        return classify_preflop(c1, c2);
    }

    let b_len = board.len();
    if b_len >= 5 {
        // 7 cards: use eval_7hand
        let cards = [c1, c2, board[0], board[1], board[2], board[3], board[4]];
        let score = eval_7hand(&cards);
        classify_from_score_and_board(score, c1, c2, board)
    } else if b_len == 3 {
        // 5 cards: use eval_5hand
        let score = eval_5hand(c1, c2, board[0], board[1], board[2]);
        classify_from_score_and_board(score, c1, c2, board)
    } else {
        // 4 board cards (Turn: 6 cards total) -> evaluate all 6 subsets of 5 cards
        let six = [c1, c2, board[0], board[1], board[2], board[3]];
        let mut best = 9999u16;
        for drop_idx in 0..6 {
            let mut five = [Card::from_raw(0); 5];
            let mut idx = 0;
            for (i, &card) in six.iter().enumerate() {
                if i != drop_idx {
                    five[idx] = card;
                    idx += 1;
                }
            }
            let s = eval_5hand(five[0], five[1], five[2], five[3], five[4]);
            best = best.min(s);
        }
        classify_from_score_and_board(best, c1, c2, board)
    }
}

fn classify_preflop(c1: Card, c2: Card) -> HandStrengthBucket {
    let r1 = c1.rank() as u8;
    let r2 = c2.rank() as u8;
    let (high, low) = if r1 > r2 { (r1, r2) } else { (r2, r1) };
    let suited = c1.suit() == c2.suit();

    // Pocket pairs
    if high == low {
        return if high >= 10 {
            // TT, JJ, QQ, KK, AA
            HandStrengthBucket::Monster
        } else if high >= 7 {
            // 77, 88, 99
            HandStrengthBucket::Strong
        } else {
            // 22-66
            HandStrengthBucket::Marginal
        };
    }

    // High Broadway / Ace combos
    if high == 12 {
        // Ace-high
        if low >= 10 {
            // AK, AQ, AJ
            return HandStrengthBucket::Strong;
        } else if suited {
            // A2s-ATs
            return HandStrengthBucket::Marginal;
        } else if low >= 8 {
            return HandStrengthBucket::Marginal;
        } else {
            return HandStrengthBucket::Air;
        }
    }

    if high >= 10 && low >= 9 {
        // KQ, KJ, QJ
        return if suited {
            HandStrengthBucket::Strong
        } else {
            HandStrengthBucket::Marginal
        };
    }

    if suited && (high - low == 1) && low >= 4 {
        // Suited connectors 54s - T9s
        return HandStrengthBucket::Draw;
    }

    HandStrengthBucket::Air
}

fn classify_from_score_and_board(
    score: u16,
    c1: Card,
    c2: Card,
    board: &[Card],
) -> HandStrengthBucket {
    // Scores in Cactus-Kev:
    // 1..=10: Straight Flush -> Monster
    // 11..=166: Four of a Kind -> Monster
    // 167..=322: Full House -> Monster
    // 323..=1599: Flush -> Monster
    // 1600..=1609: Straight -> Monster
    // 1610..=2467: Three of a Kind -> Monster
    if score <= 2467 {
        return HandStrengthBucket::Monster;
    }

    // Two Pair (2468..=3325): Strong
    if score <= 3325 {
        return HandStrengthBucket::Strong;
    }

    // One Pair (3326..=6185):
    if score <= 6185 {
        let max_board_rank = board.iter().map(|c| c.rank() as u8).max().unwrap_or(0);
        let r1 = c1.rank() as u8;
        let r2 = c2.rank() as u8;

        // Pocket pair (Overpair or Underpair)
        if r1 == r2 {
            return if r1 > max_board_rank {
                HandStrengthBucket::Strong // Overpair
            } else {
                HandStrengthBucket::Marginal // Underpair / middle pocket pair
            };
        }

        // Pair with the board
        let paired_rank = if board.iter().any(|b| b.rank() as u8 == r1) {
            r1
        } else if board.iter().any(|b| b.rank() as u8 == r2) {
            r2
        } else {
            // Paired on board alone: check kicker
            let high_hole = r1.max(r2);
            return if high_hole >= 11 {
                HandStrengthBucket::Marginal
            } else {
                HandStrengthBucket::Air
            };
        };

        if paired_rank == max_board_rank {
            // Top pair: check kicker
            let kicker = if paired_rank == r1 { r2 } else { r1 };
            // Kicker Ace, King, Queen, or Jack is Strong
            return if kicker >= 9 {
                HandStrengthBucket::Strong
            } else {
                HandStrengthBucket::Marginal
            };
        } else {
            // 2nd pair, 3rd pair, etc.
            return HandStrengthBucket::Marginal;
        }
    }

    // High Card (6186..=7462): Check for Flush Draw and Straight Draw
    if has_flush_draw(c1, c2, board) || has_straight_draw(c1, c2, board) {
        return HandStrengthBucket::Draw;
    }

    HandStrengthBucket::Air
}

/// Detects if the hand + board forms a 4-card flush draw.
fn has_flush_draw(c1: Card, c2: Card, board: &[Card]) -> bool {
    let mut suit_counts = [0u8; 4];
    suit_counts[c1.suit_bitmask().trailing_zeros() as usize] += 1;
    suit_counts[c2.suit_bitmask().trailing_zeros() as usize] += 1;
    for b in board {
        suit_counts[b.suit_bitmask().trailing_zeros() as usize] += 1;
    }
    suit_counts[0] == 4 || suit_counts[1] == 4 || suit_counts[2] == 4 || suit_counts[3] == 4
}

/// Detects if the hand + board forms a straight draw (OESD or Gutshot).
fn has_straight_draw(c1: Card, c2: Card, board: &[Card]) -> bool {
    let mut ranks_mask = (1u32 << (c1.rank() as u32)) | (1u32 << (c2.rank() as u32));
    for b in board {
        ranks_mask |= 1u32 << (b.rank() as u32);
    }
    // Also include Ace as low (bit 13) for wheel wrap
    if (ranks_mask & (1 << 12)) != 0 {
        ranks_mask |= 1 << 13; // represent Ace as bit 13 or shift
    }

    // Check all 10 5-rank spans (A-2-3-4-5 up to T-J-Q-K-A).
    // A 4-card straight draw has exactly 4 bits set in any 5-card span!
    // Spans in 13-bit representation (with Ace low handled):
    let spans: [u32; 10] = [
        0x100F, // A-2-3-4-5 (Wheel)
        0x001F, // 2-3-4-5-6
        0x003E, // 3-4-5-6-7
        0x007C, // 4-5-6-7-8
        0x00F8, // 5-6-7-8-9
        0x01F0, // 6-7-8-9-T
        0x03E0, // 7-8-9-T-J
        0x07C0, // 8-9-T-J-Q
        0x0F80, // 9-T-J-Q-K
        0x1F00, // T-J-Q-K-A (Broadway)
    ];

    for &span in &spans {
        if (ranks_mask & span).count_ones() == 4 {
            return true;
        }
    }

    false
}

impl Range {
    /// Applies Bayesian updating over the 1326 combinations conditioned on an opponent's observed action:
    ///
    /// $$W'_c = W_c \times P(\text{Action} \mid \text{HandStrengthBucket}(c, \text{Board}), \text{Profile})$$
    ///
    /// # Arguments
    /// * `board` - Community cards on the board (e.g. 3 cards on Flop, 4 on Turn, 5 on River).
    /// * `action` - Observed action of the opponent (Check, Call, Bet, Raise, Fold).
    /// * `profile` - Tendency matrix defining conditional action probabilities.
    /// * `normalize` - If true, re-scales the resulting range weights so their sum equals 1.0.
    pub fn apply_bayesian_update(
        &mut self,
        board: &[Card],
        action: OpponentAction,
        profile: &TendencyMatrix,
        normalize: bool,
    ) {
        for i in 0..crate::range::combo::NUM_HOLE_COMBOS {
            if self.weights[i] <= 0.0 {
                continue;
            }

            let (c1, c2) = combo_to_cards(i);
            let bucket = classify_combo(c1, c2, board);
            let likelihood = profile.likelihood(action, bucket);

            self.weights[i] *= likelihood;
        }

        if normalize {
            self.normalize();
        }
    }
}
