use crate::card::Card;
use crate::ingestion::schema::{ReplayActionStep, ReplayHandScenario, ReplayPlayer};
use crate::orchestrator::event_bus::PlayerAction;
use std::collections::HashMap;

/// Parsed tournament hand history data ready for replay/analysis.
#[derive(Debug, Clone, PartialEq)]
pub struct ParsedHandHistory {
    pub hand_id: u64,
    pub tournament_id: Option<u64>,
    pub sb_amount: f64,
    pub bb_amount: f64,
    pub ante_amount: f64,
    pub hero_name: Option<String>,
    pub hero_cards: Option<[Card; 2]>,
    pub button_seat: usize,
    pub players: Vec<ReplayPlayer>,
    pub actions: Vec<ReplayActionStep>,
    pub board_cards: Vec<Card>,
}

impl ParsedHandHistory {
    /// Converts the parsed hand into a ReplayHandScenario usable by MockReplayAdapter.
    pub fn to_replay_scenario(&self, payouts: Option<Vec<f64>>) -> ReplayHandScenario {
        ReplayHandScenario {
            hand_id: self.hand_id,
            button_idx: self.button_seat.saturating_sub(1),
            sb_amount: self.sb_amount,
            bb_amount: self.bb_amount,
            players: self.players.clone(),
            board_cards: self.board_cards.iter().map(|c| c.to_string()).collect(),
            action_script: self.actions.clone(),
            payouts,
            hero_idx: self
                .hero_name
                .as_ref()
                .and_then(|h| self.players.iter().position(|p| &p.name == h)),
        }
    }
}

/// Parser for standard tournament hand history text formats (PokerStars, GGPoker, etc.).
pub struct HandHistoryParser;

