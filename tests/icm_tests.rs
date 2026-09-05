use poker_analyse::tournament::{IcmCalculator, IcmError, RiskPremiumCalculator};

#[test]
fn test_m2_1_roadmap_bubble_validation_vector() {
    // 4-handed bubble test vector from ROADMAP.md:
    // Stacks: [5000, 2500, 1500, 1000], Payouts: [0.50, 0.30, 0.20]
    let stacks = [5000.0, 2500.0, 1500.0, 1000.0];
    let payouts = [0.50, 0.30, 0.20];

    let equities = IcmCalculator::compute_equity(&stacks, &payouts)
        .expect("ICM calculation should succeed");

    assert_eq!(equities.len(), 4);

    // Sum of equities must equal sum of payouts (1.00)
    let total_equity: f64 = equities.iter().sum();
    assert!(
        (total_equity - 1.00).abs() < 1e-9,
        "Total equity {total_equity} must equal sum of payouts 1.00"
    );

    // Checking strict monotonic ordering with stack sizes
    assert!(equities[0] > equities[1]);
    assert!(equities[1] > equities[2]);
    assert!(equities[2] > equities[3]);

    println!("Computed bubble equities: {:?}", equities);

    // Exact analytical values for Malmuth-Harville ICM:
    // S0 = 5000: 0.37280
    // S1 = 2500: 0.27733
    // S2 = 1500: 0.20449
    // S3 = 1000: 0.14538
    let expected = [0.37280, 0.27733, 0.20449, 0.14538];
    for (i, (&actual, &exp)) in equities.iter().zip(expected.iter()).enumerate() {
        let diff = (actual - exp).abs();
        assert!(
            diff < 0.0005,
            "Player {i} equity {actual:.5} differs from expected {exp:.5} by {diff:.6} (target eps < 0.0005)"
        );
    }
}

#[test]
fn test_equal_stacks_symmetry() {
    let stacks = [2500.0, 2500.0, 2500.0, 2500.0];
    let payouts = [0.50, 0.30, 0.20];

    let equities = IcmCalculator::compute_equity(&stacks, &payouts).unwrap();
    let expected_eq = 1.00 / 4.0; // 0.25 each

    for (i, &eq) in equities.iter().enumerate() {
        assert!(
            (eq - expected_eq).abs() < 1e-9,
            "Player {i} equity {eq} should equal {expected_eq}"
        );
    }
}

#[test]
fn test_heads_up_exact_formula() {
    let stacks = [7000.0, 3000.0];
    let payouts = [100.0, 50.0];

    let equities = IcmCalculator::compute_equity(&stacks, &payouts).unwrap();

    // Heads up analytical:
    // P0: 50 + (100 - 50) * 0.7 = 85.0
    // P1: 50 + (100 - 50) * 0.3 = 65.0
    assert!((equities[0] - 85.0).abs() < 1e-9);
    assert!((equities[1] - 65.0).abs() < 1e-9);
}

#[test]
fn test_busted_player_zero_equity() {
    let stacks = [6000.0, 4000.0, 0.0];
    let payouts = [0.65, 0.35];

    let equities = IcmCalculator::compute_equity(&stacks, &payouts).unwrap();
    assert_eq!(equities[2], 0.0, "Player with 0 chips must have 0.0 equity");

    let total: f64 = equities.iter().sum();
    assert!((total - 1.00).abs() < 1e-9);
}

#[test]
fn test_icm_fixed_stack_buffer() {
    let stacks = [5000.0, 2500.0, 1500.0, 1000.0];
    let payouts = [0.50, 0.30, 0.20];

    let eq_fixed = IcmCalculator::compute_equity_fixed(&stacks, &payouts).unwrap();
    let eq_vec = IcmCalculator::compute_equity(&stacks, &payouts).unwrap();

    for i in 0..4 {
        assert_eq!(eq_fixed[i], eq_vec[i]);
    }
}

#[test]
fn test_icm_error_handling() {
    // Empty stacks
    assert_eq!(
        IcmCalculator::compute_equity(&[], &[1.0]),
        Err(IcmError::EmptyStacks)
    );

    // Too many players (> 10)
    let eleven_players = [100.0; 11];
    assert_eq!(
        IcmCalculator::compute_equity(&eleven_players, &[1.0]),
        Err(IcmError::TooManyPlayers(11))
    );

    // Negative stack
    assert_eq!(
        IcmCalculator::compute_equity(&[100.0, -50.0], &[1.0]),
        Err(IcmError::InvalidStack(1, -50.0))
    );

    // Zero total chips
    assert_eq!(
        IcmCalculator::compute_equity(&[0.0, 0.0], &[1.0]),
        Err(IcmError::ZeroTotalChips)
    );
}

