use poker_analyse::card::Card;
use poker_analyse::range::{
    classify_combo, HandStrengthBucket, OpponentAction, OpponentArchetype, Range, TendencyMatrix,
    ThreeTierBucket,
};

#[test]
fn test_combo_classification_board_textures() {
    let parse_card = |s| Card::from_str_exact(s).unwrap();

    // 1. Dry Flop: Kh 8c 3d
    let board_dry = [parse_card("Kh"), parse_card("8c"), parse_card("3d")];

    // Ah Kd: Top Pair Top Kicker -> Strong
    assert_eq!(
        classify_combo(parse_card("Ah"), parse_card("Kd"), &board_dry),
        HandStrengthBucket::Strong
    );

    // Kd 8d: Two Pair (Kings and Eights) -> Strong
    assert_eq!(
        classify_combo(parse_card("Kd"), parse_card("8d"), &board_dry),
        HandStrengthBucket::Strong
    );

    // 8s 7s: Middle Pair -> Marginal
    assert_eq!(
        classify_combo(parse_card("8s"), parse_card("7s"), &board_dry),
        HandStrengthBucket::Marginal
    );

    // 2h 2c: Underpair -> Marginal
    assert_eq!(
        classify_combo(parse_card("2h"), parse_card("2c"), &board_dry),
        HandStrengthBucket::Marginal
    );

    // 8d 8h: Middle Set -> Monster
    assert_eq!(
        classify_combo(parse_card("8d"), parse_card("8h"), &board_dry),
        HandStrengthBucket::Monster
    );

    // 6s 5s: Pure Air (no pair, no draw) -> Air
    assert_eq!(
        classify_combo(parse_card("6s"), parse_card("5s"), &board_dry),
        HandStrengthBucket::Air
    );

    // 2. Wet Flop: Ah Kd 2h (two hearts)
    let board_wet = [parse_card("Ah"), parse_card("Kd"), parse_card("2h")];

    // Qh Jh: Two hearts + 1 heart on board = 3 hearts (no flush draw), but Q-J with A-K forms gutshot straight draw (needs Ten) -> Draw
    assert_eq!(
        classify_combo(parse_card("Qh"), parse_card("Jh"), &board_wet),
        HandStrengthBucket::Draw
    );

    // 9h 8h: Two hearts + two hearts on board = 4 hearts -> Flush Draw
    assert_eq!(
        classify_combo(parse_card("9h"), parse_card("8h"), &board_wet),
        HandStrengthBucket::Draw
    );

    // 2s 2c: Bottom Set -> Monster
    assert_eq!(
        classify_combo(parse_card("2s"), parse_card("2c"), &board_wet),
        HandStrengthBucket::Monster
    );

    // Ac Qd: Top pair Queen kicker -> Strong
    assert_eq!(
        classify_combo(parse_card("Ac"), parse_card("Qd"), &board_wet),
        HandStrengthBucket::Strong
    );

    // Ac 3d: Top pair 3 kicker -> Marginal
    assert_eq!(
        classify_combo(parse_card("Ac"), parse_card("3d"), &board_wet),
        HandStrengthBucket::Marginal
    );
}

#[test]
fn test_three_tier_mapping() {
    assert_eq!(HandStrengthBucket::Air.to_three_tier(), ThreeTierBucket::Air);
    assert_eq!(HandStrengthBucket::Draw.to_three_tier(), ThreeTierBucket::Marginal);
    assert_eq!(HandStrengthBucket::Marginal.to_three_tier(), ThreeTierBucket::Marginal);
    assert_eq!(HandStrengthBucket::Strong.to_three_tier(), ThreeTierBucket::Strong);
    assert_eq!(HandStrengthBucket::Monster.to_three_tier(), ThreeTierBucket::Strong);
}

