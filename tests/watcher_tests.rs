use futures_util::StreamExt;
use poker_analyse::api_bridge::server::{ServerMessage, WsEventServer};
use poker_analyse::api_bridge::watcher::HandHistoryWatcher;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::sync::Arc;
use tokio_tungstenite::connect_async;

const TEST_HAND_1: &str = r#"
PokerStars Hand #900100200: Tournament #777888999, $50+$5 USD Hold'em No Limit - Level I (25/50) - 2026/09/05
Table '777888999 1' 9-max Seat #1 is the button
Seat 1: ButtonUser (5000 in chips)
Seat 2: SmallBlindGuy (5000 in chips)
Seat 3: BigBlindGuy (5000 in chips)
Seat 4: Hero (5000 in chips)
SmallBlindGuy: posts small blind 25
BigBlindGuy: posts big blind 50
*** HOLE CARDS ***
Dealt to Hero [Ac As]
Hero: raises 100 to 150
ButtonUser: folds
SmallBlindGuy: folds
BigBlindGuy: calls 100
*** FLOP *** [Kd 7h 2c]
BigBlindGuy: checks
Hero: bets 200
BigBlindGuy: folds
*** SUMMARY ***
Total pot 325 | Rake 0
"#;

#[tokio::test]
async fn test_hand_history_watcher_and_streaming_integration() {
    // 1. Create a unique temporary directory for watching hand logs
    let temp_dir = std::env::temp_dir().join(format!("poker_watch_test_{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).expect("Failed to create temporary watch dir");

    // 2. Start WebSocket server
    let ws_server = Arc::new(
        WsEventServer::start("127.0.0.1:0", 1024)
            .await
            .expect("Failed to start WsEventServer"),
    );
    let ws_url = ws_server.ws_url();

    // 3. Connect a WebSocket client to verify real-time event & decision reception
    let (ws_stream, _) = connect_async(&ws_url)
        .await
        .expect("Failed to connect WS client");
    let (_, mut reader) = ws_stream.split();

    let client_collector = tokio::spawn(async move {
        let mut received_decisions = Vec::new();
        let mut received_events = Vec::new();

        while let Some(Ok(msg)) = reader.next().await {
            if let Ok(text) = msg.to_text() {
                if let Ok(parsed) = ServerMessage::from_json(text) {
                    match parsed {
                        ServerMessage::Decision(d) => received_decisions.push(d),
                        ServerMessage::Event(e) => received_events.push(e),
                        ServerMessage::Ping => {}
                    }
                }
            }
            // Stop once we receive decisions generated for Hero's turns
            if !received_decisions.is_empty() {
                break;
            }
        }
        (received_decisions, received_events)
    });

    // 4. Start HandHistoryWatcher on the temp directory
    let (watcher, mut hand_id_rx) = HandHistoryWatcher::start(
        &temp_dir,
        vec![100.0, 50.0],
        Some(Arc::clone(&ws_server)),
    )
    .expect("Failed to start HandHistoryWatcher");

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // 5. Append hand text to a watched file
    let log_file_path = temp_dir.join("tournament_hands.txt");
    {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_file_path)
            .expect("Failed to open log file for write");
        file.write_all(TEST_HAND_1.as_bytes())
            .expect("Failed to write hand history");
        file.flush().expect("Failed to flush log file");
    }

    // 6. Verify watcher notified on the parsed hand id
    let parsed_hand_id = tokio::time::timeout(tokio::time::Duration::from_secs(3), hand_id_rx.recv())
        .await
        .expect("Watcher timed out detecting hand")
        .expect("Channel closed without receiving hand id");

    assert_eq!(parsed_hand_id, 900100200);

    // 7. Verify the client received decisions and game events over WebSocket
    let (decisions, events) = tokio::time::timeout(tokio::time::Duration::from_secs(3), client_collector)
        .await
        .expect("Client collector timed out")
        .expect("Client collector task panicked");

    assert!(!decisions.is_empty(), "Expected at least one Hero DecisionReport over WebSocket");
    assert!(!events.is_empty(), "Expected GameEvents over WebSocket");

    let first_decision = &decisions[0];
    assert_eq!(first_decision.hero_cards[0], "Ac");
    assert_eq!(first_decision.hero_cards[1], "As");

    // Clean up
    watcher.stop();
    ws_server.stop();
    let _ = fs::remove_dir_all(&temp_dir);
}
