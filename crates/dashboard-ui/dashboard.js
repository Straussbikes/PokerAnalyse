/**
 * POKER ANALYSE - MODERN REAL-TIME HUD & POST-GAME REVIEWER
 * Complete interactive logic for 13x13 Range Heatmap, EV Differentials,
 * SVG Pot Odds vs Equity Dial, WebSocket Streaming, and Session Auditor.
 */

// Rank definitions for 13x13 grid
const RANKS = ['A', 'K', 'Q', 'J', 'T', '9', '8', '7', '6', '5', '4', '3', '2'];

// State
let ws = null;
let currentRangeMode = 'hero'; // 'hero' or 'villain'
let currentActiveFilter = 'all';

// Playback & Interactive Evaluation State
let currentDecisionIndex = 0;
let isAutoPlay = false; // MANUAL BY DEFAULT!
let autoPlayTimer = null;

// Comprehensive 16-Decision Playlist across 4 Tournament Hands
const DECISION_PLAYLIST = [
  // --- MÃO 1: #7001001 (Pocket Aces Ac As) ---
  {
    hand_id: 7001001,
    street: "Preflop",
    current_pot: 75.0,
    call_cost: 50.0,
    hero_stack: 5000.0,
    hero_cards: ["Ac", "As"],
    board_cards: [],
    hero_equity: 0.848,
    pot_odds: 0.40,
    action_evaluations: [
      { action: { type: "Fold" }, chip_ev: 0.0, break_even_equity: 0.0, description: "Surrender hand without committing chips" },
      { action: { type: "Call", amount: 50.0 }, chip_ev: 85.4, break_even_equity: 0.40, description: "Match BB 50.0 chips" },
      { action: { type: "Raise", amount: 150.0 }, chip_ev: 143.8, break_even_equity: 0.50, description: "Open raise 3x BB to 150.0 chips" },
      { action: { type: "AllIn", amount: 5000.0 }, chip_ev: 72.0, break_even_equity: 0.85, description: "Overbet shove remaining stack" }
    ],
    recommended_action: { type: "Raise", amount: 150.0 },
    reasoning: "Premium Pocket Aces. Raise 3x BB to build the pot and isolate villain range (Hero Equity: 84.8%)."
  },
  {
    hand_id: 7001001,
    street: "Flop",
    current_pot: 325.0,
    call_cost: 0.0,
    hero_stack: 4850.0,
    hero_cards: ["Ac", "As"],
    board_cards: ["Kd", "7h", "2c"],
    hero_equity: 0.874,
    pot_odds: 0.0,
    action_evaluations: [
      { action: { type: "Check" }, chip_ev: 195.8, break_even_equity: 0.0, description: "Pass action to opponent" },
      { action: { type: "Bet", amount: 107.0 }, chip_ev: 219.4, break_even_equity: 0.24, description: "Bet 33% Pot" },
      { action: { type: "Bet", amount: 243.8 }, chip_ev: 243.8, break_even_equity: 0.42, description: "Bet 75% Pot (Value / Protection)" },
      { action: { type: "AllIn", amount: 4850.0 }, chip_ev: -121.8, break_even_equity: 0.80, description: "Massive overbet push" }
    ],
    recommended_action: { type: "Bet", amount: 243.8 },
    reasoning: "Dry K-high texture with overpair. Bet 75% pot (+243.8 chips EV) targeting Kx, 7x and middle pocket pairs."
  },
  {
    hand_id: 7001001,
    street: "Turn",
    current_pot: 812.5,
    call_cost: 0.0,
    hero_stack: 4606.2,
    hero_cards: ["Ac", "As"],
    board_cards: ["Kd", "7h", "2c", "Js"],
    hero_equity: 0.825,
    pot_odds: 0.0,
    action_evaluations: [
      { action: { type: "Check" }, chip_ev: 412.0, break_even_equity: 0.0, description: "Pot control check" },
      { action: { type: "Bet", amount: 270.0 }, chip_ev: 485.5, break_even_equity: 0.25, description: "Bet 33% Pot" },
      { action: { type: "Bet", amount: 543.8 }, chip_ev: 543.8, break_even_equity: 0.40, description: "Bet 67% Pot (Second Barrel)" },
      { action: { type: "AllIn", amount: 4606.2 }, chip_ev: 210.0, break_even_equity: 0.75, description: "All-In shove" }
    ],
    recommended_action: { type: "Bet", amount: 543.8 },
    reasoning: "Second barrel for value. Safe Jack turn card; continue charging dominated pairs and gutshots."
  },
  {
    hand_id: 7001001,
    street: "River",
    current_pot: 1900.0,
    call_cost: 0.0,
    hero_stack: 4062.4,
    hero_cards: ["Ac", "As"],
    board_cards: ["Kd", "7h", "2c", "Js", "4d"],
    hero_equity: 0.880,
    pot_odds: 0.0,
    action_evaluations: [
      { action: { type: "Check" }, chip_ev: 850.0, break_even_equity: 0.0, description: "Check showdown" },
      { action: { type: "Bet", amount: 627.0 }, chip_ev: 1050.0, break_even_equity: 0.25, description: "Bet 33% Pot" },
      { action: { type: "Bet", amount: 1218.8 }, chip_ev: 1218.8, break_even_equity: 0.39, description: "Triple Barrel 64% Pot" },
      { action: { type: "AllIn", amount: 4062.4 }, chip_ev: 890.0, break_even_equity: 0.68, description: "All-in river shove" }
    ],
    recommended_action: { type: "Bet", amount: 1218.8 },
    reasoning: "Complete blank 4d on river. Triple barrel 64% pot extracting maximum value from Villain calling station range."
  },

  // --- MÃO 2: #7001002 (Royal Flush / Nut Straight Ah Kh) ---
  {
    hand_id: 7001002,
    street: "Preflop",
    current_pot: 75.0,
    call_cost: 50.0,
    hero_stack: 5500.0,
    hero_cards: ["Ah", "Kh"],
    board_cards: [],
    hero_equity: 0.672,
    pot_odds: 0.40,
    action_evaluations: [
      { action: { type: "Fold" }, chip_ev: 0.0, break_even_equity: 0.0, description: "Surrender" },
      { action: { type: "Call", amount: 50.0 }, chip_ev: 72.0, break_even_equity: 0.40, description: "Flat Call 50.0" },
      { action: { type: "Raise", amount: 150.0 }, chip_ev: 143.8, break_even_equity: 0.45, description: "Raise 3x BB to 150.0" },
      { action: { type: "AllIn", amount: 5500.0 }, chip_ev: 45.0, break_even_equity: 0.85, description: "All-in shove" }
    ],
    recommended_action: { type: "Raise", amount: 150.0 },
    reasoning: "Premium Big Slick Suited. Open raise 3x BB to capture initiative and thin opponent field."
  },
  {
    hand_id: 7001002,
    street: "Flop",
    current_pot: 325.0,
    call_cost: 0.0,
    hero_stack: 5350.0,
    hero_cards: ["Ah", "Kh"],
    board_cards: ["Qh", "Jh", "Th"],
    hero_equity: 1.0,
    pot_odds: 0.0,
    action_evaluations: [
      { action: { type: "Check" }, chip_ev: 185.0, break_even_equity: 0.0, description: "Slowplay check" },
      { action: { type: "Bet", amount: 107.0 }, chip_ev: 215.0, break_even_equity: 0.24, description: "Bet 33% Pot" },
      { action: { type: "Bet", amount: 243.8 }, chip_ev: 243.8, break_even_equity: 0.42, description: "Bet 75% Pot (Build Pot)" },
      { action: { type: "AllIn", amount: 5350.0 }, chip_ev: 90.0, break_even_equity: 0.90, description: "Overbet shove" }
    ],
    recommended_action: { type: "Bet", amount: 243.8 },
    reasoning: "FLOPPED ROYAL FLUSH! Absolute nuts with 100% equity. Bet 75% pot to start building a massive pot against flushes, sets, and straights."
  },
  {
    hand_id: 7001002,
    street: "Turn",
    current_pot: 775.0,
    call_cost: 0.0,
    hero_stack: 5125.0,
    hero_cards: ["Ah", "Kh"],
    board_cards: ["Qh", "Jh", "Th", "2s"],
    hero_equity: 1.0,
    pot_odds: 0.0,
    action_evaluations: [
      { action: { type: "Check" }, chip_ev: 390.0, break_even_equity: 0.0, description: "Deceptive check" },
      { action: { type: "Bet", amount: 255.0 }, chip_ev: 480.0, break_even_equity: 0.25, description: "Bet 33% Pot" },
      { action: { type: "Bet", amount: 581.3 }, chip_ev: 581.3, break_even_equity: 0.42, description: "Bet 75% Pot" },
      { action: { type: "AllIn", amount: 5125.0 }, chip_ev: 260.0, break_even_equity: 0.85, description: "All-in shove" }
    ],
    recommended_action: { type: "Bet", amount: 581.3 },
    reasoning: "Blank 2s turn card. Maintain aggressive sizing (75% pot) to target Villain's flushes and sets."
  },
  {
    hand_id: 7001002,
    street: "River",
    current_pot: 1937.5,
    call_cost: 0.0,
    hero_stack: 4543.7,
    hero_cards: ["Ah", "Kh"],
    board_cards: ["Qh", "Jh", "Th", "2s", "3d"],
    hero_equity: 1.0,
    pot_odds: 0.0,
    action_evaluations: [
      { action: { type: "Check" }, chip_ev: 720.0, break_even_equity: 0.0, description: "Check showdown" },
      { action: { type: "Bet", amount: 640.0 }, chip_ev: 1100.0, break_even_equity: 0.25, description: "Bet 33% Pot" },
      { action: { type: "Bet", amount: 1406.3 }, chip_ev: 1406.3, break_even_equity: 0.42, description: "Bet 72% Pot (Max Value)" },
      { action: { type: "AllIn", amount: 4543.7 }, chip_ev: 950.0, break_even_equity: 0.70, description: "Push remaining stack" }
    ],
    recommended_action: { type: "Bet", amount: 1406.3 },
    reasoning: "Optimal river value bet (+1,406.3 chips EV) with unbeatable Royal Flush. Opponent fold frequency is low on this texture."
  },

  // --- MÃO 3: #7001003 (Flopped Set of Kings Kh Kd) ---
  {
    hand_id: 7001003,
    street: "Preflop",
    current_pot: 150.0,
    call_cost: 100.0,
    hero_stack: 5200.0,
    hero_cards: ["Kh", "Kd"],
    board_cards: [],
    hero_equity: 0.825,
    pot_odds: 0.40,
    action_evaluations: [
      { action: { type: "Fold" }, chip_ev: 0.0, break_even_equity: 0.0, description: "Surrender" },
      { action: { type: "Call", amount: 100.0 }, chip_ev: 120.0, break_even_equity: 0.40, description: "Call 100.0" },
      { action: { type: "Raise", amount: 287.5 }, chip_ev: 287.5, break_even_equity: 0.48, description: "Raise 3-Bet to 287.5" },
      { action: { type: "AllIn", amount: 5200.0 }, chip_ev: 150.0, break_even_equity: 0.85, description: "Shove stack" }
    ],
    recommended_action: { type: "Raise", amount: 287.5 },
    reasoning: "Monster Pocket Kings. Premium 3-bet sizing to punish wide button and blinds ranges."
  },
  {
    hand_id: 7001003,
    street: "Flop",
    current_pot: 550.0,
    call_cost: 0.0,
    hero_stack: 4912.5,
    hero_cards: ["Kh", "Kd"],
    board_cards: ["Ks", "9c", "4d"],
    hero_equity: 0.950,
    pot_odds: 0.0,
    action_evaluations: [
      { action: { type: "Check" }, chip_ev: 280.0, break_even_equity: 0.0, description: "Check" },
      { action: { type: "Bet", amount: 180.0 }, chip_ev: 350.0, break_even_equity: 0.25, description: "Bet 33% Pot" },
      { action: { type: "Bet", amount: 412.5 }, chip_ev: 412.5, break_even_equity: 0.42, description: "Bet 75% Pot (Value Bet)" },
      { action: { type: "AllIn", amount: 4912.5 }, chip_ev: -50.0, break_even_equity: 0.85, description: "Huge overbet shove" }
    ],
    recommended_action: { type: "Bet", amount: 412.5 },
    reasoning: "Top Set of Kings on dry rainbow texture. Fast-play 75% pot (+412.5 chips EV) against top pairs and gutshots."
  },
  {
    hand_id: 7001003,
    street: "Turn",
    current_pot: 1375.0,
    call_cost: 0.0,
    hero_stack: 4500.0,
    hero_cards: ["Kh", "Kd"],
    board_cards: ["Ks", "9c", "4d", "2h"],
    hero_equity: 0.965,
    pot_odds: 0.0,
    action_evaluations: [
      { action: { type: "Check" }, chip_ev: 650.0, break_even_equity: 0.0, description: "Check" },
      { action: { type: "Bet", amount: 450.0 }, chip_ev: 780.0, break_even_equity: 0.25, description: "Bet 33% Pot" },
      { action: { type: "Bet", amount: 937.5 }, chip_ev: 937.5, break_even_equity: 0.40, description: "Bet 68% Pot (Turn Barrel)" },
      { action: { type: "AllIn", amount: 4500.0 }, chip_ev: 600.0, break_even_equity: 0.75, description: "Overbet shove" }
    ],
    recommended_action: { type: "Bet", amount: 937.5 },
    reasoning: "Completely brick 2h turn. Continue charging dominated Kx and pocket pair holdings."
  },
  {
    hand_id: 7001003,
    street: "River",
    current_pot: 3250.0,
    call_cost: 0.0,
    hero_stack: 3562.5,
    hero_cards: ["Kh", "Kd"],
    board_cards: ["Ks", "9c", "4d", "2h", "Jc"],
    hero_equity: 0.940,
    pot_odds: 0.0,
    action_evaluations: [
      { action: { type: "Check" }, chip_ev: 1400.0, break_even_equity: 0.0, description: "Check" },
      { action: { type: "Bet", amount: 1200.0 }, chip_ev: 2200.0, break_even_equity: 0.27, description: "Bet 37% Pot" },
      { action: { type: "Bet", amount: 2200.0 }, chip_ev: 2850.0, break_even_equity: 0.40, description: "Bet 67% Pot" },
      { action: { type: "AllIn", amount: 3562.5 }, chip_ev: 3562.5, break_even_equity: 0.52, description: "Commit Remaining Stack (Optimal Shove)" }
    ],
    recommended_action: { type: "AllIn", amount: 3562.5 },
    reasoning: "Jack connects two pair holdings like KJ and QJ. Push remaining stack (+3,562.5 chips EV) for maximum tournament EV."
  },

  // --- MÃO 4: #7001004 (Suited Connectors Straight 9s 8s) ---
  {
    hand_id: 7001004,
    street: "Preflop",
    current_pot: 150.0,
    call_cost: 100.0,
    hero_stack: 6100.0,
    hero_cards: ["9s", "8s"],
    board_cards: [],
    hero_equity: 0.420,
    pot_odds: 0.40,
    action_evaluations: [
      { action: { type: "Fold" }, chip_ev: 0.0, break_even_equity: 0.0, description: "Surrender" },
      { action: { type: "Call", amount: 100.0 }, chip_ev: 45.0, break_even_equity: 0.40, description: "Flat Call 100.0" },
      { action: { type: "Raise", amount: 225.0 }, chip_ev: 225.0, break_even_equity: 0.43, description: "Open raise 2.25x BB" },
      { action: { type: "AllIn", amount: 6100.0 }, chip_ev: -250.0, break_even_equity: 0.90, description: "Reckless shove" }
    ],
    recommended_action: { type: "Raise", amount: 225.0 },
    reasoning: "Prime speculative suited connector in late position. Raise to capitalize on postflop playability."
  },
  {
    hand_id: 7001004,
    street: "Flop",
    current_pot: 500.0,
    call_cost: 0.0,
    hero_stack: 5875.0,
    hero_cards: ["9s", "8s"],
    board_cards: ["7s", "6c", "2d"],
    hero_equity: 0.615,
    pot_odds: 0.0,
    action_evaluations: [
      { action: { type: "Check" }, chip_ev: 210.0, break_even_equity: 0.0, description: "Check draw" },
      { action: { type: "Bet", amount: 165.0 }, chip_ev: 290.0, break_even_equity: 0.25, description: "Bet 33% Pot" },
      { action: { type: "Bet", amount: 375.0 }, chip_ev: 375.0, break_even_equity: 0.42, description: "Bet 75% Pot (Semi-Bluff)" },
      { action: { type: "AllIn", amount: 5875.0 }, chip_ev: 80.0, break_even_equity: 0.85, description: "All-in semi-bluff shove" }
    ],
    recommended_action: { type: "Bet", amount: 375.0 },
    reasoning: "Open-ended straight draw with backdoor flush possibilities. Semi-bluff bet generating fold equity."
  },
  {
    hand_id: 7001004,
    street: "Turn",
    current_pot: 1050.0,
    call_cost: 0.0,
    hero_stack: 5500.0,
    hero_cards: ["9s", "8s"],
    board_cards: ["7s", "6c", "2d", "5h"],
    hero_equity: 0.910,
    pot_odds: 0.0,
    action_evaluations: [
      { action: { type: "Check" }, chip_ev: 480.0, break_even_equity: 0.0, description: "Check nuts" },
      { action: { type: "Bet", amount: 345.0 }, chip_ev: 610.0, break_even_equity: 0.25, description: "Bet 33% Pot" },
      { action: { type: "Bet", amount: 787.5 }, chip_ev: 787.5, break_even_equity: 0.42, description: "Bet 75% Pot (Turn Value)" },
      { action: { type: "AllIn", amount: 5500.0 }, chip_ev: 350.0, break_even_equity: 0.80, description: "Overbet shove" }
    ],
    recommended_action: { type: "Bet", amount: 787.5 },
    reasoning: "TURNED NUT 9-HIGH STRAIGHT! Sizing 75% pot targets 8x gutshots, sets, and two-pairs."
  },
  {
    hand_id: 7001004,
    street: "River",
    current_pot: 2625.0,
    call_cost: 0.0,
    hero_stack: 4712.5,
    hero_cards: ["9s", "8s"],
    board_cards: ["7s", "6c", "2d", "5h", "Kd"],
    hero_equity: 0.890,
    pot_odds: 0.0,
    action_evaluations: [
      { action: { type: "Check" }, chip_ev: 980.0, break_even_equity: 0.0, description: "Check" },
      { action: { type: "Bet", amount: 865.0 }, chip_ev: 1420.0, break_even_equity: 0.25, description: "Bet 33% Pot" },
      { action: { type: "Bet", amount: 1762.5 }, chip_ev: 1762.5, break_even_equity: 0.40, description: "Bet 67% Pot (Optimal Value)" },
      { action: { type: "AllIn", amount: 4712.5 }, chip_ev: 1150.0, break_even_equity: 0.64, description: "Push remaining stack" }
    ],
    recommended_action: { type: "Bet", amount: 1762.5 },
    reasoning: "Overcard King hits Villain calling range. High-frequency value bet with straight."
  }
];

