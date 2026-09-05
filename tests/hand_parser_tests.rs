use poker_analyse::api_bridge::hand_parser::HandHistoryParser;
use poker_analyse::card::Card;
use poker_analyse::orchestrator::event_bus::PlayerAction;

const SAMPLE_POKERSTARS_HAND: &str = r#"
PokerStars Hand #24019283012: Tournament #552019382, $20+$2 USD Hold'em No Limit - Level II (15/30) - 2026/09/05 18:00:00 ET
Table '552019382 3' 9-max Seat #1 is the button
Seat 1: ButtonGuy (1500 in chips)
Seat 2: SmallBlindUser (1450 in chips)
Seat 3: BigBlindUser (1600 in chips)
Seat 4: UnderTheGun (2000 in chips)
Seat 5: Hero (2500 in chips)
SmallBlindUser: posts small blind 15
BigBlindUser: posts big blind 30
*** HOLE CARDS ***
Dealt to Hero [Ah Kd]
UnderTheGun: folds
Hero: raises 60 to 90
ButtonGuy: folds
SmallBlindUser: folds
BigBlindUser: calls 60
*** FLOP *** [As 7c 2d]
BigBlindUser: checks
Hero: bets 120
BigBlindUser: calls 120
*** TURN *** [As 7c 2d] [Qh]
BigBlindUser: checks
Hero: checks
*** RIVER *** [As 7c 2d Qh] [6s]
BigBlindUser: bets 200
Hero: calls 200
*** SHOW DOWN ***
BigBlindUser: shows [Ac Ts] (a pair of Aces)
Hero: shows [Ah Kd] (a pair of Aces - King kicker)
Hero collected 855 from pot
*** SUMMARY ***
Total pot 855 | Rake 0
Board [As 7c 2d Qh 6s]
Seat 5: Hero showed [Ah Kd] and won (855) with a pair of Aces
"#;

#[test]
fn test_parse_pokerstars_tournament_hand() {
    let parsed = HandHistoryParser::parse(SAMPLE_POKERSTARS_HAND)
        .expect("Failed to parse PokerStars hand history");

    assert_eq!(parsed.hand_id, 24019283012);
    assert_eq!(parsed.tournament_id, Some(552019382));
    assert_eq!(parsed.sb_amount, 15.0);
    assert_eq!(parsed.bb_amount, 30.0);
    assert_eq!(parsed.button_seat, 1);
    assert_eq!(parsed.hero_name.as_deref(), Some("Hero"));

    let ah = Card::from_str_exact("Ah").unwrap();
    let kd = Card::from_str_exact("Kd").unwrap();
    assert_eq!(parsed.hero_cards, Some([ah, kd]));

    assert_eq!(parsed.players.len(), 5);
    assert_eq!(parsed.players[0].name, "ButtonGuy");
    assert_eq!(parsed.players[4].name, "Hero");
    assert_eq!(parsed.players[4].stack, 2500.0);
    assert_eq!(parsed.players[4].hole_cards, Some(["Ah".to_string(), "Kd".to_string()]));

    // Check board cards
    assert_eq!(parsed.board_cards.len(), 5);
    assert_eq!(parsed.board_cards[0], Card::from_str_exact("As").unwrap());
    assert_eq!(parsed.board_cards[1], Card::from_str_exact("7c").unwrap());
    assert_eq!(parsed.board_cards[2], Card::from_str_exact("2d").unwrap());
    assert_eq!(parsed.board_cards[3], Card::from_str_exact("Qh").unwrap());
    assert_eq!(parsed.board_cards[4], Card::from_str_exact("6s").unwrap());

    // Check action steps parsed
    assert!(!parsed.actions.is_empty());
    // Hero preflop raise to 90
    let hero_preflop_raise = parsed.actions.iter().find(|a| a.player_idx == 4 && a.street == "Preflop");
    assert!(hero_preflop_raise.is_some());
    assert_eq!(hero_preflop_raise.unwrap().action, PlayerAction::Raise(90.0));

    // Convert to scenario
    let scenario = parsed.to_replay_scenario(Some(vec![100.0, 60.0, 40.0]));
    assert_eq!(scenario.players.len(), 5);
    assert_eq!(scenario.hero_idx, Some(4));
    assert_eq!(scenario.board_cards.len(), 5);
}
