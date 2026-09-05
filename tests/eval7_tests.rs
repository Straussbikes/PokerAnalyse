use poker_analyse::{
    eval_5hand, eval_7hand, eval_holdem, Card, HandRankCategory, COMBOS_7_CHOOSE_5,
};
use std::time::Instant;

fn parse_7(cards_str: &str) -> [Card; 7] {
    let mut cards = [Card::from_raw(0); 7];
    assert_eq!(cards_str.len(), 14, "Expected 14 characters for 7 cards");
    for i in 0..7 {
        let s = &cards_str[i * 2..i * 2 + 2];
        cards[i] = Card::from_str_exact(s).expect("Failed to parse card");
    }
    cards
}

fn parse_2(cards_str: &str) -> [Card; 2] {
    let mut cards = [Card::from_raw(0); 2];
    assert_eq!(cards_str.len(), 4, "Expected 4 characters for 2 cards");
    for i in 0..2 {
        let s = &cards_str[i * 2..i * 2 + 2];
        cards[i] = Card::from_str_exact(s).expect("Failed to parse card");
    }
    cards
}

fn parse_5(cards_str: &str) -> [Card; 5] {
    let mut cards = [Card::from_raw(0); 5];
    assert_eq!(cards_str.len(), 10, "Expected 10 characters for 5 cards");
    for i in 0..5 {
        let s = &cards_str[i * 2..i * 2 + 2];
        cards[i] = Card::from_str_exact(s).expect("Failed to parse card");
    }
    cards
}

#[test]
fn test_combos_7_choose_5_integrity() {
    assert_eq!(COMBOS_7_CHOOSE_5.len(), 21);
    for combo in &COMBOS_7_CHOOSE_5 {
        assert_eq!(combo.len(), 5);
        for &idx in combo {
            assert!(idx < 7);
        }
        // Indices in each combo must be strictly increasing
        for w in combo.windows(2) {
            assert!(w[0] < w[1]);
        }
    }
}

#[test]
fn test_holdem_playing_the_board() {
    // Board has Royal Flush: Ah Kh Qh Jh Th
    // Player has 2c 3c
    let hole = parse_2("2c3c");
    let board = parse_5("AhKhQhJhTh");

    let score = eval_holdem(hole, board);
    assert_eq!(score, 1, "Player plays the board for a Royal Flush");
    assert_eq!(
        HandRankCategory::from_score(score),
        Some(HandRankCategory::StraightFlush)
    );
}

#[test]
fn test_holdem_playing_one_hole_card() {
    // Board has 4 hearts: Kh Qh Jh 9h 2c
    // Player has Ah 3s (Nut Flush using Ah)
    let hole = parse_2("Ah3s");
    let board = parse_5("KhQhJh9h2c");

    let score = eval_holdem(hole, board);
    assert_eq!(
        HandRankCategory::from_score(score),
        Some(HandRankCategory::Flush)
    );

    // Should equal the 5-card evaluation of Ah Kh Qh Jh 9h
    let expected_5 = parse_5("AhKhQhJh9h");
    assert_eq!(score, eval_5hand(expected_5[0], expected_5[1], expected_5[2], expected_5[3], expected_5[4]));
}

#[test]
fn test_holdem_playing_two_hole_cards() {
    // Player has Pocket Aces: As Ah
    // Board has Ac Kd Ks 2c 3d (Full House Aces full of Kings)
    let hole = parse_2("AsAh");
    let board = parse_5("AcKdKs2c3d");

    let score = eval_holdem(hole, board);
    assert_eq!(
        HandRankCategory::from_score(score),
        Some(HandRankCategory::FullHouse)
    );

    let expected_5 = parse_5("AsAhAcKdKs");
    assert_eq!(score, eval_5hand(expected_5[0], expected_5[1], expected_5[2], expected_5[3], expected_5[4]));
}

#[test]
fn test_holdem_full_house_beats_flush_on_paired_board() {
    // Board: Ks Kh 9s 4s 2s (3 spades, paired Kings)
    let board = parse_5("KsKh9s4s2s");

    // Player A has Kd 9d (Full House: Kings full of Nines)
    let player_a = eval_holdem(parse_2("Kd9d"), board);

    // Player B has As Qs (Ace-high Flush)
    let player_b = eval_holdem(parse_2("AsQs"), board);

    assert_eq!(
        HandRankCategory::from_score(player_a),
        Some(HandRankCategory::FullHouse)
    );
    assert_eq!(
        HandRankCategory::from_score(player_b),
        Some(HandRankCategory::Flush)
    );

    // Player A wins (lower score is better)
    assert!(player_a < player_b);
}

