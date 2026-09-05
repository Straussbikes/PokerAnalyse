use poker_analyse::ingestion::{
    MockReplayAdapter, ReplayActionStep, ReplayHandScenario, ReplayPlayer,
};
use poker_analyse::orchestrator::PlayerAction;

#[test]
fn test_mock_replay_adapter_full_tournament_hand() {
    // Construct a recorded tournament hand in JSON format
    let scenario_json = r#"{
        "hand_id": 999123,
        "button_idx": 0,
        "sb_amount": 50.0,
        "bb_amount": 100.0,
        "players": [
            { "name": "Hero", "stack": 2500.0, "hole_cards": ["Ah", "Kh"] },
            { "name": "Villain1", "stack": 1800.0, "hole_cards": ["Qs", "Qc"] },
            { "name": "Villain2", "stack": 1200.0, "hole_cards": null }
        ],
        "board_cards": ["Ac", "7d", "2h", "5s", "9c"],
        "action_script": [
            { "street": "Preflop", "player_idx": 0, "action": { "type": "Raise", "amount": 250.0 } },
            { "street": "Preflop", "player_idx": 1, "action": { "type": "Call", "amount": 200.0 } },
            { "street": "Preflop", "player_idx": 2, "action": { "type": "Fold" } },
            { "street": "Flop", "player_idx": 1, "action": { "type": "Check" } },
            { "street": "Flop", "player_idx": 0, "action": { "type": "Bet", "amount": 200.0 } },
            { "street": "Flop", "player_idx": 1, "action": { "type": "Call", "amount": 200.0 } },
            { "street": "Turn", "player_idx": 1, "action": { "type": "Check" } },
            { "street": "Turn", "player_idx": 0, "action": { "type": "Check" } },
            { "street": "River", "player_idx": 1, "action": { "type": "Check" } },
            { "street": "River", "player_idx": 0, "action": { "type": "Check" } }
        ],
        "payouts": [500.0, 300.0, 200.0],
        "hero_idx": 0
    }"#;

    let scenario = ReplayHandScenario::from_json(scenario_json)
        .expect("Scenario JSON must deserialize successfully");

    assert_eq!(scenario.hand_id, 999123);
    assert_eq!(scenario.players.len(), 3);
    assert_eq!(scenario.action_script.len(), 10);

    let result = MockReplayAdapter::replay(&scenario).expect("Replay must execute without errors");

    assert_eq!(result.hand_id, 999123);

    // Verify DecisionReports were generated at Hero's action nodes
    assert!(
        !result.decision_reports.is_empty(),
        "Hero decision reports must be generated during replay"
    );

    println!(
        "\nReplay completed: {} events emitted, {} decision reports generated",
        result.events.len(),
        result.decision_reports.len()
    );

    for (i, report) in result.decision_reports.iter().enumerate() {
        println!(
            "Decision {i} [{}] recommended action: {:?} (Hero Equity: {:.1}%)",
            report.street,
            report.recommended_action,
            report.hero_equity * 100.0
        );
    }

    // Hero had Ah Kh on Ac 7d 2h 5s 9c (Pair of Aces with King kicker)
    // Villain1 had Qs Qc (Pair of Queens)
    // Hero wins showdown and takes the pot!
    assert!(
        result.final_stacks[0] > 2500.0,
        "Hero stack {} should increase after winning the pot",
        result.final_stacks[0]
    );
    assert!(
        result.final_stacks[1] < 1800.0,
        "Villain stack {} should decrease after losing the pot",
        result.final_stacks[1]
    );
}

#[test]
fn test_mock_replay_programmatic_scenario_builder() {
    let scenario = ReplayHandScenario {
        hand_id: 101,
        button_idx: 0,
        sb_amount: 10.0,
        bb_amount: 20.0,
        players: vec![
            ReplayPlayer {
                name: "Hero".to_string(),
                stack: 500.0,
                hole_cards: Some(["Kh".to_string(), "Kd".to_string()]),
            },
            ReplayPlayer {
                name: "Opponent".to_string(),
                stack: 300.0,
                hole_cards: Some(["Jh".to_string(), "Jc".to_string()]),
            },
        ],
        board_cards: vec![
            "2c".to_string(),
            "4d".to_string(),
            "8s".to_string(),
            "9h".to_string(),
            "Th".to_string(),
        ],
        action_script: vec![
            ReplayActionStep {
                street: "Preflop".to_string(),
                player_idx: 0,
                action: PlayerAction::Call(10.0),
            },
            ReplayActionStep {
                street: "Preflop".to_string(),
                player_idx: 1,
                action: PlayerAction::Check,
            },
            ReplayActionStep {
                street: "Flop".to_string(),
                player_idx: 1,
                action: PlayerAction::Check,
            },
            ReplayActionStep {
                street: "Flop".to_string(),
                player_idx: 0,
                action: PlayerAction::Check,
            },
            ReplayActionStep {
                street: "Turn".to_string(),
                player_idx: 1,
                action: PlayerAction::Check,
            },
            ReplayActionStep {
                street: "Turn".to_string(),
                player_idx: 0,
                action: PlayerAction::Check,
            },
            ReplayActionStep {
                street: "River".to_string(),
                player_idx: 1,
                action: PlayerAction::Check,
            },
            ReplayActionStep {
                street: "River".to_string(),
                player_idx: 0,
                action: PlayerAction::Check,
            },
        ],
        payouts: None,
        hero_idx: Some(0),
    };

    let result = MockReplayAdapter::replay(&scenario).unwrap();
    assert_eq!(result.final_stacks[0], 520.0);
    assert_eq!(result.final_stacks[1], 280.0);
}
