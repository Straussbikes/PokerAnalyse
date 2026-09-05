use crate::orchestrator::event_bus::GameEvent;
use crate::sim::ev::DecisionReport;
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::broadcast;
use tokio_tungstenite::tungstenite::Message;

/// Envelope for all messages streamed across WebSocket / IPC to connected clients.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "msg_type", content = "payload")]
pub enum ServerMessage {
    /// Domain state transition or action event.
    Event(GameEvent),
    /// Solved EV decision report for Hero.
    Decision(DecisionReport),
    /// Heartbeat ping message.
    Ping,
}

impl ServerMessage {
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    pub fn from_json(s: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(s)
    }
}

/// High-performance asynchronous WebSocket streaming server.
pub struct WsEventServer {
    local_addr: SocketAddr,
    sender: broadcast::Sender<ServerMessage>,
    cmd_sender: broadcast::Sender<String>,
    shutdown: Arc<AtomicBool>,
}

impl WsEventServer {
    /// Spawns an event server listening on the specified socket address (e.g. "127.0.0.1:0").
    ///
    /// Returns the server handle along with the actual bound SocketAddr.
    pub async fn start(addr: &str, channel_capacity: usize) -> Result<Self, std::io::Error> {
        let listener = TcpListener::bind(addr).await?;
        let local_addr = listener.local_addr()?;
        let (sender, _) = broadcast::channel(channel_capacity);
        let (cmd_sender, _) = broadcast::channel(64);
        let shutdown = Arc::new(AtomicBool::new(false));

        let srv_sender = sender.clone();
        let srv_cmd_sender = cmd_sender.clone();
        let srv_shutdown = Arc::clone(&shutdown);

        tokio::spawn(async move {
            while !srv_shutdown.load(Ordering::Relaxed) {
                match listener.accept().await {
                    Ok((stream, peer_addr)) => {
                        let rx = srv_sender.subscribe();
                        let cmd_tx = srv_cmd_sender.clone();
                        tokio::spawn(handle_client(stream, peer_addr, rx, cmd_tx));
                    }
                    Err(_) => {
                        if srv_shutdown.load(Ordering::Relaxed) {
                            break;
                        }
                    }
                }
            }
        });

        Ok(WsEventServer {
            local_addr,
            sender,
            cmd_sender,
            shutdown,
        })
    }

    /// Bound local address of the running server.
    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    /// Returns a WebSocket connection URL string, e.g. "ws://127.0.0.1:54321".
    pub fn ws_url(&self) -> String {
        format!("ws://{}", self.local_addr)
    }

    /// Non-blocking broadcast of a GameEvent to all connected clients.
    pub fn broadcast_event(&self, event: GameEvent) -> usize {
        self.sender.send(ServerMessage::Event(event)).unwrap_or(0)
    }

    /// Non-blocking broadcast of a DecisionReport to all connected clients.
    pub fn broadcast_decision(&self, decision: DecisionReport) -> usize {
        self.sender.send(ServerMessage::Decision(decision)).unwrap_or(0)
    }

    /// Direct broadcast of any ServerMessage.
    pub fn broadcast(&self, msg: ServerMessage) -> usize {
        self.sender.send(msg).unwrap_or(0)
    }

    /// Returns a new subscriber receiver to tap directly into the internal broadcast bus.
    pub fn subscribe(&self) -> broadcast::Receiver<ServerMessage> {
        self.sender.subscribe()
    }

    /// Returns a receiver to listen for commands sent from connected WebSocket clients.
    pub fn subscribe_commands(&self) -> broadcast::Receiver<String> {
        self.cmd_sender.subscribe()
    }

    /// Signals the server to stop accepting new connections.
    pub fn stop(&self) {
        self.shutdown.store(true, Ordering::Relaxed);
    }
}

use tokio::io::{AsyncReadExt, AsyncWriteExt};

async fn handle_client(
    mut stream: TcpStream,
    peer: SocketAddr,
    mut rx: broadcast::Receiver<ServerMessage>,
    cmd_tx: broadcast::Sender<String>,
) {
    let mut peek_buf = [0u8; 1024];
    let n = stream.peek(&mut peek_buf).await.unwrap_or(0);
    let peek_str = String::from_utf8_lossy(&peek_buf[..n]);

    // If it's a standard HTTP GET (not a WebSocket upgrade), serve dashboard UI
    if peek_str.starts_with("GET ") && !peek_str.to_lowercase().contains("upgrade: websocket") {
        let mut discard = vec![0u8; n];
        let _ = stream.read_exact(&mut discard).await;

        let path = peek_str
            .lines()
            .next()
            .and_then(|l| l.split_whitespace().nth(1))
            .unwrap_or("/");

        let (content_type, file_content) = match path {
            "/dashboard.css" => (
                "text/css; charset=utf-8",
                include_str!("../../crates/dashboard-ui/dashboard.css"),
            ),
            "/dashboard.js" => (
                "application/javascript; charset=utf-8",
                include_str!("../../crates/dashboard-ui/dashboard.js"),
            ),
            _ => (
                "text/html; charset=utf-8",
                include_str!("../../crates/dashboard-ui/index.html"),
            ),
        };

        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: {}\r\nContent-Length: {}\r\nAccess-Control-Allow-Origin: *\r\nConnection: close\r\n\r\n{}",
            content_type,
            file_content.len(),
            file_content
        );

        let _ = stream.write_all(response.as_bytes()).await;
        let _ = stream.flush().await;
        return;
    }

    println!("[Server] WebSocket client connected from {}", peer);

    let ws_stream = match tokio_tungstenite::accept_async(stream).await {
        Ok(ws) => ws,
        Err(_) => return,
    };

    let (mut ws_sender, mut ws_receiver) = ws_stream.split();

    // Outbound forwarder task: reads from broadcast channel and writes to WebSocket sink
    let forward_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            if let Ok(json) = msg.to_json() {
                if ws_sender.send(Message::Text(json.into())).await.is_err() {
                    break;
                }
            }
        }
    });

    // Inbound receiver task: handles client pings / commands / disconnects
    let recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = ws_receiver.next().await {
            if msg.is_close() {
                break;
            }
            if let Message::Text(txt) = msg {
                let _ = cmd_tx.send(txt.to_string());
            }
        }
    });

    tokio::select! {
        _ = forward_task => {},
        _ = recv_task => {},
    };
    println!("[Server] WebSocket client disconnected from {}", peer);
}
