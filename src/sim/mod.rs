use crate::card::Card;
use crate::deck::DeckBitmask;
use crate::eval::eval_7hand;
use crate::range::combo::combo_to_cards;
use crate::range::matrix::Range;

pub mod ev;
pub use ev::{ActionEv, CandidateAction, DecisionReport, EvContext, EvEngine};

/// Lightweight, ultra-fast 64-bit Xorshift PRNG for non-allocating Monte Carlo runouts.
#[derive(Debug, Clone)]
pub struct FastPrng(pub u64);

impl FastPrng {
    #[inline(always)]
    pub fn new(seed: u64) -> Self {
        FastPrng(if seed == 0 { 0xDEADBEEFCAFE1234 } else { seed })
    }

    #[inline(always)]
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    #[inline(always)]
    pub fn next_f32(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u32 << 24) as f32
    }
}

/// Result of a Monte Carlo equity simulation.
#[derive(Debug, Clone, PartialEq)]
pub struct EquityResult {
    /// Hero total equity (win rate + chop shares).
    pub equity: f64,
    /// Pure win frequency.
    pub win_rate: f64,
    /// Tie / split pot frequency.
    pub tie_rate: f64,
    /// Number of simulated samples.
    pub samples: usize,
    /// Time taken for the simulation in microseconds.
    pub elapsed_micros: u64,
}

/// Cumulative distribution lookup for fast alias/inverse-CDF sampling of a Villain range.
#[derive(Debug, Clone)]
pub struct RangeSampler {
    /// Active combo indices.
    pub combo_indices: Vec<usize>,
    /// Cumulative weights normalized to [0.0, 1.0].
    pub cum_weights: Vec<f32>,
}

impl RangeSampler {
    pub fn new(range: &Range, dead: DeckBitmask) -> Option<Self> {
        let mut filtered = range.clone();
        filtered.apply_blockers(dead);

        let total = filtered.total_weight();
        if total <= 1e-6 {
            return None;
        }

        let mut combo_indices = Vec::with_capacity(128);
        let mut cum_weights = Vec::with_capacity(128);
        let mut acc = 0.0f32;

        for (idx, &w) in filtered.weights.iter().enumerate() {
            if w > 0.0 {
                acc += w;
                combo_indices.push(idx);
                cum_weights.push(acc / total);
            }
        }

        Some(RangeSampler {
            combo_indices,
            cum_weights,
        })
    }

    #[inline(always)]
    pub fn sample(&self, prng: &mut FastPrng) -> (Card, Card) {
        let r = prng.next_f32();
        let idx = match self.cum_weights.binary_search_by(|w| w.partial_cmp(&r).unwrap()) {
            Ok(i) => i,
            Err(i) => i.min(self.combo_indices.len() - 1),
        };
        combo_to_cards(self.combo_indices[idx])
    }
}

/// High-speed Monte Carlo simulator for Texas Hold'em runouts.
pub struct MonteCarloSimulator;

