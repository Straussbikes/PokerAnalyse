use poker_analyse::card::{Card, Rank, Suit};
use poker_analyse::deck::DeckBitmask;
use poker_analyse::range::{
    cards_to_combo, combo_to_bitmask, combo_to_cards, CanonicalHand, Range, NUM_HOLE_COMBOS,
};
use std::collections::HashSet;

#[test]
fn test_all_1326_combos_bijection() {
    assert_eq!(NUM_HOLE_COMBOS, 1326);

    let mut seen_indices = HashSet::new();

    for c1_idx in 0..52 {
        let card1 = Card::from_index(c1_idx).unwrap();
        for c2_idx in (c1_idx + 1)..52 {
            let card2 = Card::from_index(c2_idx).unwrap();

            let idx = cards_to_combo(card1, card2);
            assert!(
                idx < NUM_HOLE_COMBOS,
                "Combo index {idx} out of range [0, 1326)"
            );

            // Symmetry check
            assert_eq!(
                cards_to_combo(card2, card1),
                idx,
                "cards_to_combo must be order-independent"
            );

            // Unique index check
            assert!(
                seen_indices.insert(idx),
                "Duplicate combo index detected: {idx}"
            );

            // Reverse lookup check
            let (ret_c1, ret_c2) = combo_to_cards(idx);
            assert_eq!(ret_c1, card1);
            assert_eq!(ret_c2, card2);

            // Bitmask representation check
            let mask = combo_to_bitmask(idx);
            assert_eq!(mask.count(), 2);
            assert!(mask.has_card(card1));
            assert!(mask.has_card(card2));
        }
    }

    assert_eq!(
        seen_indices.len(),
        1326,
        "All 1326 combinations must be uniquely indexed"
    );
}

#[test]
fn test_canonical_169_hands_partition() {
    let mut total_combos_seen = 0;
    let mut combo_set = HashSet::new();

    // 13 pocket pairs
    for r in 0..13 {
        let hand = CanonicalHand::Pair(Rank::ALL[r]);
        let combos = hand.combo_indices();
        assert_eq!(combos.len(), 6);
        for idx in combos {
            assert!(combo_set.insert(idx));
            total_combos_seen += 1;
        }
    }
    assert_eq!(total_combos_seen, 78);

    // 78 suited hands
    for r1 in 0..13 {
        for r2 in 0..r1 {
            let hand = CanonicalHand::Suited(Rank::ALL[r1], Rank::ALL[r2]);
            let combos = hand.combo_indices();
            assert_eq!(combos.len(), 4);
            for idx in combos {
                assert!(combo_set.insert(idx));
                total_combos_seen += 1;
            }
        }
    }
    assert_eq!(total_combos_seen, 78 + 312);

    // 78 offsuit hands
    for r1 in 0..13 {
        for r2 in 0..r1 {
            let hand = CanonicalHand::Offsuit(Rank::ALL[r1], Rank::ALL[r2]);
            let combos = hand.combo_indices();
            assert_eq!(combos.len(), 12);
            for idx in combos {
                assert!(combo_set.insert(idx));
                total_combos_seen += 1;
            }
        }
    }

    assert_eq!(total_combos_seen, 1326);
    assert_eq!(combo_set.len(), 1326);
}

#[test]
fn test_range_parser_syntax() {
    // Single categories
    let range_aa = Range::parse("AA").unwrap();
    assert_eq!(range_aa.total_weight(), 6.0);

    let range_aks = Range::parse("AKs").unwrap();
    assert_eq!(range_aks.total_weight(), 4.0);

    let range_ako = Range::parse("AKo").unwrap();
    assert_eq!(range_ako.total_weight(), 12.0);

    let range_ak = Range::parse("AK").unwrap();
    assert_eq!(range_ak.total_weight(), 16.0);

    // Plus notation for pairs: TT+ (TT, JJ, QQ, KK, AA = 5 * 6 = 30)
    let range_tt_plus = Range::parse("TT+").unwrap();
    assert_eq!(range_tt_plus.total_weight(), 30.0);

    // Plus notation for suited: ATs+ (ATs, AJs, AQs, AKs = 4 * 4 = 16)
    let range_ats_plus = Range::parse("ATs+").unwrap();
    assert_eq!(range_ats_plus.total_weight(), 16.0);

    // Plus notation for offsuit: AJo+ (AJo, AQo, AKo = 3 * 12 = 36)
    let range_ajo_plus = Range::parse("AJo+").unwrap();
    assert_eq!(range_ajo_plus.total_weight(), 36.0);

    // Dash notation for pairs: 99-77 (99, 88, 77 = 3 * 6 = 18)
    let range_dash_pairs = Range::parse("99-77").unwrap();
    assert_eq!(range_dash_pairs.total_weight(), 18.0);

    // Dash notation for suited: A5s-A2s (A5s, A4s, A3s, A2s = 4 * 4 = 16)
    let range_dash_suited = Range::parse("A5s-A2s").unwrap();
    assert_eq!(range_dash_suited.total_weight(), 16.0);

    // Weighted syntax
    let range_weighted = Range::parse("0.5*AA, 0.25*KK").unwrap();
    assert_eq!(range_weighted.total_weight(), 6.0 * 0.5 + 6.0 * 0.25);

    // Specific combo: AhKh
    let range_specific = Range::parse("AhKh").unwrap();
    assert_eq!(range_specific.total_weight(), 1.0);
    let ah = Card::from_str_exact("Ah").unwrap();
    let kh = Card::from_str_exact("Kh").unwrap();
    assert_eq!(range_specific.get_weight(ah, kh), 1.0);
}