let currentDecision = DECISION_PLAYLIST[0];

// Default Session Audit Data (3 hands from sample_tournament.txt)
let sessionAuditData = {
  total_hands: 3,
  hero_hands: 3,
  total_decisions: 7,
  total_chip_ev_loss: 2341.32,
  total_bb_ev_loss: 46.83,
  blunder_count: 1,
  mistake_count: 1,
  inaccuracy_count: 5,
  optimal_count: 0,
  leak_free_percentage: 0.0,
  all_audits: [
    {
      hand_id: 7001001,
      street: "Preflop",
      hero_cards: ["Ac", "As"],
      board_cards: [],
      pot_before: 75.0,
      call_cost: 50.0,
      hero_stack: 5000.0,
      action_taken: { type: "Raise", amount: 150.0 },
      recommended_action: { type: "Raise", amount: 143.75 },
      actual_chip_ev: 102.08,
      optimal_chip_ev: 142.04,
      chip_ev_loss: 39.96,
      ev_loss_bb: 0.80,
      severity: "Inaccuracy",
      reasoning: "Optimal Raise(143.75) (+142.04 chips) vs actual Raise(150.0) (+102.08 chips). Equity: 84.8%, Pot Odds: 40.0%."
    },
    {
      hand_id: 7001001,
      street: "Flop",
      hero_cards: ["Ac", "As"],
      board_cards: ["Kd", "7h", "2c"],
      pot_before: 325.0,
      call_cost: 0.0,
      hero_stack: 4850.0,
      action_taken: { type: "Bet", amount: 200.0 },
      recommended_action: { type: "Bet", amount: 243.75 },
      actual_chip_ev: 333.84,
      optimal_chip_ev: 347.66,
      chip_ev_loss: 13.81,
      ev_loss_bb: 0.28,
      severity: "Inaccuracy",
      reasoning: "Optimal Bet(243.75) (+347.66 chips) vs actual Bet(200.0) (+333.84 chips). Equity: 87.4%, Pot Odds: 0.0%."
    },
    {
      hand_id: 7001002,
      street: "Preflop",
      hero_cards: ["7h", "2c"],
      board_cards: [],
      pot_before: 75.0,
      call_cost: 0.0,
      hero_stack: 5375.0,
      action_taken: { type: "Fold" },
      recommended_action: { type: "Bet", amount: 24.75 },
      actual_chip_ev: 0.0,
      optimal_chip_ev: 37.66,
      chip_ev_loss: 37.66,
      ev_loss_bb: 0.75,
      severity: "Inaccuracy",
      reasoning: "Optimal Bet(24.75) (+37.66 chips) vs actual Fold (+0.00 chips). Equity: 35.4%, Pot Odds: 0.0%."
    },
    {
      hand_id: 7001003,
      street: "Preflop",
      hero_cards: ["Ah", "Kh"],
      board_cards: [],
      pot_before: 75.0,
      call_cost: 25.0,
      hero_stack: 5375.0,
      action_taken: { type: "Raise", amount: 150.0 },
      recommended_action: { type: "Raise", amount: 81.25 },
      actual_chip_ev: 65.56,
      optimal_chip_ev: 79.14,
      chip_ev_loss: 13.58,
      ev_loss_bb: 0.27,
      severity: "Inaccuracy",
      reasoning: "Optimal Raise(81.25) (+79.14 chips) vs actual Raise(150.0) (+65.56 chips). Equity: 65.6%, Pot Odds: 25.0%."
    },
    {
      hand_id: 7001003,
      street: "Flop",
      hero_cards: ["Ah", "Kh"],
      board_cards: ["Qh", "Jh", "Th"],
      pot_before: 400.0,
      call_cost: 0.0,
      hero_stack: 5225.0,
      action_taken: { type: "Bet", amount: 200.0 },
      recommended_action: { type: "Bet", amount: 300.0 },
      actual_chip_ev: 452.00,
      optimal_chip_ev: 497.50,
      chip_ev_loss: 45.50,
      ev_loss_bb: 0.91,
      severity: "Inaccuracy",
      reasoning: "Optimal Bet(300.0) (+497.50 chips) vs actual Bet(200.0) (+452.00 chips). Equity: 100.0%, Pot Odds: 0.0%."
    },
    {
      hand_id: 7001003,
      street: "Turn",
      hero_cards: ["Ah", "Kh"],
      board_cards: ["Qh", "Jh", "Th", "2s"],
      pot_before: 800.0,
      call_cost: 0.0,
      hero_stack: 5025.0,
      action_taken: { type: "Bet", amount: 400.0 },
      recommended_action: { type: "Bet", amount: 600.0 },
      actual_chip_ev: 915.20,
      optimal_chip_ev: 1016.00,
      chip_ev_loss: 100.80,
      ev_loss_bb: 2.02,
      severity: "Mistake",
      reasoning: "Optimal Bet(600.0) (+1016.00 chips) vs actual Bet(400.0) (+915.20 chips). Equity: 100.0%, Pot Odds: 0.0%."
    },
    {
      hand_id: 7001003,
      street: "River",
      hero_cards: ["Ah", "Kh"],
      board_cards: ["Qh", "Jh", "Th", "2s", "3d"],
      pot_before: 1600.0,
      call_cost: 0.0,
      hero_stack: 4625.0,
      action_taken: { type: "Fold" },
      recommended_action: { type: "AllIn", amount: 4625.0 },
      actual_chip_ev: 0.0,
      optimal_chip_ev: 2090.00,
      chip_ev_loss: 2090.00,
      ev_loss_bb: 41.80,
      severity: "Blunder",
      reasoning: "Optimal AllIn(4625.0) (+2090.00 chips) vs actual Fold (+0.00 chips). Equity: 100.0%, Pot Odds: 0.0%."
    }
  ]
};