#[test]
fn test_nit_raise_filtering() {
    let mut range = Range::uniform();
    let board = [
        Card::from_str_exact("Ks").unwrap(),
        Card::from_str_exact("7d").unwrap(),
        Card::from_str_exact("2c").unwrap(),
    ];

    let nit_profile = TendencyMatrix::from_archetype(OpponentArchetype::Nit);

    // Before update: all weights are 1.0
    let ah = Card::from_str_exact("Ah").unwrap();
    let kd = Card::from_str_exact("Kd").unwrap(); // Top pair top kicker -> Strong
    let seven_h = Card::from_str_exact("7h").unwrap();
    let seven_c = Card::from_str_exact("7c").unwrap(); // Set of 7s -> Monster
    let q_h = Card::from_str_exact("Qh").unwrap();
    let j_h = Card::from_str_exact("Jh").unwrap(); // Air

    assert_eq!(range.get_weight(ah, kd), 1.0);
    assert_eq!(range.get_weight(seven_h, seven_c), 1.0);
    assert_eq!(range.get_weight(q_h, j_h), 1.0);

    // Apply Bayesian update for Opponent Action = Raise
    range.apply_bayesian_update(&board, OpponentAction::Raise, &nit_profile, false);

    // Nit raises:
    // P(Raise | Monster) = 0.90
    // P(Raise | Strong) = 0.50
    // P(Raise | Air) = 0.01
    assert_eq!(range.get_weight(seven_h, seven_c), 0.90);
    assert_eq!(range.get_weight(ah, kd), 0.50);
    assert_eq!(range.get_weight(q_h, j_h), 0.01);

    // Air is heavily discounted (50x less than Strong, 90x less than Monster)
    assert!(range.get_weight(seven_h, seven_c) > range.get_weight(ah, kd));
    assert!(range.get_weight(ah, kd) > range.get_weight(q_h, j_h));
}

#[test]
fn test_calling_station_call_filtering() {
    let mut range = Range::uniform();
    let board = [
        Card::from_str_exact("Ks").unwrap(),
        Card::from_str_exact("8d").unwrap(),
        Card::from_str_exact("3c").unwrap(),
    ];

    let station_profile = TendencyMatrix::from_archetype(OpponentArchetype::CallingStation);

    // Calling station calls:
    // P(Call | Marginal) = 0.85
    // P(Call | Draw) = 0.70
    // P(Call | Air) = 0.20
    range.apply_bayesian_update(&board, OpponentAction::Call, &station_profile, false);

    let eight_s = Card::from_str_exact("8s").unwrap();
    let seven_s = Card::from_str_exact("7s").unwrap(); // 8s 7s = Marginal (Middle pair)
    let q_h = Card::from_str_exact("Qh").unwrap();
    let j_h = Card::from_str_exact("Jh").unwrap(); // Air

    assert_eq!(range.get_weight(eight_s, seven_s), 0.85);
    assert_eq!(range.get_weight(q_h, j_h), 0.20);
}

#[test]
fn test_bayesian_update_speed_benchmark() {
    let range = Range::uniform();
    let board = [
        Card::from_str_exact("As").unwrap(),
        Card::from_str_exact("Kd").unwrap(),
        Card::from_str_exact("4c").unwrap(),
    ];
    let profile = TendencyMatrix::from_archetype(OpponentArchetype::Balanced);

    let iters = 10_000;
    let start = std::time::Instant::now();
    let mut dummy_weight = 0.0f32;

    for _ in 0..iters {
        let mut r = std::hint::black_box(range.clone());
        r.apply_bayesian_update(
            std::hint::black_box(&board),
            OpponentAction::Bet,
            std::hint::black_box(&profile),
            false,
        );
        dummy_weight += r.weights[0];
    }

    let elapsed = start.elapsed();
    let us_per_update = elapsed.as_micros() as f64 / iters as f64;

    println!(
        "\nBayesian update: {} updates in {:.2}ms ({:.2} µs/update, dummy: {})",
        iters,
        elapsed.as_secs_f64() * 1000.0,
        us_per_update,
        dummy_weight
    );

    // In release mode, each full 1326-combo Bayesian update must be < 200 µs
    #[cfg(not(debug_assertions))]
    assert!(
        us_per_update < 200.0,
        "Bayesian update took {:.2} µs, expected < 200.0 µs",
        us_per_update
    );
}
