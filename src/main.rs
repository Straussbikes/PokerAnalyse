use poker_analyse::api_bridge::server::WsEventServer;
use poker_analyse::api_bridge::watcher::HandHistoryWatcher;
use poker_analyse::audit::SessionAuditor;
use poker_analyse::ingestion::{MockReplayAdapter, ReplayHandScenario};
use poker_analyse::sim::ev::DecisionReport;
use std::env;
use std::path::PathBuf;
use std::sync::Arc;

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    let command = args.get(1).map(|s| s.as_str()).unwrap_or("help");

    match command {
        "audit" => {
            if args.len() < 3 {
                eprintln!("Usage: poker_cli audit <hand_history_file> [--json]");
                std::process::exit(1);
            }
            let file_path = &args[2];
            let as_json = args.iter().any(|a| a == "--json");

            println!("Analyzing tournament session from '{}'...", file_path);
            match SessionAuditor::audit_file(file_path, None) {
                Ok(report) => {
                    if as_json {
                        println!("{}", report.to_json().unwrap());
                    } else {
                        println!("{}", report.format_terminal_table());
                    }
                }
                Err(err) => {
                    eprintln!("Audit error: {err}");
                    std::process::exit(1);
                }
            }
        }

        "serve" => {
            let port = args
                .iter()
                .position(|a| a == "--port")
                .and_then(|i| args.get(i + 1))
                .and_then(|p| p.parse::<u16>().ok())
                .unwrap_or(9001);

            let watch_dir = args
                .iter()
                .position(|a| a == "--watch")
                .and_then(|i| args.get(i + 1))
                .map(PathBuf::from);

            let addr = format!("127.0.0.1:{port}");
            println!("Starting WsEventServer on ws://{addr}...");
            let server = match WsEventServer::start(&addr, 1024).await {
                Ok(s) => Arc::new(s),
                Err(e) => {
                    eprintln!("Failed to bind server to {addr}: {e}");
                    std::process::exit(1);
                }
            };
            println!("Server listening at {}", server.ws_url());

            if let Some(dir) = watch_dir {
                println!("Watching directory for hand histories: {:?}", dir);
                let (_watcher, mut rx) = HandHistoryWatcher::start(
                    &dir,
                    vec![100.0, 50.0],
                    Some(Arc::clone(&server)),
                )
                .expect("Failed to start watcher");

                tokio::spawn(async move {
                    while let Some(hand_id) = rx.recv().await {
                        println!("[Watcher] Ingested and streamed hand #{hand_id}");
                    }
                });
            }

            println!("Press Ctrl+C to stop.");
            tokio::signal::ctrl_c().await.ok();
            server.stop();
            println!("Server shutdown gracefully.");
        }

        "demo" => {
            let port = args
                .iter()
                .position(|a| a == "--port")
                .and_then(|i| args.get(i + 1))
                .and_then(|p| p.parse::<u16>().ok())
                .unwrap_or(9001);

            let addr = format!("127.0.0.1:{port}");
            let server = match WsEventServer::start(&addr, 1024).await {
                Ok(s) => Arc::new(s),
                Err(e) => {
                    eprintln!("Failed to start server on {addr}: {e}");
                    std::process::exit(1);
                }
            };
            println!("============================================================");
            println!("   PokerAnalyse: Live Streaming & Web HUD Server Active     ");
            println!("============================================================");
            println!("* Web Dashboard:  http://127.0.0.1:{port}/");
            println!("* WebSocket URL:  ws://127.0.0.1:{port}/");
            println!("--> Open http://127.0.0.1:{port}/ in your browser!");
            println!("Broadcasting live tournament hands every 4 seconds... (Ctrl+C to quit)");
            println!("------------------------------------------------------------");

            let srv_clone = Arc::clone(&server);
            tokio::spawn(async move {
                let scenarios_json = [
                    // Hand 1: Pocket Aces (Overpair)
                    r#"{
                        "hand_id": 7001001,
                        "button_idx": 0,
                        "sb_amount": 25.0,
                        "bb_amount": 50.0,
                        "players": [
                            { "name": "Hero", "stack": 5000.0, "hole_cards": ["Ac", "As"] },
                            { "name": "Villain1", "stack": 4800.0, "hole_cards": null },
                            { "name": "Villain2", "stack": 5200.0, "hole_cards": null }
                        ],
                        "board_cards": ["Kd", "7h", "2c", "Js", "4d"],
                        "action_script": [
                            { "street": "Preflop", "player_idx": 0, "action": { "type": "Raise", "amount": 150.0 } },
                            { "street": "Preflop", "player_idx": 1, "action": { "type": "Fold" } },
                            { "street": "Preflop", "player_idx": 2, "action": { "type": "Call", "amount": 100.0 } },
                            { "street": "Flop", "player_idx": 2, "action": { "type": "Check" } },
                            { "street": "Flop", "player_idx": 0, "action": { "type": "Bet", "amount": 200.0 } },
                            { "street": "Flop", "player_idx": 2, "action": { "type": "Call", "amount": 200.0 } },
                            { "street": "Turn", "player_idx": 2, "action": { "type": "Check" } },
                            { "street": "Turn", "player_idx": 0, "action": { "type": "Bet", "amount": 450.0 } },
                            { "street": "Turn", "player_idx": 2, "action": { "type": "Call", "amount": 450.0 } },
                            { "street": "River", "player_idx": 2, "action": { "type": "Check" } },
                            { "street": "River", "player_idx": 0, "action": { "type": "Bet", "amount": 900.0 } },
                            { "street": "River", "player_idx": 2, "action": { "type": "Fold" } }
                        ],
                        "payouts": [100.0, 50.0],
                        "hero_idx": 0
                    }"#,
                    // Hand 2: Royal Flush / Nut Broadway Straight
                    r#"{
                        "hand_id": 7001002,
                        "button_idx": 0,
                        "sb_amount": 25.0,
                        "bb_amount": 50.0,
                        "players": [
                            { "name": "Hero", "stack": 5500.0, "hole_cards": ["Ah", "Kh"] },
                            { "name": "Villain1", "stack": 4600.0, "hole_cards": null },
                            { "name": "Villain2", "stack": 4900.0, "hole_cards": null }
                        ],
                        "board_cards": ["Qh", "Jh", "Th", "2s", "3d"],
                        "action_script": [
                            { "street": "Preflop", "player_idx": 0, "action": { "type": "Raise", "amount": 150.0 } },
                            { "street": "Preflop", "player_idx": 1, "action": { "type": "Fold" } },
                            { "street": "Preflop", "player_idx": 2, "action": { "type": "Call", "amount": 100.0 } },
                            { "street": "Flop", "player_idx": 2, "action": { "type": "Check" } },
                            { "street": "Flop", "player_idx": 0, "action": { "type": "Bet", "amount": 225.0 } },
                            { "street": "Flop", "player_idx": 2, "action": { "type": "Call", "amount": 225.0 } },
                            { "street": "Turn", "player_idx": 2, "action": { "type": "Check" } },
                            { "street": "Turn", "player_idx": 0, "action": { "type": "Bet", "amount": 550.0 } },
                            { "street": "Turn", "player_idx": 2, "action": { "type": "Call", "amount": 550.0 } },
                            { "street": "River", "player_idx": 2, "action": { "type": "Check" } },
                            { "street": "River", "player_idx": 0, "action": { "type": "Bet", "amount": 1200.0 } },
                            { "street": "River", "player_idx": 2, "action": { "type": "Fold" } }
                        ],
                        "payouts": [100.0, 50.0],
                        "hero_idx": 0
                    }"#,
                    // Hand 3: Flopped Set of Kings
                    r#"{
                        "hand_id": 7001003,
                        "button_idx": 0,
                        "sb_amount": 50.0,
                        "bb_amount": 100.0,
                        "players": [
                            { "name": "Hero", "stack": 5200.0, "hole_cards": ["Kh", "Kd"] },
                            { "name": "Villain1", "stack": 4300.0, "hole_cards": null },
                            { "name": "Villain2", "stack": 5500.0, "hole_cards": null }
                        ],
                        "board_cards": ["Ks", "9c", "4d", "2h", "Jc"],
                        "action_script": [
                            { "street": "Preflop", "player_idx": 0, "action": { "type": "Raise", "amount": 250.0 } },
                            { "street": "Preflop", "player_idx": 1, "action": { "type": "Fold" } },
                            { "street": "Preflop", "player_idx": 2, "action": { "type": "Call", "amount": 150.0 } },
                            { "street": "Flop", "player_idx": 2, "action": { "type": "Check" } },
                            { "street": "Flop", "player_idx": 0, "action": { "type": "Bet", "amount": 350.0 } },
                            { "street": "Flop", "player_idx": 2, "action": { "type": "Call", "amount": 350.0 } },
                            { "street": "Turn", "player_idx": 2, "action": { "type": "Check" } },
                            { "street": "Turn", "player_idx": 0, "action": { "type": "Bet", "amount": 750.0 } },
                            { "street": "Turn", "player_idx": 2, "action": { "type": "Call", "amount": 750.0 } },
                            { "street": "River", "player_idx": 2, "action": { "type": "Check" } },
                            { "street": "River", "player_idx": 0, "action": { "type": "Bet", "amount": 1500.0 } },
                            { "street": "River", "player_idx": 2, "action": { "type": "Fold" } }
                        ],
                        "payouts": [100.0, 50.0],
                        "hero_idx": 0
                    }"#,
                    // Hand 4: Suited Connectors Straight
                    r#"{
                        "hand_id": 7001004,
                        "button_idx": 0,
                        "sb_amount": 50.0,
                        "bb_amount": 100.0,
                        "players": [
                            { "name": "Hero", "stack": 6100.0, "hole_cards": ["9s", "8s"] },
                            { "name": "Villain1", "stack": 3800.0, "hole_cards": null },
                            { "name": "Villain2", "stack": 5100.0, "hole_cards": null }
                        ],
                        "board_cards": ["7s", "6c", "2d", "5h", "Kd"],
                        "action_script": [
                            { "street": "Preflop", "player_idx": 0, "action": { "type": "Raise", "amount": 225.0 } },
                            { "street": "Preflop", "player_idx": 1, "action": { "type": "Fold" } },
                            { "street": "Preflop", "player_idx": 2, "action": { "type": "Call", "amount": 125.0 } },
                            { "street": "Flop", "player_idx": 2, "action": { "type": "Check" } },
                            { "street": "Flop", "player_idx": 0, "action": { "type": "Bet", "amount": 275.0 } },
                            { "street": "Flop", "player_idx": 2, "action": { "type": "Call", "amount": 275.0 } },
                            { "street": "Turn", "player_idx": 2, "action": { "type": "Check" } },
                            { "street": "Turn", "player_idx": 0, "action": { "type": "Bet", "amount": 650.0 } },
                            { "street": "Turn", "player_idx": 2, "action": { "type": "Call", "amount": 650.0 } },
                            { "street": "River", "player_idx": 2, "action": { "type": "Check" } },
                            { "street": "River", "player_idx": 0, "action": { "type": "Bet", "amount": 1400.0 } },
                            { "street": "River", "player_idx": 2, "action": { "type": "Call", "amount": 1400.0 } }
                        ],
                        "payouts": [100.0, 50.0],
                        "hero_idx": 0
                    }"#
                ];

                let mut all_decisions: Vec<DecisionReport> = Vec::new();
                for json_str in &scenarios_json {
                    if let Ok(scenario) = ReplayHandScenario::from_json(json_str) {
                        if let Ok(replay_res) = MockReplayAdapter::replay(&scenario) {
                            all_decisions.extend(replay_res.decision_reports);
                        }
                    }
                }

                println!("[Demo Engine] Pre-calculated {} interactive decision nodes across 4 tournament hands.", all_decisions.len());
                println!("[Demo Engine] Mode: MANUAL STEPPING (Controlled via Web UI or Spacebar / Arrow Keys).");

                if all_decisions.is_empty() {
                    return;
                }

                let mut cursor = 0;
                let mut cmd_rx = srv_clone.subscribe_commands();

                // Broadcast initial decision so connected client immediately sees Hand 1 Preflop
                srv_clone.broadcast_decision(all_decisions[cursor].clone());

                while let Ok(cmd_str) = cmd_rx.recv().await {
                    let cmd_lower = cmd_str.to_lowercase();
                    if cmd_lower.contains("next") || cmd_lower.contains("step") {
                        cursor = (cursor + 1) % all_decisions.len();
                        let dec = &all_decisions[cursor];
                        println!(
                            "  [Step {}/{}] Hand #{} [{}] Hero: [{} {}] Board: [{}] | Rec: {:?}",
                            cursor + 1,
                            all_decisions.len(),
                            dec.hand_id,
                            dec.street,
                            dec.hero_cards[0],
                            dec.hero_cards[1],
                            dec.board_cards.join(" "),
                            dec.recommended_action
                        );
                        srv_clone.broadcast_decision(dec.clone());
                    } else if cmd_lower.contains("prev") {
                        if cursor == 0 {
                            cursor = all_decisions.len() - 1;
                        } else {
                            cursor -= 1;
                        }
                        let dec = &all_decisions[cursor];
                        println!(
                            "  [Step {}/{}] Hand #{} [{}] Hero: [{} {}] Board: [{}] | Rec: {:?}",
                            cursor + 1,
                            all_decisions.len(),
                            dec.hand_id,
                            dec.street,
                            dec.hero_cards[0],
                            dec.hero_cards[1],
                            dec.board_cards.join(" "),
                            dec.recommended_action
                        );
                        srv_clone.broadcast_decision(dec.clone());
                    } else if cmd_lower.contains("init") || cmd_lower.contains("connect") {
                        srv_clone.broadcast_decision(all_decisions[cursor].clone());
                    }
                }
            });

            tokio::signal::ctrl_c().await.ok();
            server.stop();
        }

        _ => {
            println!("============================================================");
            println!("   PokerAnalyse: Tournament Poker Decision & Analytics Engine");
            println!("============================================================");
            println!("Available subcommands:");
            println!("  audit <file> [--json]          Run session audit on tournament log file");
            println!("  serve [--port <port>] [--watch <dir>] Start live streaming WebSocket server");
            println!("  demo [--port <port>]           Run live simulated hands for dashboard demo");
            println!("\nExamples:");
            println!("  poker_cli audit sample_tournament.txt");
            println!("  poker_cli serve --port 9001 --watch ./hand_history");
            println!("  poker_cli demo --port 9001");
            println!("============================================================");
        }
    }
}