impl MonteCarloSimulator {
    /// Simulates Hero equity against 1 to 3 opponents with weighted ranges.
    ///
    /// # Arguments
    /// * `hero` - Hero's 2 hole cards.
    /// * `board` - Community cards on the board (0, 3, 4, or 5 cards).
    /// * `villain_ranges` - Slice of opponent Ranges (1 to 3 active opponents).
    /// * `dead_cards` - Cards blocked from the deck (Hero cards + Board + folded cards).
    /// * `samples` - Number of Monte Carlo iterations (e.g. 10,000).
    /// * `seed` - Optional PRNG seed.
    pub fn simulate_equity(
        hero: [Card; 2],
        board: &[Card],
        villain_ranges: &[Range],
        dead_cards: DeckBitmask,
        samples: usize,
        seed: Option<u64>,
    ) -> EquityResult {
        let start = std::time::Instant::now();
        let mut prng = FastPrng::new(seed.unwrap_or(0xCAFEBABEDEAD1234));

        let mut initial_dead = dead_cards;
        initial_dead.add_card(hero[0]);
        initial_dead.add_card(hero[1]);
        for &b in board {
            initial_dead.add_card(b);
        }

        // Precompute range samplers for each active villain
        let mut samplers = Vec::with_capacity(villain_ranges.len());
        for vr in villain_ranges {
            if let Some(sampler) = RangeSampler::new(vr, initial_dead) {
                samplers.push(sampler);
            }
        }

        let num_villains = samplers.len();
        if num_villains == 0 {
            // No active villains: Hero wins uncontested
            return EquityResult {
                equity: 1.0,
                win_rate: 1.0,
                tie_rate: 0.0,
                samples,
                elapsed_micros: start.elapsed().as_micros() as u64,
            };
        }

        let cards_needed = 5 - board.len();
        let mut wins = 0.0f64;
        let mut pure_wins = 0.0f64;
        let mut ties = 0.0f64;

        // Reusable array buffer of available deck cards
        let available_deck_mask = DeckBitmask::full().dead_cards(initial_dead);
        let mut available_deck = [Card::from_raw(0); 52];
        let mut deck_len = 0;
        for c in available_deck_mask {
            available_deck[deck_len] = c;
            deck_len += 1;
        }

        for _ in 0..samples {
            let mut sample_dead = initial_dead;
            let mut villain_hands = [(Card::from_raw(0), Card::from_raw(0)); 3];
            let mut valid_sample = true;

            for v in 0..num_villains {
                let mut attempts = 0;
                let mut v_cards = samplers[v].sample(&mut prng);

                // Ensure sampled villain cards don't collide with already chosen cards
                while (sample_dead.has_card(v_cards.0) || sample_dead.has_card(v_cards.1)) && attempts < 10 {
                    v_cards = samplers[v].sample(&mut prng);
                    attempts += 1;
                }

                if sample_dead.has_card(v_cards.0) || sample_dead.has_card(v_cards.1) {
                    valid_sample = false;
                    break;
                }

                sample_dead.add_card(v_cards.0);
                sample_dead.add_card(v_cards.1);
                villain_hands[v] = v_cards;
            }

            if !valid_sample {
                continue;
            }

            // Sample runout cards to reach 5 board cards
            let mut full_board = [Card::from_raw(0); 5];
            for (i, &b) in board.iter().enumerate() {
                full_board[i] = b;
            }

            if cards_needed > 0 {
                let mut runout_count = 0;
                let mut pool = available_deck;
                let mut pool_len = deck_len;

                while runout_count < cards_needed && pool_len > 0 {
                    let pick_idx = (prng.next_u64() as usize) % pool_len;
                    let candidate = pool[pick_idx];
                    pool_len -= 1;
                    pool[pick_idx] = pool[pool_len];

                    if !sample_dead.has_card(candidate) {
                        full_board[board.len() + runout_count] = candidate;
                        runout_count += 1;
                    }
                }
            }

            // Evaluate Hero hand
            let hero_7 = [
                hero[0], hero[1],
                full_board[0], full_board[1], full_board[2], full_board[3], full_board[4],
            ];
            let hero_score = eval_7hand(&hero_7);

            // Evaluate all Villain hands
            let mut best_villain_score = 9999u16;
            let mut villain_count_at_best = 0;

            for &(v_c0, v_c1) in villain_hands.iter().take(num_villains) {
                let v_7 = [
                    v_c0, v_c1,
                    full_board[0], full_board[1], full_board[2], full_board[3], full_board[4],
                ];
                let v_score = eval_7hand(&v_7);

                if v_score < best_villain_score {
                    best_villain_score = v_score;
                    villain_count_at_best = 1;
                } else if v_score == best_villain_score {
                    villain_count_at_best += 1;
                }
            }

            // Determine outcome (lower score is stronger in Cactus-Kev)
            if hero_score < best_villain_score {
                wins += 1.0;
                pure_wins += 1.0;
            } else if hero_score == best_villain_score {
                let share = 1.0 / ((villain_count_at_best + 1) as f64);
                wins += share;
                ties += 1.0;
            }
        }

        let elapsed = start.elapsed();
        let f_samples = samples as f64;

        EquityResult {
            equity: wins / f_samples,
            win_rate: pure_wins / f_samples,
            tie_rate: ties / f_samples,
            samples,
            elapsed_micros: elapsed.as_micros() as u64,
        }
    }
}