// ==========================================================================
// PLAYBACK CONTROLS & MANUAL EVALUATION STEPPING
// ==========================================================================
function setupPlaybackControls() {
  const btnNext = document.getElementById('btn-next-step');
  const btnPrev = document.getElementById('btn-prev-step');
  const btnToggle = document.getElementById('btn-toggle-play');

  if (btnNext) btnNext.addEventListener('click', nextDecision);
  if (btnPrev) btnPrev.addEventListener('click', prevDecision);
  if (btnToggle) btnToggle.addEventListener('click', toggleAutoPlay);

  // Keyboard shortcuts: Space or Right Arrow for Next, Left Arrow for Previous
  window.addEventListener('keydown', (e) => {
    if (['INPUT', 'TEXTAREA'].includes(document.activeElement?.tagName)) return;
    if (e.code === 'Space' || e.key === 'ArrowRight') {
      e.preventDefault();
      nextDecision();
    } else if (e.key === 'ArrowLeft') {
      e.preventDefault();
      prevDecision();
    }
  });

  updatePlaybackUI();
}

function goToDecision(index) {
  if (index < 0) index = 0;
  if (index >= DECISION_PLAYLIST.length) index = DECISION_PLAYLIST.length - 1;
  currentDecisionIndex = index;
  currentDecision = DECISION_PLAYLIST[currentDecisionIndex];
  renderDecisionReport(currentDecision);
  updatePlaybackUI();

  appendTickerMessage(`[Passo ${currentDecisionIndex + 1}/${DECISION_PLAYLIST.length}] Mão #${currentDecision.hand_id} [${currentDecision.street}]: Rec ${formatActionName(currentDecision.recommended_action)}`);

  // Inform backend WebSocket if connected
  if (ws && ws.readyState === WebSocket.OPEN) {
    ws.send(JSON.stringify({ cmd: "next", index: currentDecisionIndex }));
  }
}

