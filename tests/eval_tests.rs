use poker_analyse::{eval_5hand, eval_5hand_array, Card, HandRankCategory};

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
fn test_roadmap_validation_vectors() {
    // 1. Royal Flush == 1
    let royal_flush = parse_5("AhKhQhJhTh");
    let royal_score = eval_5hand_array(royal_flush);
    assert_eq!(royal_score, 1, "Royal Flush must evaluate to score 1");
    assert_eq!(
        HandRankCategory::from_score(royal_score),
        Some(HandRankCategory::StraightFlush)
    );

    // Also test in Spades
    let royal_spades = parse_5("AsKsQsJsTs");
    assert_eq!(eval_5hand_array(royal_spades), 1);

    // 2. 7-5-4-3-2 unsuited == 7462 (the worst hand in poker)
    let worst_hand = parse_5("7s5h4d3c2s");
    let worst_score = eval_5hand_array(worst_hand);
    assert_eq!(
        worst_score, 7462,
        "7-5-4-3-2 unsuited must evaluate to score 7462"
    );
    assert_eq!(
        HandRankCategory::from_score(worst_score),
        Some(HandRankCategory::HighCard)
    );
}

#[test]
fn test_all_hand_categories_boundaries() {
    // Straight Flush: 1 to 10
    let best_sf = parse_5("AhKhQhJhTh");
    assert_eq!(eval_5hand_array(best_sf), 1);
    let wheel_sf = parse_5("5d4d3d2dAd");
    assert_eq!(eval_5hand_array(wheel_sf), 10);
    assert_eq!(
        HandRankCategory::from_score(10),
        Some(HandRankCategory::StraightFlush)
    );

    // Four of a Kind: 11 to 166
    let best_quads = parse_5("AsAhAdAcKs");
    assert_eq!(eval_5hand_array(best_quads), 11);
    assert_eq!(
        HandRankCategory::from_score(11),
        Some(HandRankCategory::FourOfAKind)
    );
    let worst_quads = parse_5("2s2h2d2c3s");
    assert_eq!(eval_5hand_array(worst_quads), 166);
    assert_eq!(
        HandRankCategory::from_score(166),
        Some(HandRankCategory::FourOfAKind)
    );

    // Full House: 167 to 322
    let best_fh = parse_5("AsAhAdKsKh");
    assert_eq!(eval_5hand_array(best_fh), 167);
    assert_eq!(
        HandRankCategory::from_score(167),
        Some(HandRankCategory::FullHouse)
    );
    let worst_fh = parse_5("2s2h2d3s3h");
    assert_eq!(eval_5hand_array(worst_fh), 322);
    assert_eq!(
        HandRankCategory::from_score(322),
        Some(HandRankCategory::FullHouse)
    );

    // Flush: 323 to 1599
    let best_flush = parse_5("AsKsQsJs9s");
    assert_eq!(eval_5hand_array(best_flush), 323);
    assert_eq!(
        HandRankCategory::from_score(323),
        Some(HandRankCategory::Flush)
    );
    let worst_flush = parse_5("7s5s4s3s2s");
    assert_eq!(eval_5hand_array(worst_flush), 1599);
    assert_eq!(
        HandRankCategory::from_score(1599),
        Some(HandRankCategory::Flush)
    );

    // Straight: 1600 to 1609
    let broadway = parse_5("AsKhQdJcTs");
    assert_eq!(eval_5hand_array(broadway), 1600);
    assert_eq!(
        HandRankCategory::from_score(1600),
        Some(HandRankCategory::Straight)
    );
    let wheel = parse_5("5s4h3d2cAs");
    assert_eq!(eval_5hand_array(wheel), 1609);
    assert_eq!(
        HandRankCategory::from_score(1609),
        Some(HandRankCategory::Straight)
    );

    // Three of a Kind: 1610 to 2467
    let best_trips = parse_5("AsAhAdKsQc");
    assert_eq!(eval_5hand_array(best_trips), 1610);
    assert_eq!(
        HandRankCategory::from_score(1610),
        Some(HandRankCategory::ThreeOfAKind)
    );
    let worst_trips = parse_5("2s2h2d4s3c");
    assert_eq!(eval_5hand_array(worst_trips), 2467);
    assert_eq!(
        HandRankCategory::from_score(2467),
        Some(HandRankCategory::ThreeOfAKind)
    );

    // Two Pair: 2468 to 3325
    let best_twopair = parse_5("AsAhKsKhQd");
    assert_eq!(eval_5hand_array(best_twopair), 2468);
    assert_eq!(
        HandRankCategory::from_score(2468),
        Some(HandRankCategory::TwoPair)
    );
    let worst_twopair = parse_5("3s3h2s2h4d");
    assert_eq!(eval_5hand_array(worst_twopair), 3325);
    assert_eq!(
        HandRankCategory::from_score(3325),
        Some(HandRankCategory::TwoPair)
    );

    // One Pair: 3326 to 6185
    let best_onepair = parse_5("AsAhKsQdJc");
    assert_eq!(eval_5hand_array(best_onepair), 3326);
    assert_eq!(
        HandRankCategory::from_score(3326),
        Some(HandRankCategory::OnePair)
    );
    let worst_onepair = parse_5("2s2h5s4d3c");
    assert_eq!(eval_5hand_array(worst_onepair), 6185);
    assert_eq!(
        HandRankCategory::from_score(6185),
        Some(HandRankCategory::OnePair)
    );

    // High Card: 6186 to 7462
    let best_highcard = parse_5("AsKhQdJc9s");
    assert_eq!(eval_5hand_array(best_highcard), 6186);
    assert_eq!(
        HandRankCategory::from_score(6186),
        Some(HandRankCategory::HighCard)
    );
    let worst_highcard = parse_5("7s5h4d3c2s");
    assert_eq!(eval_5hand_array(worst_highcard), 7462);
    assert_eq!(
        HandRankCategory::from_score(7462),
        Some(HandRankCategory::HighCard)
    );
}