impl HandHistoryParser {
    /// Parses a raw hand history text block into a structured `ParsedHandHistory`.
    pub fn parse(text: &str) -> Result<ParsedHandHistory, String> {
        let mut hand_id = 0u64;
        let mut tournament_id = None;
        let mut sb_amount = 0.0;
        let mut bb_amount = 0.0;
        let mut ante_amount = 0.0;
        let mut button_seat = 1;
        let mut hero_name = None;
        let mut hero_cards = None;
        let mut seat_map: HashMap<usize, (String, f64)> = HashMap::new();
        let mut player_name_to_idx: HashMap<String, usize> = HashMap::new();
        let mut actions = Vec::new();
        let mut board_cards = Vec::new();

        let mut in_preflop = false;
        let mut in_flop = false;
        let mut in_turn = false;
        let mut in_river = false;
        let mut in_showdown = false;

        for raw_line in text.lines() {
            let line = raw_line.trim();
            if line.is_empty() {
                continue;
            }

            // 1. Hand ID / Tournament ID Line
            // e.g.: "PokerStars Hand #23849102938: Tournament #340918239, $10+$1 USD Hold'em No Limit - Level I (10/20) - 2026/09/05"
            // or: "PokerStars Hand #123456789: Hold'em No Limit (10/20) - 2026/09/05"
            if line.starts_with("PokerStars Hand #") || line.starts_with("Hand #") {
                if let Some(hash_pos) = line.find('#') {
                    let rest = &line[hash_pos + 1..];
                    if let Some(colon_pos) = rest.find(':') {
                        let id_str = &rest[..colon_pos].trim();
                        if let Ok(id) = id_str.parse::<u64>() {
                            hand_id = id;
                        }
                    }
                }
                if let Some(tourn_pos) = line.find("Tournament #") {
                    let rest = &line[tourn_pos + 12..];
                    if let Some(comma_pos) = rest.find(',') {
                        if let Ok(tid) = rest[..comma_pos].trim().parse::<u64>() {
                            tournament_id = Some(tid);
                        }
                    }
                }
                // Extract level blinds: "(10/20)" or "(50/100)"
                if let (Some(open_p), Some(close_p)) = (line.rfind('('), line.rfind(')')) {
                    if open_p < close_p {
                        let blinds_str = &line[open_p + 1..close_p];
                        let parts: Vec<&str> = blinds_str.split('/').collect();
                        if parts.len() >= 2 {
                            if let (Ok(sb), Ok(bb)) = (
                                parts[0].trim().parse::<f64>(),
                                parts[1].trim().parse::<f64>(),
                            ) {
                                sb_amount = sb;
                                bb_amount = bb;
                            }
                        }
                    }
                }
                continue;
            }

            // 2. Table / Button Line
            // e.g.: "Table '340918239 1' 9-max Seat #1 is the button"
            if line.contains("is the button") {
                if let Some(seat_pos) = line.find("Seat #") {
                    let rest = &line[seat_pos + 6..];
                    if let Some(space_pos) = rest.find(' ') {
                        if let Ok(btn) = rest[..space_pos].trim().parse::<usize>() {
                            button_seat = btn;
                        }
                    }
                }
                continue;
            }

            // 3. Seat definitions
            // e.g.: "Seat 1: Player1 (1000 in chips)"
            // e.g.: "Seat 2: Hero (2500 in chips)"
            if line.starts_with("Seat ") && line.contains(" in chips)") {
                if let Some(colon_pos) = line.find(':') {
                    let seat_str = line[5..colon_pos].trim();
                    if let Ok(seat_num) = seat_str.parse::<usize>() {
                        let after_colon = line[colon_pos + 1..].trim();
                        if let Some(open_p) = after_colon.rfind('(') {
                            let name = after_colon[..open_p].trim().to_string();
                            let chip_part = &after_colon[open_p + 1..];
                            if let Some(in_pos) = chip_part.find(" in chips)") {
                                if let Ok(chips) = chip_part[..in_pos].trim().parse::<f64>() {
                                    seat_map.insert(seat_num, (name, chips));
                                }
                            }
                        }
                    }
                }
                continue;
            }

            // 4. Antes and Blinds posting
            // e.g.: "Player1: posts small blind 10"
            // e.g.: "Player2: posts big blind 20"
            // e.g.: "Player1: posts the ante 5"
            if line.contains("posts the ante") {
                if let Some(last_word) = line.split_whitespace().last() {
                    if let Ok(ante) = last_word.parse::<f64>() {
                        ante_amount = ante;
                    }
                }
                continue;
            }
            if line.contains("posts small blind") {
                if let Some(last_word) = line.split_whitespace().last() {
                    if let Ok(sb) = last_word.parse::<f64>() {
                        sb_amount = sb;
                    }
                }
                continue;
            }
            if line.contains("posts big blind") {
                if let Some(last_word) = line.split_whitespace().last() {
                    if let Ok(bb) = last_word.parse::<f64>() {
                        bb_amount = bb;
                    }
                }
                continue;
            }

            // 5. Dealt to Hero [Xx Yy]
            // e.g.: "Dealt to Hero [Ah Kd]"
            if let Some(rest) = line.strip_prefix("Dealt to ") {
                if let Some(open_b) = rest.find('[') {
                    let name = rest[..open_b].trim().to_string();
                    hero_name = Some(name);
                    if let Some(close_b) = rest.find(']') {
                        let cards_str = &rest[open_b + 1..close_b];
                        let parts: Vec<&str> = cards_str.split_whitespace().collect();
                        if parts.len() == 2 {
                            if let (Ok(c1), Ok(c2)) = (
                                Card::from_str_exact(parts[0]),
                                Card::from_str_exact(parts[1]),
                            ) {
                                hero_cards = Some([c1, c2]);
                            }
                        }
                    }
                }
                continue;
            }

            // 6. Street transitions
            if line.starts_with("*** HOLE CARDS ***") {
                in_preflop = true;
                continue;
            }
            if line.starts_with("*** FLOP ***") {
                in_preflop = false;
                in_flop = true;
                Self::extract_board_cards(line, &mut board_cards);
                continue;
            }
            if line.starts_with("*** TURN ***") {
                in_flop = false;
                in_turn = true;
                Self::extract_turn_river_card(line, &mut board_cards);
                continue;
            }
            if line.starts_with("*** RIVER ***") {
                in_turn = false;
                in_river = true;
                Self::extract_turn_river_card(line, &mut board_cards);
                continue;
            }
            if line.starts_with("*** SHOW DOWN ***") || line.starts_with("*** SUMMARY ***") {
                in_river = false;
                in_showdown = true;
                continue;
            }

            // 7. Player actions during betting rounds
            if (in_preflop || in_flop || in_turn || in_river) && !in_showdown {
                if let Some(colon_pos) = line.find(':') {
                    let actor_name = line[..colon_pos].trim();
                    let action_text = line[colon_pos + 1..].trim();

                    if let Some(action) = Self::parse_action(action_text) {
                        let street = if in_river {
                            "River".to_string()
                        } else if in_turn {
                            "Turn".to_string()
                        } else if in_flop {
                            "Flop".to_string()
                        } else {
                            "Preflop".to_string()
                        };
                        actions.push((actor_name.to_string(), action, street));
                    }
                }
            }
        }

        if seat_map.is_empty() {
            return Err("Failed to parse seat definitions from hand history".to_string());
        }

        // Sort seats by seat number (1..=N)
        let mut sorted_seats: Vec<usize> = seat_map.keys().copied().collect();
        sorted_seats.sort_unstable();

        let mut replay_players = Vec::new();
        for &seat_num in &sorted_seats {
            let (name, chips) = &seat_map[&seat_num];
            let idx = replay_players.len();
            player_name_to_idx.insert(name.clone(), idx);

            let is_hero = hero_name.as_deref() == Some(name.as_str());
            let cards = if is_hero {
                hero_cards.map(|c| [c[0].to_string(), c[1].to_string()])
            } else {
                None
            };

            replay_players.push(ReplayPlayer {
                name: name.clone(),
                stack: *chips,
                hole_cards: cards,
            });
        }

        // Map string actor actions to player index actions
        let mut replay_actions = Vec::new();
        for (name, action, street) in actions {
            if let Some(&p_idx) = player_name_to_idx.get(&name) {
                replay_actions.push(ReplayActionStep {
                    street,
                    player_idx: p_idx,
                    action,
                });
            }
        }

        Ok(ParsedHandHistory {
            hand_id,
            tournament_id,
            sb_amount,
            bb_amount,
            ante_amount,
            hero_name,
            hero_cards,
            button_seat,
            players: replay_players,
            actions: replay_actions,
            board_cards,
        })
    }