#[test]
fn test_blocker_removal_combinatorics() {
    let mut range = Range::uniform();
    assert_eq!(range.total_weight(), 1326.0);

    // Hero holds Ah Kh
    let ah = Card::from_str_exact("Ah").unwrap();
    let kh = Card::from_str_exact("Kh").unwrap();
    let mut dead = DeckBitmask::empty();
    dead.add_card(ah);
    dead.add_card(kh);

    range.apply_blockers(dead);

    // Inclusion-exclusion:
    // Combos with Ah = 51
    // Combos with Kh = 51
    // Overlap AhKh = 1
    // Blocked = 51 + 51 - 1 = 101 combos
    // Active = 1326 - 101 = 1225 combos
    assert_eq!(range.total_weight(), 1225.0);

    // Verify specifically that AhKh is 0.0
    assert_eq!(range.get_weight(ah, kh), 0.0);

    // Verify that any combo with Ah is 0.0
    for s in 0..4 {
        let as_card = Card::new(Rank::Ace, Suit::ALL[s]);
        if as_card != ah {
            assert_eq!(range.get_weight(ah, as_card), 0.0);
        }
    }

    // Now add a flop: Qh Jh Th (total 5 dead cards)
    let qh = Card::from_str_exact("Qh").unwrap();
    let jh = Card::from_str_exact("Jh").unwrap();
    let th = Card::from_str_exact("Th").unwrap();
    dead.add_card(qh);
    dead.add_card(jh);
    dead.add_card(th);

    let mut range2 = Range::uniform();
    range2.apply_blockers(dead);

    // With 5 dead cards, remaining unblocked cards = 52 - 5 = 47 cards.
    // Number of unblocked 2-card combos = 47 * 46 / 2 = 1081 combos!
    assert_eq!(range2.total_weight(), 1081.0);

    // Active combo iterator check
    let active_count = range2.active_combos().count();
    assert_eq!(active_count, 1081);
}

#[test]
fn test_blocker_application_speed_benchmark() {
    let range = Range::uniform();

    let mut dead = DeckBitmask::empty();
    dead.add_card(Card::from_str_exact("Ah").unwrap());
    dead.add_card(Card::from_str_exact("Kd").unwrap());
    dead.add_card(Card::from_str_exact("Qc").unwrap());
    dead.add_card(Card::from_str_exact("Js").unwrap());
    dead.add_card(Card::from_str_exact("Th").unwrap());

    let iters = 100_000;
    let start = std::time::Instant::now();
    let mut dummy_sum = 0.0f32;

    for _ in 0..iters {
        let mut r = std::hint::black_box(range.clone());
        r.apply_blockers(std::hint::black_box(dead));
        dummy_sum += r.weights[0];
    }

    let elapsed = start.elapsed();
    let us_per_op = elapsed.as_micros() as f64 / iters as f64;

    println!(
        "\nBlocker application (with clone): {} passes in {:.2}ms ({:.3} µs/pass, dummy: {})",
        iters,
        elapsed.as_secs_f64() * 1000.0,
        us_per_op,
        dummy_sum
    );

    // Blocker application must be sub-microsecond
    #[cfg(not(debug_assertions))]
    assert!(
        us_per_op < 1.0,
        "Blocker application took {:.3} µs, expected < 1.0 µs",
        us_per_op
    );
}