function nextDecision() {
  if (currentDecisionIndex < DECISION_PLAYLIST.length - 1) {
    goToDecision(currentDecisionIndex + 1);
  } else {
    // Loop back to first hand
    goToDecision(0);
  }
}

function prevDecision() {
  if (currentDecisionIndex > 0) {
    goToDecision(currentDecisionIndex - 1);
  } else {
    goToDecision(DECISION_PLAYLIST.length - 1);
  }
}

function toggleAutoPlay() {
  isAutoPlay = !isAutoPlay;
  const icon = document.getElementById('play-icon');
  const text = document.getElementById('play-text');
  const badge = document.getElementById('playback-mode-badge');

  if (isAutoPlay) {
    if (icon) icon.textContent = '⏸';
    if (text) text.textContent = 'Pausar';
    if (badge) {
      badge.textContent = '▶ REPRODUÇÃO AUTOMÁTICA';
      badge.className = 'badge-playback-mode auto';
    }
    autoPlayTimer = setInterval(nextDecision, 4000);
    appendTickerMessage("Modo Auto-Play ativado (avança a cada 4 segundos).");
  } else {
    if (icon) icon.textContent = '▶';
    if (text) text.textContent = 'Auto-Play';
    if (badge) {
      badge.textContent = '⏸ PAUSADO (MODO MANUAL)';
      badge.className = 'badge-playback-mode manual';
    }
    if (autoPlayTimer) clearInterval(autoPlayTimer);
    autoPlayTimer = null;
    appendTickerMessage("Modo Manual ativado. Avalie no seu próprio ritmo com [Passo Seguinte].");
  }

  if (ws && ws.readyState === WebSocket.OPEN) {
    ws.send(JSON.stringify({ cmd: isAutoPlay ? "auto_on" : "auto_off" }));
  }
}

function updatePlaybackUI() {
  const dec = DECISION_PLAYLIST[currentDecisionIndex];
  const pill = document.getElementById('step-counter-pill');
  if (pill && dec) {
    pill.textContent = `Decisão ${currentDecisionIndex + 1} / ${DECISION_PLAYLIST.length} • Mão #${dec.hand_id} (${dec.street})`;
  }
}

// ==========================================================================
// INITIALIZATION
// ==========================================================================
document.addEventListener('DOMContentLoaded', () => {
  setupNavigation();
  initRangeGrid();
  setupPlaybackControls();
  goToDecision(0); // Start paused on Decision 1
  renderSessionAuditor(sessionAuditData);
  setupAuditorControls();
  initWebSocket();

  // Button: Simulate Live Hand toggles auto-play or steps
  const btnSim = document.getElementById('btn-simulate-hand');
  if (btnSim) btnSim.addEventListener('click', toggleAutoPlay);
});

// ==========================================================================
// NAVIGATION & VIEW SWITCHING
// ==========================================================================
function setupNavigation() {
  const tabLive = document.getElementById('tab-live-hud');
  const tabAudit = document.getElementById('tab-auditor');
  const viewLive = document.getElementById('view-live-hud');
  const viewAudit = document.getElementById('view-auditor');

  tabLive.addEventListener('click', () => {
    tabLive.classList.add('active');
    tabLive.setAttribute('aria-selected', 'true');
    tabAudit.classList.remove('active');
    tabAudit.setAttribute('aria-selected', 'false');

    viewLive.classList.add('active');
    viewAudit.classList.remove('active');
  });

  tabAudit.addEventListener('click', () => {
    tabAudit.classList.add('active');
    tabAudit.setAttribute('aria-selected', 'true');
    tabLive.classList.remove('active');
    tabLive.setAttribute('aria-selected', 'false');

    viewAudit.classList.add('active');
    viewLive.classList.remove('active');

    // Trigger canvas chart render when tab becomes visible
    drawTimelineChart();
  });
}

