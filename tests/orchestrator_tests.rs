use poker_analyse::card::Card;
use poker_analyse::orchestrator::{
    HandStateMachine, Player, PlayerAction, SidePotCalculator, Street,
};

#[test]
fn test_multiway_side_pot_isolation() {
    // 3 players all-in with unequal stacks:
    // Player 0: 100 chips
    // Player 1: 250 chips
    // Player 2: 500 chips
    let contributions = [100.0, 250.0, 500.0];
    let is_active = [true, true, true];

    let result = SidePotCalculator::calculate_pots(&contributions, &is_active);

    // Main Pot: 100 from each player = 300, eligible: [0, 1, 2]
    assert_eq!(result.main_pot.amount, 300.0);
    assert_eq!(result.main_pot.eligible_players, vec![0, 1, 2]);

    // Side Pot 1: 150 from Player 1 and Player 2 = 300, eligible: [1, 2]
    assert_eq!(result.side_pots.len(), 1);
    assert_eq!(result.side_pots[0].amount, 300.0);
    assert_eq!(result.side_pots[0].eligible_players, vec![1, 2]);

    // Uncalled chips returned to Player 2: 500 - 250 = 250 chips
    assert_eq!(result.uncalled_chips.len(), 1);
    assert_eq!(result.uncalled_chips[0], (2, 250.0));

    // Total collected equals sum of contributions
    assert_eq!(result.total_collected, 850.0);
}

#[test]
fn test_hand_state_machine_uncontested_fold() {
    let players = vec![
        Player::new(0, "Hero", 1000.0),
        Player::new(1, "Villain1", 1000.0),
        Player::new(2, "Villain2", 1000.0),
    ];

    let mut sm = HandStateMachine::new(1, players, 0, 10.0, 20.0).unwrap();

    // In 3-player game with button 0:
    // SB = Player 1 (posts 10)
    // BB = Player 2 (posts 20)
    // UTG = Player 0 (Hero)
    assert_eq!(sm.action_idx, 0);
    assert_eq!(sm.pot_total(), 30.0);

    // Hero raises to 60
    sm.apply_action(0, PlayerAction::Raise(60.0)).unwrap();
    assert_eq!(sm.current_bet, 60.0);

    // SB folds
    assert_eq!(sm.action_idx, 1);
    sm.apply_action(1, PlayerAction::Fold).unwrap();

    // BB folds
    assert_eq!(sm.action_idx, 2);
    sm.apply_action(2, PlayerAction::Fold).unwrap();

    // Hand ends uncontested! Pot awarded to Hero (Player 0)
    assert_eq!(sm.street, Street::HandEnded);
    // Hero started with 1000, committed 60, won total pot of 90 (60 + 10 + 20)
    // Net gain = +30 (SB 10 + BB 20) -> Final stack = 1030.0
    assert_eq!(sm.players[0].stack, 1030.0);
    assert_eq!(sm.players[1].stack, 990.0);
    assert_eq!(sm.players[2].stack, 980.0);
}

#[test]
fn test_hand_state_machine_showdown_with_best_hand() {
    let parse = |s| Card::from_str_exact(s).unwrap();

    let players = vec![
        Player::new(0, "Hero", 500.0),
        Player::new(1, "Villain", 500.0),
    ];

    let mut sm = HandStateMachine::new(2, players, 0, 10.0, 20.0).unwrap();

    // Heads up: Button is Player 0 (SB, posts 10), Player 1 is BB (posts 20)
    // Hero hole cards: Ah Kh (Nut Flush on heart runout)
    sm.set_hole_cards(0, [parse("Ah"), parse("Kh")]).unwrap();
    // Villain hole cards: Qs Qd (Pocket Queens)
    sm.set_hole_cards(1, [parse("Qs"), parse("Qd")]).unwrap();

    // Preflop: Hero calls 10 more (to match 20)
    assert_eq!(sm.action_idx, 0);
    sm.apply_action(0, PlayerAction::Call(10.0)).unwrap();

    // Villain checks
    assert_eq!(sm.action_idx, 1);
    sm.apply_action(1, PlayerAction::Check).unwrap();

    // Street advanced to Flop!
    assert_eq!(sm.street, Street::Flop);
    assert_eq!(sm.pot_total(), 40.0);

    // Deal Flop: Qh Jh 2h (Villain has set of Queens, Hero has Nut Flush!)
    sm.deal_board_cards(&[parse("Qh"), parse("Jh"), parse("2h")]);

    // Flop action: First to act is Villain (BB)
    assert_eq!(sm.action_idx, 1);
    sm.apply_action(1, PlayerAction::Check).unwrap();

    // Hero bets 30
    assert_eq!(sm.action_idx, 0);
    sm.apply_action(0, PlayerAction::Bet(30.0)).unwrap();

    // Villain calls 30
    assert_eq!(sm.action_idx, 1);
    sm.apply_action(1, PlayerAction::Call(30.0)).unwrap();

    // Street advanced to Turn!
    assert_eq!(sm.street, Street::Turn);
    assert_eq!(sm.pot_total(), 100.0);

    // Deal Turn: 5c
    sm.deal_board_cards(&[parse("5c")]);

    // Turn: Check - Check
    sm.apply_action(1, PlayerAction::Check).unwrap();
    sm.apply_action(0, PlayerAction::Check).unwrap();

    // Street advanced to River!
    assert_eq!(sm.street, Street::River);

    // Deal River: 9d
    sm.deal_board_cards(&[parse("9d")]);

    // River: Check - Check -> Showdown
    sm.apply_action(1, PlayerAction::Check).unwrap();
    sm.apply_action(0, PlayerAction::Check).unwrap();

    // Hand finished at Showdown!
    assert_eq!(sm.street, Street::HandEnded);

    // Hero Nut Flush beats Villain Set of Queens -> Hero awarded entire 100 pot!
    // Hero committed 50, Villain committed 50. Hero final stack = 500 - 50 + 100 = 550.0
    assert_eq!(sm.players[0].stack, 550.0);
    assert_eq!(sm.players[1].stack, 450.0);
}
