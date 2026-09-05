use crate::api_bridge::hand_parser::HandHistoryParser;
use crate::api_bridge::server::WsEventServer;
use crate::ingestion::mock_replay::MockReplayAdapter;
use notify::{Event, EventKind, RecursiveMode, Watcher};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

/// Configuration for watching hand history directories.
pub struct WatcherConfig {
    pub watch_dir: PathBuf,
    pub default_payouts: Vec<f64>,
}

/// Zero-polling OS filesystem watcher that detects tournament hand file changes,
/// parses newly appended hands, drives the state machine, and streams events via WsEventServer.
pub struct HandHistoryWatcher {
    _watcher: notify::RecommendedWatcher,
    shutdown: Arc<AtomicBool>,
}

impl HandHistoryWatcher {
    /// Starts watching a directory for newly created or modified hand history files.
    ///
    /// When a hand is parsed, its events and Hero decision reports are forwarded to `ws_server`.
    pub fn start<P: AsRef<Path>>(
        dir_path: P,
        default_payouts: Vec<f64>,
        ws_server: Option<Arc<WsEventServer>>,
    ) -> Result<(Self, mpsc::UnboundedReceiver<u64>), notify::Error> {
        let watch_dir = dir_path.as_ref().to_path_buf();
        let (tx_hand_ids, rx_hand_ids) = mpsc::unbounded_channel();
        let shutdown = Arc::new(AtomicBool::new(false));

        // Track byte offset per file to read only newly appended content
        let file_offsets: Arc<Mutex<std::collections::HashMap<PathBuf, u64>>> =
            Arc::new(Mutex::new(std::collections::HashMap::new()));

        let (notify_tx, mut notify_rx) = mpsc::unbounded_channel::<PathBuf>();

        // Build OS watcher
        let mut watcher = notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
            if let Ok(event) = res {
                match event.kind {
                    EventKind::Create(_) | EventKind::Modify(_) => {
                        for path in event.paths {
                            let _ = notify_tx.send(path);
                        }
                    }
                    _ => {}
                }
            }
        })?;

        watcher.watch(&watch_dir, RecursiveMode::NonRecursive)?;

        let srv_ref = ws_server;
        let shutdown_loop = Arc::clone(&shutdown);

        // Background worker consuming file change notifications
        tokio::spawn(async move {
            while let Some(path) = notify_rx.recv().await {
                if shutdown_loop.load(Ordering::Relaxed) {
                    break;
                }

                if !path.is_file() {
                    continue;
                }

                // Read appended bytes
                let mut offsets = file_offsets.lock().unwrap();
                let last_offset = offsets.entry(path.clone()).or_insert(0);

                if let Ok(mut file) = File::open(&path) {
                    if let Ok(meta) = file.metadata() {
                        let len = meta.len();
                        if len > *last_offset
                            && file.seek(SeekFrom::Start(*last_offset)).is_ok()
                        {
                            let mut buffer = String::new();
                            if file.read_to_string(&mut buffer).is_ok() {
                                    *last_offset = len;
                                    drop(offsets);

                                    // Parse potential hands in the newly appended content
                                    // A file can have one or multiple hand histories separated by blank lines or headers
                                    let hand_blocks = split_hand_blocks(&buffer);
                                    for block in hand_blocks {
                                        if let Ok(parsed) = HandHistoryParser::parse(&block) {
                                            let hand_id = parsed.hand_id;
                                            let scenario = parsed.to_replay_scenario(Some(default_payouts.clone()));

                                            if let Ok(replay_res) = MockReplayAdapter::replay(&scenario) {
                                                if let Some(ref srv) = srv_ref {
                                                    for event in replay_res.events {
                                                        srv.broadcast_event(event);
                                                    }
                                                    for decision in replay_res.decision_reports {
                                                        srv.broadcast_decision(decision);
                                                    }
                                                }

                                                let _ = tx_hand_ids.send(hand_id);
                                            }
                                        }
                                    }
                                    continue;
                                }
                            }
                    }
                }
            }
        });

        Ok((
            HandHistoryWatcher {
                _watcher: watcher,
                shutdown,
            },
            rx_hand_ids,
        ))
    }

    /// Stops the watcher.
    pub fn stop(&self) {
        self.shutdown.store(true, Ordering::Relaxed);
    }
}

/// Helper to split multi-hand text into individual hand blocks.
pub fn split_hand_blocks(text: &str) -> Vec<String> {
    let mut blocks = Vec::new();
    let mut current_block = String::new();

    for line in text.lines() {
        if (line.starts_with("PokerStars Hand #") || line.starts_with("Hand #"))
            && !current_block.trim().is_empty()
        {
            blocks.push(current_block.clone());
            current_block.clear();
        }
        current_block.push_str(line);
        current_block.push('\n');
    }

    if !current_block.trim().is_empty() {
        blocks.push(current_block);
    }

    blocks
}