#[test]
fn test_risk_premium_bubble_confrontation() {
    // 4-handed bubble: [5000, 2500, 1500, 1000], Payouts: [0.50, 0.30, 0.20]
    let stacks = [5000.0, 2500.0, 1500.0, 1000.0];
    let payouts = [0.50, 0.30, 0.20];

    // Player 1 (2500 chips) faces all-in from Player 0 (5000 chips)
    // Pot before call = 2500 (shove), call cost = 2500 (effective stack)
    // Pot odds = 2500 / 5000 = 50% ChipEV
    let confrontation = RiskPremiumCalculator::calculate_allin_confrontation(
        &stacks,
        &payouts,
        1, // Hero = 2500
        0, // Villain = 5000
        2500.0,
        2500.0,
    )
    .unwrap();

    assert_eq!(confrontation.pot_odds_chip_ev, 0.50);

    // On the bubble, losing 2500 chips loses ALL equity (~0.2687),
    // but winning 2500 chips only increases equity from 0.2687 to 5000-chip equity (~0.3833),
    // gain = 0.1146, loss = 0.2687.
    // Bubble Factor = loss / gain > 2.0!
    assert!(
        confrontation.bubble_factor > 2.0,
        "Bubble factor {} should be > 2.0 on the direct bubble",
        confrontation.bubble_factor
    );

    // Break-even required tournament equity must be significantly higher than 50%
    assert!(
        confrontation.required_tournament_equity > 0.65,
        "Required tournament equity {} must be > 65%",
        confrontation.required_tournament_equity
    );

    // Positive Risk Premium (e.g. > +15%)
    assert!(
        confrontation.risk_premium > 0.15,
        "Risk premium {} must be > +15%",
        confrontation.risk_premium
    );

    // Range tightener test
    let base_call_threshold = 0.50; // ChipEV
    let adjusted = RiskPremiumCalculator::adjust_equity_threshold(
        base_call_threshold,
        confrontation.risk_premium,
    );
    assert_eq!(adjusted, base_call_threshold + confrontation.risk_premium);
}

#[test]
fn test_pairwise_risk_matrix() {
    let stacks = [5000.0, 2500.0, 1500.0, 1000.0];
    let payouts = [0.50, 0.30, 0.20];

    let matrix = RiskPremiumCalculator::compute_pairwise_risk_matrix(&stacks, &payouts)
        .unwrap();

    assert_eq!(matrix.num_players, 4);

    // Diagonal must be 0.0 (cannot play against self)
    for i in 0..4 {
        assert_eq!(matrix.get(i, i), 0.0);
    }

    // Chip leader (0) vs short stack (3):
    // Chip leader risks only 1000 out of 5000 chips, so risk premium is very low
    let rp_big_vs_short = matrix.get(0, 3);
    // Medium stack (1) vs Chip leader (0):
    // Medium stack risks their entire tournament life, so risk premium is high
    let rp_med_vs_big = matrix.get(1, 0);

    assert!(
        rp_med_vs_big > rp_big_vs_short,
        "Medium stack vs Big stack RP ({rp_med_vs_big}) must be higher than Big stack vs Short stack RP ({rp_big_vs_short})"
    );
}

#[test]
fn test_icm_speed_benchmark() {
    // 9-handed final table ICM performance test
    let stacks = [
        12000.0, 9500.0, 8000.0, 6500.0, 5000.0, 4200.0, 3100.0, 2000.0, 1200.0,
    ];
    let payouts = [1000.0, 600.0, 400.0, 250.0, 180.0, 120.0, 90.0, 60.0, 40.0];

    let start = std::time::Instant::now();
    let iters = 10_000;
    let mut acc = 0.0;

    for _ in 0..iters {
        let eq = IcmCalculator::compute_equity(&stacks, &payouts).unwrap();
        acc += eq[0];
    }

    let elapsed = start.elapsed();
    let us_per_calc = elapsed.as_micros() as f64 / iters as f64;
    println!(
        "\nICM 9-handed: 10,000 evaluations in {:.2}ms ({:.2} µs/eval, acc: {:.1})",
        elapsed.as_secs_f64() * 1000.0,
        us_per_calc,
        acc
    );

    #[cfg(debug_assertions)]
    let max_allowed_us = 300.0;
    #[cfg(not(debug_assertions))]
    let max_allowed_us = 20.0;

    assert!(
        us_per_calc < max_allowed_us,
        "ICM calculation took {:.2} µs/eval, expected < {:.2} µs",
        us_per_calc,
        max_allowed_us
    );
}