// ==========================================================================
// 13x13 RANGE HEATMAP MATRIX
// ==========================================================================
function initRangeGrid() {
  const container = document.getElementById('range-grid-13x13');
  container.innerHTML = '';

  const btnHero = document.getElementById('btn-range-hero');
  const btnVillain = document.getElementById('btn-range-villain');

  btnHero.addEventListener('click', () => {
    btnHero.classList.add('active');
    btnVillain.classList.remove('active');
    currentRangeMode = 'hero';
    updateRangeHeatmapWeights();
  });

  btnVillain.addEventListener('click', () => {
    btnVillain.classList.add('active');
    btnHero.classList.remove('active');
    currentRangeMode = 'villain';
    updateRangeHeatmapWeights();
  });

  for (let r = 0; r < 13; r++) {
    for (let c = 0; c < 13; c++) {
      const cell = document.createElement('div');
      cell.className = 'range-cell';
      
      let handName = '';
      let isPair = false;
      let isSuited = false;

      if (r === c) {
        handName = `${RANKS[r]}${RANKS[c]}`;
        isPair = true;
        cell.classList.add('pair');
      } else if (r < c) {
        handName = `${RANKS[r]}${RANKS[c]}s`;
        isSuited = true;
      } else {
        handName = `${RANKS[c]}${RANKS[r]}o`;
      }

      cell.textContent = handName;
      cell.dataset.hand = handName;
      cell.dataset.isPair = isPair;
      cell.dataset.isSuited = isSuited;

      cell.addEventListener('mouseenter', () => inspectRangeCell(handName, isPair, isSuited));
      container.appendChild(cell);
    }
  }

  updateRangeHeatmapWeights();
}

function getHandWeight(handName, mode) {
  // Analytical weights for poker ranges
  const premiumPairs = ['AA', 'KK', 'QQ', 'JJ', 'TT'];
  const broadwaySuited = ['AKs', 'AQs', 'AJs', 'ATs', 'KQs', 'KJs', 'QJs', 'JTs'];
  const broadwayOffsuit = ['AKo', 'AQo', 'AJo', 'KQo'];
  const connectors = ['98s', '87s', '76s', '65s', '54s', 'T9s'];
  
  if (mode === 'hero') {
    if (premiumPairs.includes(handName)) return 1.0;
    if (broadwaySuited.includes(handName)) return 0.95;
    if (broadwayOffsuit.includes(handName)) return 0.75;
    if (connectors.includes(handName)) return 0.60;
    if (handName.endsWith('s') && (handName.startsWith('A') || handName.startsWith('K'))) return 0.50;
    if (handName.startsWith('99') || handName.startsWith('88') || handName.startsWith('77')) return 0.70;
    return 0.10;
  } else {
    // Villain Bayesian filtered range (polar/tightened against Hero's bet)
    if (premiumPairs.includes(handName)) return 0.85;
    if (broadwaySuited.includes(handName)) return 0.65;
    if (handName === 'QJs' || handName === 'JTs' || handName === 'T9s') return 0.90; // Connected boards
    if (handName.endsWith('o') && !broadwayOffsuit.includes(handName)) return 0.02;
    return 0.20;
  }
}

function updateRangeHeatmapWeights() {
  const cells = document.querySelectorAll('.range-cell');
  cells.forEach(cell => {
    const hand = cell.dataset.hand;
    const weight = getHandWeight(hand, currentRangeMode);
    cell.dataset.weight = weight;

    if (currentRangeMode === 'hero') {
      // Emerald gradient
      cell.style.background = `rgba(16, 185, 129, ${Math.max(0.08, weight * 0.85)})`;
      cell.style.color = weight > 0.4 ? '#ffffff' : '#9ca3af';
    } else {
      // Cyan/Blue gradient for villain
      cell.style.background = `rgba(6, 182, 212, ${Math.max(0.08, weight * 0.85)})`;
      cell.style.color = weight > 0.4 ? '#ffffff' : '#9ca3af';
    }
  });
}

function inspectRangeCell(handName, isPair, isSuited) {
  const weight = getHandWeight(handName, currentRangeMode);
  const combos = isPair ? 6 : (isSuited ? 4 : 12);
  let category = "Marginal";
  if (['AA', 'KK', 'QQ'].includes(handName)) category = "Premium Pair";
  else if (handName.includes('A') && handName.includes('K')) category = "Premium Broadway";
  else if (isSuited && ['QJs', 'JTs', 'T9s', '98s'].includes(handName)) category = "Suited Connector";
  else if (isPair) category = "Pocket Pair";

  const inspector = document.getElementById('range-cell-inspector');
  inspector.innerHTML = `
    <span class="inspector-hand">${handName}</span>
    <span class="inspector-details">${combos} combos | Weight: ${(weight * 100).toFixed(1)}% | ${category}</span>
  `;
}

// ==========================================================================
// DECISION REPORT & LIVE EV CARD RENDERER
// ==========================================================================
function renderDecisionReport(dec) {
  // 1. Table Info
  if (dec.hand_id) {
    const el = document.getElementById('live-hand-id');
    if (el) el.textContent = `#${dec.hand_id}`;
  }
  if (dec.hero_stack) {
    const el = document.getElementById('live-hero-stack');
    if (el) el.textContent = `${formatChips(dec.hero_stack)} chips`;
  }
  document.getElementById('live-street-badge').textContent = dec.street.toUpperCase();
  document.getElementById('live-street-badge').className = `badge street-badge street-${dec.street.toLowerCase()}`;
  document.getElementById('live-pot-amount').textContent = formatChips(dec.current_pot);
  document.getElementById('live-call-cost').textContent = `${formatChips(dec.call_cost)} chips`;

  // 2. Hero & Board Cards
  renderCards('hero-cards-container', dec.hero_cards, true);
  renderCards('board-cards-container', dec.board_cards, false);

  const comboLabel = document.getElementById('hero-combo-label');
  if (comboLabel && dec.hero_cards && dec.hero_cards.length === 2) {
    comboLabel.textContent = formatComboDescription(dec.hero_cards, dec.board_cards);
  }
  highlightHeroComboInMatrix(dec.hero_cards);

  // 3. Recommended Action Badge
  const rec = dec.recommended_action;
  const recActionName = formatActionName(rec);
  document.getElementById('rec-action-name').textContent = recActionName;

  // 4. EV Differential Action Bars
  const barsContainer = document.getElementById('ev-action-bars');
  barsContainer.innerHTML = '';

  const maxChipEv = Math.max(...dec.action_evaluations.map(a => a.chip_ev), 1.0);
  const optimalAction = dec.recommended_action;
  const optimalEval = dec.action_evaluations.find(a => actionEquals(a.action, optimalAction)) || dec.action_evaluations[0];
  const optimalEv = optimalEval.chip_ev;

  dec.action_evaluations.forEach(evalItem => {
    const isRec = actionEquals(evalItem.action, optimalAction);
    const actName = formatActionName(evalItem.action);
    const chipEv = evalItem.chip_ev;
    const delta = chipEv - optimalEv;

    const row = document.createElement('div');
    row.className = `action-bar-row ${isRec ? 'recommended' : ''}`;

    const fillPct = Math.max(5, (Math.max(0, chipEv) / maxChipEv) * 100);
    const fillClass = chipEv > 0 ? 'positive' : (chipEv === 0 ? 'zero' : 'negative');

    row.innerHTML = `
      <div class="action-name-col">
        ${isRec ? '<span style="color:#f59e0b">★</span>' : ''}
        <span>${actName}</span>
      </div>
      <div class="action-bar-track">
        <div class="action-bar-fill ${fillClass}" style="width: ${fillPct}%"></div>
      </div>
      <div class="action-ev-val ${chipEv >= 0 ? 'text-emerald' : 'text-crimson'}">
        ${chipEv >= 0 ? '+' : ''}${chipEv.toFixed(1)} chips
      </div>
      <div class="action-delta-val">
        ${isRec ? '<b class="text-gold">BEST</b>' : `${delta.toFixed(1)}`}
      </div>
    `;

    barsContainer.appendChild(row);
  });

  // 5. Pot Odds vs Equity Gauge
  updateGauge(dec.hero_equity, dec.pot_odds);

  // 6. Solver Reasoning Box
  document.getElementById('solver-reasoning-text').textContent = dec.reasoning;
}

