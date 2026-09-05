use poker_analyse::card::Card;
use poker_analyse::deck::DeckBitmask;
use poker_analyse::range::Range;
use poker_analyse::sim::{CandidateAction, EvContext, EvEngine, MonteCarloSimulator};

#[test]
fn test_monte_carlo_multiway_simulator_speed() {
    let parse = |s| Card::from_str_exact(s).unwrap();
    let hero = [parse("Ah"), parse("Kh")];
    let board = [parse("Qh"), parse("Jh"), parse("2c")];

    let villain1_range = Range::parse("TT+, AJs+, KQs, QJs").unwrap();
    let villain2_range = Range::parse("22+, A2s+, KTs+, QTs+, JTs").unwrap();

    let samples = 10_000;
    let res = MonteCarloSimulator::simulate_equity(
        hero,
        &board,
        &[villain1_range, villain2_range],
        DeckBitmask::empty(),
        samples,
        Some(42),
    );

    println!(
        "\nMonte Carlo 3-way (10,000 samples): Equity = {:.2}%, Win = {:.2}%, Tie = {:.2}% (elapsed: {:.2}ms)",
        res.equity * 100.0,
        res.win_rate * 100.0,
        res.tie_rate * 100.0,
        res.elapsed_micros as f64 / 1000.0
    );

    // Hero has flush draw + gutshot straight draw with overcards
    assert!(res.equity > 0.35 && res.equity < 0.85);

    // Speed target from Roadmap: 10,000 samples in < 20ms in release mode
    #[cfg(not(debug_assertions))]
    assert!(
        res.elapsed_micros < 20_000,
        "Simulation took {} µs, expected < 20,000 µs (<20ms)",
        res.elapsed_micros
    );
}

#[test]
fn test_ev_engine_decision_report_and_json() {
    let parse = |s| Card::from_str_exact(s).unwrap();
    let hero = [parse("As"), parse("Kd")];
    let board = vec![parse("Kh"), parse("7d"), parse("2c")];

    let ctx = EvContext {
        street: "Flop".to_string(),
        current_pot: 100.0,
        call_cost: 0.0, // Check or Bet scenario
        hero_stack: 450.0,
        hero_cards: hero,
        board,
        hero_equity: 0.78,             // Top pair top kicker
        hero_equity_when_called: 0.65, // Against calling range
        opp_fold_freq: 0.40,
        risk_premium: 0.02,
    };

    let report = EvEngine::evaluate_decision(&ctx);

    assert_eq!(report.street, "Flop");
    assert_eq!(report.current_pot, 100.0);
    assert_eq!(report.call_cost, 0.0);

    // Check that Bet 75% or Bet 33% is recommended over Check or Fold
    assert!(matches!(
        report.recommended_action,
        CandidateAction::Bet(_) | CandidateAction::AllIn(_)
    ));

    // Verify JSON serialization works seamlessly
    let json_str = report.to_json().expect("DecisionReport must serialize to JSON");
    println!("\nSample JSON Decision Report:\n{}", json_str);

    assert!(json_str.contains("\"street\": \"Flop\""));
    assert!(json_str.contains("\"recommended_action\""));
    assert!(json_str.contains("\"action_evaluations\""));
}