#[test]
fn test_relative_hand_comparisons() {
    let royal = eval_5hand_array(parse_5("AhKhQhJhTh"));
    let quads_aces = eval_5hand_array(parse_5("AsAhAdAcKs"));
    let quads_kings = eval_5hand_array(parse_5("KsKhKdKcAs"));
    let full_house = eval_5hand_array(parse_5("AsAhAdKsKh"));
    let nut_flush = eval_5hand_array(parse_5("AsKsQsJs9s"));
    let broadway = eval_5hand_array(parse_5("AsKdQhJcTs"));
    let trips = eval_5hand_array(parse_5("AsAhAdKsQc"));
    let two_pair = eval_5hand_array(parse_5("AsAhKsKhQd"));
    let one_pair = eval_5hand_array(parse_5("AsAhKsQdJc"));
    let high_card = eval_5hand_array(parse_5("AsKhQdJc9s"));

    // Lower score is strictly better
    assert!(royal < quads_aces);
    assert!(quads_aces < quads_kings);
    assert!(quads_kings < full_house);
    assert!(full_house < nut_flush);
    assert!(nut_flush < broadway);
    assert!(broadway < trips);
    assert!(trips < two_pair);
    assert!(two_pair < one_pair);
    assert!(one_pair < high_card);
}

#[test]
fn test_direct_eval_5hand_call() {
    let c1 = Card::from_str_exact("Ah").unwrap();
    let c2 = Card::from_str_exact("Kh").unwrap();
    let c3 = Card::from_str_exact("Qh").unwrap();
    let c4 = Card::from_str_exact("Jh").unwrap();
    let c5 = Card::from_str_exact("Th").unwrap();

    let score = eval_5hand(c1, c2, c3, c4, c5);
    assert_eq!(score, 1);
}