    fn extract_board_cards(line: &str, board: &mut Vec<Card>) {
        // e.g.: "*** FLOP *** [Kh 7d 2c]"
        if let (Some(open_b), Some(close_b)) = (line.find('['), line.find(']')) {
            let cards_str = &line[open_b + 1..close_b];
            for card_str in cards_str.split_whitespace() {
                if let Ok(c) = Card::from_str_exact(card_str) {
                    board.push(c);
                }
            }
        }
    }

    fn extract_turn_river_card(line: &str, board: &mut Vec<Card>) {
        // e.g.: "*** TURN *** [Kh 7d 2c] [4s]"
        if let (Some(last_open), Some(last_close)) = (line.rfind('['), line.rfind(']')) {
            if last_open < last_close {
                let card_str = line[last_open + 1..last_close].trim();
                if let Ok(c) = Card::from_str_exact(card_str) {
                    board.push(c);
                }
            }
        }
    }

    fn parse_action(text: &str) -> Option<PlayerAction> {
        let parts: Vec<&str> = text.split_whitespace().collect();
        if parts.is_empty() {
            return None;
        }

        match parts[0] {
            "folds" => Some(PlayerAction::Fold),
            "checks" => Some(PlayerAction::Check),
            "calls" => {
                let amt = parts.get(1).and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.0);
                Some(PlayerAction::Call(amt))
            }
            "bets" => {
                let amt = parts.get(1).and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.0);
                if text.contains("and is all-in") {
                    Some(PlayerAction::AllIn(amt))
                } else {
                    Some(PlayerAction::Bet(amt))
                }
            }
            "raises" => {
                // e.g. "raises 40 to 60" or "raises 40 to 60 and is all-in"
                let amt = if let Some(to_pos) = parts.iter().position(|&p| p == "to") {
                    parts.get(to_pos + 1).and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.0)
                } else {
                    parts.get(1).and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.0)
                };

                if text.contains("and is all-in") {
                    Some(PlayerAction::AllIn(amt))
                } else {
                    Some(PlayerAction::Raise(amt))
                }
            }
            _ => None,
        }
    }
}