function updateGauge(equity, potOdds) {
  const circumference = 408.4; // 2 * PI * 65
  
  // Equity Arc
  const equityArc = document.getElementById('gauge-equity-arc');
  const equityOffset = circumference - (equity * circumference);
  equityArc.style.strokeDashoffset = equityOffset;

  // Pot Odds (Break-even threshold) Arc
  const beArc = document.getElementById('gauge-breakeven-arc');
  const beOffset = circumference - (potOdds * circumference);
  beArc.style.strokeDashoffset = beOffset;

  // Numbers
  document.getElementById('dial-equity-pct').textContent = `${(equity * 100).toFixed(1)}%`;
  document.getElementById('dial-pot-odds-val').textContent = `${(potOdds * 100).toFixed(1)}%`;

  const margin = (equity - potOdds) * 100;
  const marginEl = document.getElementById('dial-margin-val');
  if (margin >= 0) {
    marginEl.textContent = `+${margin.toFixed(1)}% Margin`;
    marginEl.className = 'foot-col margin-positive text-emerald';
  } else {
    marginEl.textContent = `${margin.toFixed(1)}% Deficit`;
    marginEl.className = 'foot-col margin-negative text-crimson';
  }
}

function renderCards(containerId, cardsList, isHero) {
  const container = document.getElementById(containerId);
  container.innerHTML = '';

  cardsList.forEach(cardStr => {
    const cardEl = document.createElement('div');
    const rank = cardStr[0];
    const suitChar = cardStr[1];
    let suitSymbol = '♠';
    let suitClass = 'card-spades';

    switch (suitChar) {
      case 'h': suitSymbol = '♥'; suitClass = 'card-hearts'; break;
      case 'd': suitSymbol = '♦'; suitClass = 'card-diamonds'; break;
      case 'c': suitSymbol = '♣'; suitClass = 'card-clubs'; break;
      case 's': suitSymbol = '♠'; suitClass = 'card-spades'; break;
    }

    cardEl.className = `poker-card ${suitClass} ${isHero ? 'hero-card' : ''}`;
    cardEl.innerHTML = `
      <span class="card-rank">${rank}</span>
      <span class="card-suit">${suitSymbol}</span>
    `;
    container.appendChild(cardEl);
  });

  // If board has less than 5 cards, pad with placeholders
  if (!isHero && cardsList.length < 5) {
    for (let i = cardsList.length; i < 5; i++) {
      const ph = document.createElement('div');
      ph.className = 'poker-card card-placeholder';
      ph.innerHTML = '<span class="placeholder-dot">?</span>';
      container.appendChild(ph);
    }
  }
}

function getCanonicalCombo(c1, c2) {
  if (!c1 || !c2 || c1.length < 2 || c2.length < 2) return null;
  const r1 = c1[0], s1 = c1[1];
  const r2 = c2[0], s2 = c2[1];
  const rankOrder = 'AKQJT98765432';
  const i1 = rankOrder.indexOf(r1);
  const i2 = rankOrder.indexOf(r2);
  if (i1 === -1 || i2 === -1) return null;
  if (i1 === i2) return `${r1}${r2}`;
  const isSuited = s1 === s2;
  const high = i1 < i2 ? r1 : r2;
  const low = i1 < i2 ? r2 : r1;
  return `${high}${low}${isSuited ? 's' : 'o'}`;
}

function highlightHeroComboInMatrix(heroCards) {
  document.querySelectorAll('.range-cell.active-hero-combo').forEach(el => {
    el.classList.remove('active-hero-combo');
  });
  if (!heroCards || heroCards.length < 2) return;
  const combo = getCanonicalCombo(heroCards[0], heroCards[1]);
  if (!combo) return;
  const cell = document.querySelector(`.range-cell[data-hand="${combo}"]`);
  if (cell) {
    cell.classList.add('active-hero-combo');
  }
}

function formatComboDescription(heroCards, boardCards) {
  if (!heroCards || heroCards.length < 2) return "Active Hand";
  const r1 = heroCards[0][0], s1 = heroCards[0][1];
  const r2 = heroCards[1][0], s2 = heroCards[1][1];
  const isPair = r1 === r2;
  const isSuited = s1 === s2;
  const numBoard = boardCards ? boardCards.length : 0;
  
  if (isPair) {
    if (r1 === 'A') return numBoard > 0 ? "Pocket Aces / Overpair" : "Pocket Aces (AA)";
    if (r1 === 'K') return numBoard > 0 ? "Three of a Kind (Set of Kings)" : "Pocket Kings (KK)";
    if (r1 === 'Q') return numBoard > 0 ? "Pocket Queens / Overpair" : "Pocket Queens (QQ)";
    return `Pocket Pair (${r1}${r2})`;
  }
  if ((r1 === 'A' && r2 === 'K') || (r1 === 'K' && r2 === 'A')) {
    if (isSuited && numBoard >= 3) return "Royal Flush Draw / Nut Broadway Straight";
    return isSuited ? "AK Suited (Big Slick)" : "AK Offsuit";
  }
  if ((r1 === '9' && r2 === '8') || (r1 === '8' && r2 === '9')) {
    return numBoard >= 3 ? "Nut Straight (9-High)" : "Suited Connector (9-8s)";
  }
  return `${r1}${r2}${isSuited ? 's' : 'o'} Hand`;
}

// ==========================================================================
// SESSION AUDITOR VIEW & LEAKS TIMELINE
// ==========================================================================
function renderSessionAuditor(report) {
  // KPI Metrics
  document.getElementById('kpi-total-hands').textContent = report.total_hands;
  document.getElementById('kpi-hero-hands').textContent = report.hero_hands;
  document.getElementById('kpi-total-decisions').textContent = report.total_decisions;
  document.getElementById('kpi-cumulative-bb-loss').textContent = `-${report.total_bb_ev_loss.toFixed(2)} BB`;
  document.getElementById('kpi-cumulative-chips').textContent = `-${report.total_chip_ev_loss.toFixed(2)}`;
  document.getElementById('kpi-blunder-count').textContent = report.blunder_count;
  document.getElementById('kpi-mistake-count').textContent = report.mistake_count;
  document.getElementById('kpi-inaccuracy-count').textContent = report.inaccuracy_count;
  document.getElementById('kpi-accuracy-rate').textContent = `${report.leak_free_percentage.toFixed(1)}%`;

  renderAuditTable(report.all_audits);
  drawTimelineChart();
}

function renderAuditTable(audits) {
  const tbody = document.getElementById('audit-table-body');
  tbody.innerHTML = '';

  const filtered = audits.filter(a => {
    if (currentActiveFilter === 'all') return true;
    return a.severity.toLowerCase() === currentActiveFilter.toLowerCase();
  });

  filtered.forEach(audit => {
    const tr = document.createElement('tr');
    tr.dataset.handId = audit.hand_id;

    const cardsDisplay = audit.hero_cards.join(' ');
    const boardDisplay = audit.board_cards.length > 0 ? audit.board_cards.join(' ') : '-';
    const actionTakenStr = formatActionName(audit.action_taken);
    const recActionStr = formatActionName(audit.recommended_action);
    const sevClass = `sev-${audit.severity.toLowerCase()}`;

    tr.innerHTML = `
      <td class="font-mono">#${audit.hand_id}</td>
      <td><span class="badge street-badge street-${audit.street.toLowerCase()}">${audit.street}</span></td>
      <td class="font-mono" style="font-weight:700">${cardsDisplay}</td>
      <td class="font-mono">${boardDisplay}</td>
      <td><b>${actionTakenStr}</b></td>
      <td class="text-gold"><b>${recActionStr}</b></td>
      <td class="font-mono text-crimson">-${audit.chip_ev_loss.toFixed(1)}</td>
      <td class="font-mono text-crimson" style="font-weight:800">-${audit.ev_loss_bb.toFixed(2)} BB</td>
      <td><span class="badge-severity ${sevClass}">${audit.severity.toUpperCase()}</span></td>
    `;

    tr.addEventListener('click', () => openDecisionModal(audit));
    tbody.appendChild(tr);
  });
}

