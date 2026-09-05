use poker_analyse::audit::{LeakSeverity, SessionAuditor};

const OPTIMAL_HAND: &str = r#"
PokerStars Hand #100001: Tournament #5001, $10+$1 USD Hold'em No Limit - Level I (25/50) - 2026/09/05
Table '5001 1' 9-max Seat #1 is the button
Seat 1: Villain1 (5000 in chips)
Seat 2: Villain2 (5000 in chips)
Seat 3: Hero (5000 in chips)
Villain1: posts small blind 25
Villain2: posts big blind 50
*** HOLE CARDS ***
Dealt to Hero [Ac As]
Hero: raises 100 to 150
Villain1: folds
Villain2: calls 100
*** FLOP *** [Kd 7h 2c]
Villain2: checks
Hero: bets 200
Villain2: folds
*** SUMMARY ***
Total pot 325 | Rake 0
"#;

const BLUNDER_HAND: &str = r#"
PokerStars Hand #100002: Tournament #5001, $10+$1 USD Hold'em No Limit - Level I (25/50) - 2026/09/05
Table '5001 1' 9-max Seat #1 is the button
Seat 1: Villain1 (5000 in chips)
Seat 2: Villain2 (5000 in chips)
Seat 3: Hero (5000 in chips)
Villain1: posts small blind 25
Villain2: posts big blind 50
*** HOLE CARDS ***
Dealt to Hero [Ah Kh]
Hero: raises 100 to 150
Villain1: folds
Villain2: calls 100
*** FLOP *** [Qh Jh Th]
Villain2: checks
Hero: bets 200
Villain2: calls 200
*** TURN *** [Qh Jh Th] [2s]
Villain2: checks
Hero: bets 400
Villain2: calls 400
*** RIVER *** [Qh Jh Th 2s] [3d]
Villain2: checks
Hero: folds
*** SUMMARY ***
Total pot 1525 | Rake 0
"#;

const MULTI_HAND_SESSION: &str = r#"
PokerStars Hand #200001: Tournament #9001, $20+$2 USD Hold'em No Limit - Level I (25/50) - 2026/09/05
Table '9001 1' 9-max Seat #1 is the button
Seat 1: ButtonUser (5000 in chips)
Seat 2: SmallBlindGuy (5000 in chips)
Seat 3: Hero (5000 in chips)
SmallBlindGuy: posts small blind 25
Hero: posts big blind 50
*** HOLE CARDS ***
Dealt to Hero [Kh Kd]
ButtonUser: raises 100 to 150
SmallBlindGuy: folds
Hero: raises 300 to 450
ButtonUser: calls 300
*** FLOP *** [2s 4h 9c]
Hero: bets 500
ButtonUser: folds
*** SUMMARY ***
Total pot 925 | Rake 0

PokerStars Hand #200002: Tournament #9001, $20+$2 USD Hold'em No Limit - Level I (25/50) - 2026/09/05
Table '9001 1' 9-max Seat #2 is the button
Seat 1: ButtonUser (4550 in chips)
Seat 2: SmallBlindGuy (4975 in chips)
Seat 3: Hero (5475 in chips)
Hero: posts small blind 25
ButtonUser: posts big blind 50
*** HOLE CARDS ***
Dealt to Hero [7c 2h]
Hero: folds
ButtonUser: checks
*** SUMMARY ***
Total pot 75 | Rake 0

PokerStars Hand #200003: Tournament #9001, $20+$2 USD Hold'em No Limit - Level I (25/50) - 2026/09/05
Table '9001 1' 9-max Seat #3 is the button
Seat 1: ButtonUser (4575 in chips)
Seat 2: SmallBlindGuy (4975 in chips)
Seat 3: Hero (5450 in chips)
ButtonUser: posts small blind 25
SmallBlindGuy: posts big blind 50
*** HOLE CARDS ***
Dealt to Hero [As Ks]
Hero: raises 100 to 150
ButtonUser: calls 125
SmallBlindGuy: calls 100
*** FLOP *** [Qs Js Th]
SmallBlindGuy: checks
Hero: folds
ButtonUser: checks
*** SUMMARY ***
Total pot 450 | Rake 0
"#;

#[test]
fn test_session_auditor_single_hand_optimal() {
    let report = SessionAuditor::audit_text(OPTIMAL_HAND, None)
        .expect("Failed to audit optimal hand");

    assert_eq!(report.total_hands, 1);
    assert_eq!(report.hero_hands, 1);
    assert!(report.total_decisions >= 2);
    assert_eq!(report.blunder_count, 0);
    assert_eq!(report.mistake_count, 0);
    assert!(report.total_bb_ev_loss < 1.0);
}

#[test]
fn test_session_auditor_blunder_detection() {
    let report = SessionAuditor::audit_text(BLUNDER_HAND, None)
        .expect("Failed to audit blunder hand");

    assert_eq!(report.total_hands, 1);
    assert_eq!(report.hero_hands, 1);
    assert_eq!(report.total_decisions, 4);

    // Folding the Royal Flush on river in a 1500+ chip pot is a massive blunder (>3 BB)
    assert!(report.blunder_count >= 1);
    assert!(report.total_bb_ev_loss >= 3.0);
    assert!(!report.top_leaks.is_empty());

    let worst_leak = &report.top_leaks[0];
    assert_eq!(worst_leak.hand_id, 100002);
    assert_eq!(worst_leak.severity, LeakSeverity::Blunder);
    assert!(worst_leak.reasoning.contains("Optimal"));
}

#[test]
fn test_session_auditor_multi_hand_batch() {
    let report = SessionAuditor::audit_text(MULTI_HAND_SESSION, None)
        .expect("Failed to audit multi-hand session");

    assert_eq!(report.total_hands, 3);
    assert_eq!(report.hero_hands, 3);
    assert!(report.total_decisions >= 3);

    // Hand 1 is optimal
    // Hand 2 (72o fold) is optimal
    // Hand 3 (folding flopped royal straight flush draw in a 450 pot) is a blunder
    assert!(report.blunder_count >= 1);
    assert!(!report.top_leaks.is_empty());

    // Check street breakdown
    let preflop_stats = report.street_stats.get("Preflop").expect("Missing Preflop stats");
    assert!(preflop_stats.decisions_count >= 3);

    let table_str = report.format_terminal_table();
    assert!(table_str.contains("TOURNAMENT SESSION AUDIT REPORT"));
    assert!(table_str.contains("LEAK CLASSIFICATION BREAKDOWN"));
    assert!(table_str.contains("BLUNDER"));
}

#[test]
fn test_session_audit_report_json_serialization() {
    let report = SessionAuditor::audit_text(MULTI_HAND_SESSION, None)
        .expect("Failed to audit multi-hand session");

    let json = report.to_json().expect("Failed to serialize audit report to JSON");
    assert!(json.contains("\"total_hands\": 3"));
    assert!(json.contains("\"blunder_count\":"));

    let deserialized = poker_analyse::audit::SessionAuditReport::from_json(&json)
        .expect("Failed to deserialize audit report");
    assert_eq!(deserialized.total_hands, report.total_hands);
    assert_eq!(deserialized.blunder_count, report.blunder_count);
}