#[test]
fn test_holdem_two_pair_kicker_tiebreak() {
    // Board: Ks Qs 5d 4c 2h
    let board = parse_5("KsQs5d4c2h");

    // Player A: Kh Qh (Two Pair, Kings and Queens, 5 kicker)
    // Actually board has Ks Qs, so player with Kd Qd has Kings and Queens with 5 kicker
    let player_a = eval_holdem(parse_2("KdQd"), board);

    // Player B: Kc Jc (Pair of Kings with Q, J, 5 kickers)
    let player_b = eval_holdem(parse_2("KcJc"), board);

    assert!(player_a < player_b, "Two pair beats One pair");
}

#[test]
fn test_worst_possible_7card_hand() {
    // Worst 5-card is 7-5-4-3-2 unsuited (7462).
    // In 7 cards, you have 7 distinct ranks without straight or flush.
    // Lowest 7 cards without straight: 8-7-6-4-3-2 with no flush.
    // The best 5 of {8, 7, 6, 4, 3, 2, x} will be 8-7-6-4-3 or 8-7-6-4-2 (High Card).
    let cards = parse_7("8s7h6d4c3s2h9d");
    let score = eval_7hand(&cards);
    assert_eq!(
        HandRankCategory::from_score(score),
        Some(HandRankCategory::HighCard)
    );
}

/// Fast 64-bit Xorshift PRNG for generating random hands with zero allocations.
struct Xorshift64(u64);

impl Xorshift64 {
    #[inline(always)]
    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    #[inline(always)]
    fn sample_7(&mut self, deck: &[Card; 52]) -> [Card; 7] {
        let mut selected = [Card::from_raw(0); 7];
        let mut used = 0u64;
        let mut count = 0;
        while count < 7 {
            let idx = (self.next_u64() % 52) as usize;
            let bit = 1u64 << idx;
            if (used & bit) == 0 {
                used |= bit;
                selected[count] = deck[idx];
                count += 1;
            }
        }
        selected
    }
}

#[test]
fn test_single_core_eval7_benchmark() {
    // Generate standard 52-card deck
    let mut deck = [Card::from_raw(0); 52];
    for (i, card_slot) in deck.iter_mut().enumerate() {
        *card_slot = Card::from_index(i).unwrap();
    }

    const BATCH_SIZE: usize = 2_048;
    let mut prng = Xorshift64(0xDEADBEEFCAFE1234);
    let mut hands = Vec::with_capacity(BATCH_SIZE);
    for _ in 0..BATCH_SIZE {
        hands.push(prng.sample_7(&deck));
    }

    // Number of evaluations: 15,000,000
    #[cfg(debug_assertions)]
    let target_evals = 500_000usize;
    #[cfg(not(debug_assertions))]
    let target_evals = 15_000_000usize;

    let iterations = target_evals.div_ceil(BATCH_SIZE);
    let actual_total = iterations * BATCH_SIZE;

    println!("\nStarting Single-Core 7-Card Evaluator Benchmark (total evals: {})...", actual_total);

    let start = Instant::now();
    let mut dummy_acc = 0u64;

    for _ in 0..iterations {
        for hand in &hands {
            let score = eval_7hand(hand);
            dummy_acc = dummy_acc.wrapping_add(score as u64);
        }
    }

    let elapsed = start.elapsed();
    let elapsed_secs = elapsed.as_secs_f64();
    let hands_per_sec = (actual_total as f64) / elapsed_secs;
    let m_hands_per_sec = hands_per_sec / 1_000_000.0;

    println!("--------------------------------------------------");
    println!("Evaluated {} hands in {:.4} seconds", actual_total, elapsed_secs);
    println!("Throughput: {:.2} Million hands/second (Checksum: {})", m_hands_per_sec, dummy_acc);
    println!("--------------------------------------------------");

    // Only assert >= 15M hands/sec in release builds
    #[cfg(not(debug_assertions))]
    {
        assert!(
            m_hands_per_sec >= 15.0,
            "Benchmark failed: throughput {:.2} M hands/sec is below target >= 15.0 M hands/sec",
            m_hands_per_sec
        );
    }
}