function setupAuditorControls() {
  // Filter buttons
  const filterPills = document.querySelectorAll('.filter-pill');
  filterPills.forEach(pill => {
    pill.addEventListener('click', () => {
      filterPills.forEach(p => p.classList.remove('active'));
      pill.classList.add('active');
      currentActiveFilter = pill.dataset.filter;
      renderAuditTable(sessionAuditData.all_audits);
    });
  });

  // Modal close
  document.getElementById('btn-close-modal').addEventListener('click', closeModal);
  document.getElementById('decision-modal').addEventListener('click', (e) => {
    if (e.target.id === 'decision-modal') closeModal();
  });

  // Load sample session button
  document.getElementById('btn-load-sample').addEventListener('click', () => {
    document.getElementById('auditor-file-name').textContent = 'sample_tournament.txt';
    renderSessionAuditor(sessionAuditData);
  });

  // File upload
  const fileInput = document.getElementById('audit-file-input');
  document.getElementById('btn-trigger-upload').addEventListener('click', () => fileInput.click());
  fileInput.addEventListener('change', (e) => {
    const file = e.target.files[0];
    if (file) {
      document.getElementById('auditor-file-name').textContent = file.name;
      const reader = new FileReader();
      reader.onload = (event) => {
        // If file content is loaded, we can simulate an audit or process
        appendTickerMessage(`Loaded hand log file '${file.name}' (${(file.size / 1024).toFixed(1)} KB)`);
      };
      reader.readAsText(file);
    }
  });

  // Export JSON Report
  document.getElementById('btn-export-audit-json').addEventListener('click', () => {
    const dataStr = "data:text/json;charset=utf-8," + encodeURIComponent(JSON.stringify(sessionAuditData, null, 2));
    const dl = document.createElement('a');
    dl.setAttribute("href", dataStr);
    dl.setAttribute("download", "session_audit_report.json");
    dl.click();
  });
}

function drawTimelineChart() {
  const canvas = document.getElementById('audit-timeline-canvas');
  if (!canvas) return;
  const ctx = canvas.getContext('2d');
  const w = canvas.width;
  const h = canvas.height;

  ctx.clearRect(0, 0, w, h);

  // Background Grid Lines
  ctx.strokeStyle = "rgba(255, 255, 255, 0.05)";
  ctx.lineWidth = 1;
  for (let y = 30; y < h; y += 40) {
    ctx.beginPath();
    ctx.moveTo(40, y);
    ctx.lineTo(w - 20, y);
    ctx.stroke();
  }

  const audits = sessionAuditData.all_audits;
  if (audits.length === 0) return;

  // Compute cumulative BB loss over decisions
  let cumLoss = 0;
  const points = [{ x: 0, y: 0 }];
  audits.forEach((a, idx) => {
    cumLoss += a.ev_loss_bb;
    points.push({ x: idx + 1, y: cumLoss });
  });

  const maxLoss = Math.max(cumLoss * 1.15, 10);
  const paddingLeft = 50;
  const paddingBottom = 30;
  const plotW = w - paddingLeft - 30;
  const plotH = h - paddingBottom - 20;

  // Y-axis labels
  ctx.fillStyle = "#6b7280";
  ctx.font = "10px Inter";
  ctx.textAlign = "right";
  ctx.fillText("0 BB", paddingLeft - 10, h - paddingBottom);
  ctx.fillText(`-${(maxLoss / 2).toFixed(1)} BB`, paddingLeft - 10, h - paddingBottom - plotH / 2);
  ctx.fillText(`-${maxLoss.toFixed(1)} BB`, paddingLeft - 10, 25);

  // Stepped Area under the line
  ctx.beginPath();
  ctx.moveTo(paddingLeft, h - paddingBottom);

  points.forEach((pt, i) => {
    const px = paddingLeft + (pt.x / audits.length) * plotW;
    const py = (h - paddingBottom) - (pt.y / maxLoss) * plotH;

    if (i > 0) {
      const prevX = paddingLeft + (points[i - 1].x / audits.length) * plotW;
      ctx.lineTo(px, (h - paddingBottom) - (points[i - 1].y / maxLoss) * plotH); // Step horizontal
    }
    ctx.lineTo(px, py); // Step vertical
  });

  ctx.lineTo(paddingLeft + plotW, h - paddingBottom);
  ctx.closePath();

  const grad = ctx.createLinearGradient(0, 0, 0, h);
  grad.addColorStop(0, "rgba(239, 68, 68, 0.35)");
  grad.addColorStop(1, "rgba(239, 68, 68, 0.02)");
  ctx.fillStyle = grad;
  ctx.fill();

  // Line Stroke
  ctx.beginPath();
  points.forEach((pt, i) => {
    const px = paddingLeft + (pt.x / audits.length) * plotW;
    const py = (h - paddingBottom) - (pt.y / maxLoss) * plotH;

    if (i === 0) {
      ctx.moveTo(px, py);
    } else {
      ctx.lineTo(px, (h - paddingBottom) - (points[i - 1].y / maxLoss) * plotH);
      ctx.lineTo(px, py);
    }
  });
  ctx.strokeStyle = "#ef4444";
  ctx.lineWidth = 2.5;
  ctx.stroke();

  // Blunder Markers (Red glowing dots on big steps)
  audits.forEach((a, idx) => {
    if (a.severity === "Blunder") {
      const pt = points[idx + 1];
      const px = paddingLeft + (pt.x / audits.length) * plotW;
      const py = (h - paddingBottom) - (pt.y / maxLoss) * plotH;

      ctx.beginPath();
      ctx.arc(px, py, 6, 0, Math.PI * 2);
      ctx.fillStyle = "#ef4444";
      ctx.shadowColor = "#ef4444";
      ctx.shadowBlur = 10;
      ctx.fill();
      ctx.shadowBlur = 0;
    }
  });
}

function openDecisionModal(audit) {
  const modal = document.getElementById('decision-modal');
  document.getElementById('modal-title').textContent = `Hand #${audit.hand_id} - ${audit.street} Decision Node`;

  const boardStr = audit.board_cards.length > 0 ? audit.board_cards.join(' ') : 'None (Preflop)';
  const sevBadge = `<span class="badge-severity sev-${audit.severity.toLowerCase()}">${audit.severity.toUpperCase()}</span>`;

  document.getElementById('modal-content').innerHTML = `
    <div style="display:flex; justify-content:space-between; margin-bottom:16px;">
      <div>
        <div style="font-size:0.75rem; color:#9ca3af">HERO HOLE CARDS</div>
        <div class="font-mono" style="font-size:1.2rem; font-weight:800; color:#10b981">${audit.hero_cards.join(' ')}</div>
      </div>
      <div>
        <div style="font-size:0.75rem; color:#9ca3af">COMMUNITY BOARD</div>
        <div class="font-mono" style="font-size:1.2rem">${boardStr}</div>
      </div>
      <div>
        <div style="font-size:0.75rem; color:#9ca3af">LEAK SEVERITY</div>
        <div style="margin-top:4px">${sevBadge}</div>
      </div>
    </div>

    <div style="background:rgba(0,0,0,0.3); padding:12px; border-radius:8px; margin-bottom:16px;">
      <div style="display:grid; grid-template-columns:1fr 1fr; gap:12px;">
        <div>Action Taken: <b>${formatActionName(audit.action_taken)}</b> (EV: ${audit.actual_chip_ev >= 0 ? '+' : ''}${audit.actual_chip_ev.toFixed(1)})</div>
        <div style="color:#f59e0b">Recommended: <b>${formatActionName(audit.recommended_action)}</b> (EV: +${audit.optimal_chip_ev.toFixed(1)})</div>
      </div>
      <div style="margin-top:8px; color:#ef4444; font-weight:700">
        EV Cost: -${audit.chip_ev_loss.toFixed(1)} chips (-${audit.ev_loss_bb.toFixed(2)} Big Blinds)
      </div>
    </div>

    <div style="font-size:0.85rem; line-height:1.6; color:#d1d5db; background:rgba(255,255,255,0.03); padding:12px; border-radius:8px;">
      <b>Analytical Justification:</b><br>
      ${audit.reasoning}
    </div>
  `;

  modal.classList.remove('hidden');
}

