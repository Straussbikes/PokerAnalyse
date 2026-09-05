use futures_util::StreamExt;
use poker_analyse::api_bridge::server::{ServerMessage, WsEventServer};
use poker_analyse::sim::ev::{ActionEv, CandidateAction, DecisionReport};
use std::time::Instant;
use tokio_tungstenite::connect_async;

#[tokio::test]
async fn test_ws_event_server_1000_reports_streaming_latency() {
    // 1. Bind WebSocket server to ephemeral loopback port
    let server = WsEventServer::start("127.0.0.1:0", 4096)
        .await
        .expect("Failed to start WsEventServer");

    let ws_url = server.ws_url();

    // 2. Connect Tungstenite client
    let (ws_stream, _) = connect_async(&ws_url)
        .await
        .expect("Failed to connect client to WsEventServer");

    let (_, mut reader) = ws_stream.split();

    // Spawn client reader task to verify and count received reports
    let client_task = tokio::spawn(async move {
        let mut count = 0;
        while let Some(Ok(msg)) = reader.next().await {
            if let Ok(text) = msg.to_text() {
                if let Ok(ServerMessage::Decision(report)) = ServerMessage::from_json(text) {
                    assert_eq!(report.street, "Turn");
                    assert_eq!(report.hero_cards[0], "Ah");
                    count += 1;
                    if count == 1000 {
                        break;
                    }
                }
            }
        }
        count
    });

    // Give connection handshake a moment to settle
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    // 3. Build a standard dummy DecisionReport
    let sample_report = DecisionReport {
        hand_id: 999123,
        street: "Turn".to_string(),
        current_pot: 1200.0,
        call_cost: 300.0,
        hero_stack: 4500.0,
        hero_cards: ["Ah".to_string(), "Kd".to_string()],
        board_cards: vec!["Qh".to_string(), "Jh".to_string(), "2c".to_string(), "Ts".to_string()],
        hero_equity: 0.68,
        pot_odds: 0.20,
        action_evaluations: vec![
            ActionEv {
                action: CandidateAction::Fold,
                chip_ev: 0.0,
                dollar_ev: Some(0.0),
                break_even_equity: 0.0,
                description: "Surrender".to_string(),
            },
            ActionEv {
                action: CandidateAction::Call(300.0),
                chip_ev: 516.0,
                dollar_ev: Some(516.0),
                break_even_equity: 0.20,
                description: "Call bet".to_string(),
            },
        ],
        recommended_action: CandidateAction::Call(300.0),
        reasoning: "Drawing to nuts with monster equity".to_string(),
    };

    // 4. Stream 1,000 serialized decision reports over localhost and measure latency
    let start_time = Instant::now();
    for _ in 0..1000 {
        server.broadcast_decision(sample_report.clone());
    }

    let received_count = client_task
        .await
        .expect("Client receive task panicked");

    let elapsed = start_time.elapsed();
    let per_report_us = elapsed.as_micros() as f64 / 1000.0;
    println!(
        "Streamed and received {} reports in {:?} ({:.2} µs/report, < 1ms requirement)",
        received_count, elapsed, per_report_us
    );

    assert_eq!(received_count, 1000);
    // Requirement from Roadmap: latency < 1ms (1000 µs) per report
    assert!(
        per_report_us < 1000.0,
        "Streaming latency exceeded 1ms: {:.2} µs/report",
        per_report_us
    );

    server.stop();
}