function closeModal() {
  document.getElementById('decision-modal').classList.add('hidden');
}

// ==========================================================================
// WEBSOCKET INTEGRATION
// ==========================================================================
function initWebSocket() {
  const urlInput = document.getElementById('ws-url-input');
  const btnConnect = document.getElementById('btn-ws-connect');
  const statusInd = document.getElementById('ws-status-indicator');
  const statusTxt = document.getElementById('ws-status-text');

  // If page is loaded via HTTP, automatically use same host for WebSocket
  if (window.location.protocol.startsWith('http')) {
    urlInput.value = `ws://${window.location.host}`;
  }

  function connect() {
    const url = urlInput.value.trim();
    if (!url) return;

    if (ws) {
      ws.close();
    }

    statusTxt.textContent = "Connecting...";
    statusInd.className = "status-indicator offline";

    try {
      ws = new WebSocket(url);

      ws.onopen = () => {
        statusInd.className = "status-indicator online";
        statusTxt.textContent = "LIVE";
        appendTickerMessage(`Connected to PokerAnalyse WebSocket streaming server at ${url}`);
      };

      ws.onmessage = (event) => {
        try {
          const msg = JSON.parse(event.data);
          handleServerMessage(msg);
        } catch (e) {
          console.error("Failed to parse WS message", e);
        }
      };

      ws.onerror = () => {
        statusInd.className = "status-indicator offline";
        statusTxt.textContent = "Error";
      };

      ws.onclose = () => {
        statusInd.className = "status-indicator offline";
        statusTxt.textContent = "Offline";
        // Auto-reconnect attempt after 3s
        setTimeout(connect, 3000);
      };
    } catch (err) {
      console.warn("WS connect failed", err);
    }
  }

  btnConnect.addEventListener('click', connect);

  // Auto-connect attempt on startup
  connect();
}

function handleServerMessage(msg) {
  if (msg.msg_type === "Decision") {
    currentDecision = msg.payload;
    renderDecisionReport(currentDecision);
    appendTickerMessage(`[Decision] ${currentDecision.street}: Recommended ${formatActionName(currentDecision.recommended_action)} (EV +${currentDecision.action_evaluations[0]?.chip_ev.toFixed(1) || 0})`);
  } else if (msg.msg_type === "Event") {
    const ev = msg.payload;
    appendTickerMessage(`[Event] ${JSON.stringify(ev)}`);
  }
}

function appendTickerMessage(text) {
  const ticker = document.getElementById('event-ticker-content');
  ticker.textContent = text;
}

// ==========================================================================
// SIMULATION / DEMO MODE
// ==========================================================================
function runSimulatedHandAnimation() {
  appendTickerMessage("Starting simulated tournament hand sequence...");

  // Step 1: Preflop
  currentDecision.street = "Preflop";
  currentDecision.current_pot = 225.0;
  currentDecision.call_cost = 50.0;
  currentDecision.hero_cards = ["Ac", "As"];
  currentDecision.board_cards = [];
  currentDecision.hero_equity = 0.852;
  currentDecision.pot_odds = 0.222;
  currentDecision.action_evaluations = [
    { action: { type: "Fold" }, chip_ev: 0.0, break_even_equity: 0.0 },
    { action: { type: "Call", amount: 50.0 }, chip_ev: 141.6, break_even_equity: 0.22 },
    { action: { type: "Raise", amount: 150.0 }, chip_ev: 182.4, break_even_equity: 0.40 },
    { action: { type: "AllIn", amount: 4850.0 }, chip_ev: 150.0, break_even_equity: 0.70 }
  ];
  currentDecision.recommended_action = { type: "Raise", amount: 150.0 };
  currentDecision.reasoning = "Premium Pocket Aces. Raise 3x BB to build the pot and isolate villain range.";
  renderDecisionReport(currentDecision);

  // Step 2: Flop after 2.5s
  setTimeout(() => {
    currentDecision.street = "Flop";
    currentDecision.current_pot = 425.0;
    currentDecision.call_cost = 0.0;
    currentDecision.board_cards = ["Kd", "7h", "2c"];
    currentDecision.hero_equity = 0.884;
    currentDecision.pot_odds = 0.0;
    currentDecision.action_evaluations = [
      { action: { type: "Check" }, chip_ev: 375.7, break_even_equity: 0.0 },
      { action: { type: "Bet", amount: 140.0 }, chip_ev: 420.2, break_even_equity: 0.24 },
      { action: { type: "Bet", amount: 318.0 }, chip_ev: 445.8, break_even_equity: 0.42 },
      { action: { type: "AllIn", amount: 4700.0 }, chip_ev: 390.0, break_even_equity: 0.75 }
    ];
    currentDecision.recommended_action = { type: "Bet", amount: 318.0 };
    currentDecision.reasoning = "Top pair board texture. Bet 75% pot for maximum value against Kings and pocket pairs.";
    renderDecisionReport(currentDecision);
    appendTickerMessage("[Flop] Kd 7h 2c dealt | Pot: 425 chips | Recommended Bet 75%");
  }, 2500);

  // Step 3: Turn after 5s
  setTimeout(() => {
    currentDecision.street = "Turn";
    currentDecision.current_pot = 1060.0;
    currentDecision.board_cards = ["Kd", "7h", "2c", "Js"];
    currentDecision.hero_equity = 0.845;
    currentDecision.action_evaluations = [
      { action: { type: "Check" }, chip_ev: 895.0, break_even_equity: 0.0 },
      { action: { type: "Bet", amount: 795.0 }, chip_ev: 1045.0, break_even_equity: 0.42 },
      { action: { type: "AllIn", amount: 4382.0 }, chip_ev: 980.0, break_even_equity: 0.70 }
    ];
    currentDecision.recommended_action = { type: "Bet", amount: 795.0 };
    currentDecision.reasoning = "Safe turn card. Continue barrelling 75% pot with overpair value.";
    renderDecisionReport(currentDecision);
    appendTickerMessage("[Turn] Js dealt | Pot: 1,060 chips | Recommended Bet 75%");
  }, 5000);
}

// ==========================================================================
// UTILITY HELPERS
// ==========================================================================
function formatActionName(action) {
  if (!action) return "Check";
  if (action.type === "Fold") return "Fold";
  if (action.type === "Check") return "Check";
  if (action.type === "Call") return `Call ${formatChips(action.amount)}`;
  if (action.type === "Bet") return `Bet ${formatChips(action.amount)}`;
  if (action.type === "Raise") return `Raise to ${formatChips(action.amount)}`;
  if (action.type === "AllIn") return `All-In (${formatChips(action.amount)})`;
  return JSON.stringify(action);
}

function actionEquals(a, b) {
  if (!a || !b) return false;
  return a.type === b.type;
}

function formatChips(num) {
  if (num === undefined || num === null) return "0";
  return Math.round(num).toLocaleString();
}
