# Project Implementation Roadmap

## Milestone 1: Core Algebra & Bitboard Evaluator
- [x] **M1.1: Card Primitive Implementation**
  - Implement 32-bit packed card encoder (`rank`, `suit`, `prime`, `bitmask`).
  - Implement string parser (`"Ah"`, `"Ks"`, etc.) with zero heap allocations.
  - Implement `u64` deck bitmask operations (`add_card`, `has_card`, `dead_cards`).
  - *Validation:* Unit test with all 52 cards and check bit layout integrity.

- [x] **M1.2: 5-Card Cactus-Kev Evaluator**
  - Generate/embed static lookup tables (`flushes`, `unique5`, prime product hash map).
  - Implement $O(1)$ evaluation returning score in `[1, 7462]`.
  - *Validation:* Royal Flush == 1, 7-5-4-3-2 unsuited == 7462.

- [x] **M1.3: 7-Card Evaluator**
  - Unroll static $\binom{7}{5} = 21$ combinations array.
  - Evaluate 7-card hands by finding the minimal 5-card score.
  - *Validation:* Single-core benchmark must achieve $\ge 15\text{M}$ hands/sec. (Achieved 46.01M hands/sec)

---

## Milestone 2: Tournament Engine (ICM & Preflop Ranges)
- [x] **M2.1: Malmuth-Harville ICM Calculator**
  - Implement dynamic programming bitmask algorithm for $N \le 10$ players.
  - Output monetary equity vector given stacks and payout structures.
  - *Validation:* 4-handed bubble test vector: Stacks `[5000, 2500, 1500, 1000]`, Payouts `[0.50, 0.30, 0.20]`. Result must match within $\epsilon < 0.0005$. (Validated: diff < 0.00001, 7.79 µs/eval)

- [x] **M2.2: Risk Premium & Range Tightener**
  - Calculate marginal $\Delta \$EV_{\text{loss}} / \Delta \$EV_{\text{win}}$ per opponent.
  - Map ChipEV thresholds to adjusted calling/shoving ranges.

---

## Milestone 3: Range Representation & Opponent Modeling
- [x] **M3.1: 1326-Combo Weight Matrix**
  - Implement array of 1326 float weights representing hand distributions.
  - Implement fast dead-card blocker application (`weight = 0.0` if card blocked). (Validated: 0.56 µs/pass, 32/32 tests passing)

- [x] **M3.2: Bayesian Tendency Filter**
  - Implement updating function: $W'_c = W_c \times P(\text{Action} \mid \text{HandStrengthBucket}, \text{Profile})$.
  - Categorize combos into Air, Marginal, Strong based on board texture. (Validated: 14.9 µs/update, 37/37 tests passing)

---

## Milestone 4: Fast Multiway Monte Carlo & Decision Engine
- [x] **M4.1: High-Speed Multiway Simulator**
  - Non-allocating Monte Carlo runout generator using Xorshift / PCG PRNG.
  - Calculate Hero equity against 1 to 3 active opponent range weightings.
  - *Validation:* 10,000 samples executed in $<20\text{ms}$. (Validated: 10k samples in ~2.9ms)

- [x] **M4.2: Best-Response EV Engine**
  - Calculate EV for candidate actions: `Fold`, `Call`, `Bet 33%`, `Bet 75%`, `All-In`.
  - Output structured JSON decision report.
  - *Validation:* Validated tournament $EV and ChipEV with JSON serialization tests.

---

## Phase 2: Application Layer & State Machine
- [x] **2.1: Multiway Side Pot Calculator**
  - Exact multiway pot leveling, player eligibility sets, and uncalled excess return.
  - *Validation:* Triple all-in asymmetric side pot isolation tests.
- [x] **2.2: Hand State Machine & Event Bus**
  - Full round transitions (`Preflop` -> `Flop` -> `Turn` -> `River` -> `Showdown`).
  - Active bettor tracking, dead-card board updates, uncontested fold handling, and 7-card showdown pot payouts.
  - Decoupled `EventBus` emitting structured `GameEvent` stream.

---

## Phase 3: Ingestion & Mock Replay Adapter
- [x] **3.1: Mock Replay Adapter**
  - Consumes JSON scenario / hand files (`ReplayHandScenario`, `ReplayPlayer`, `ReplayActionStep`).
  - Feeds the state orchestrator event-by-event.
  - Emits real-time `DecisionReport` on Hero's turn using the EV engine and Monte Carlo simulator.
  - *Validation:* Full simulated tournament hand test vector (`test_mock_replay_adapter_full_tournament_hand`).

---

## Phase 4: Integration Bridge & Streaming API (`crates/api-bridge`)
- [x] **4.1: IPC & WebSocket Event Server**
  - Implement low-latency WebSocket / IPC server (using `tokio` and `tokio-tungstenite`).
  - Stream `GameEvent` notifications and `DecisionReport` payloads asynchronously without blocking engine calculations.
  - *Validation:* Integration test streaming 1,000 serialized decision reports over localhost with latency < 1ms (Validated: 80.28 µs/report).

- [x] **4.2: Real Hand History Watcher (File Ingestion Adapter)**
  - Implement a zero-polling / OS-level file watcher (`notify` crate) monitoring tournament hand history directories.
  - Regex/Nom parser for standard hand history text formats into `GameState` events.
  - *Validation:* Ingest a live-written text hand history and trigger matching `HeroToAct` decisions (`test_hand_history_watcher_and_streaming_integration`).

---

## Phase 5: Analytics & Real-Time Visualization (`crates/dashboard-ui`)
- [x] **5.1: Real-Time Heads-Up Dashboard (Frontend/UI)**
  - Lightweight, rich dark glassmorphic Web UI in `crates/dashboard-ui`.
  - 13x13 Range Matrix Heatmap rendering Hero vs. Bayesian-filtered Villain ranges with combo inspectors.
  - Live EV Differential Card: Real-time visual comparison of `FOLD`, `CHECK`, `CALL`, and bet sizings with highlighted optimal argmax EV.
  - Pot Odds vs. Required Equity SVG dial gauge with margin readout.
  - WebSocket streaming ingestion from `WsEventServer` and interactive simulated live hand animations.
  - *Validation:* HTML5, CSS3, and JavaScript validated; verified real-time stream ingestion and animation sequences.

- [x] **5.2: End-to-End Session Auditor / Post-Game Reviewer**
  - Batch tournament runner reading full tournament log files.
  - Calculates cumulative EV loss (Mistakes / Leaks) comparing actual player actions vs. optimal `DecisionReport` recommendations.
  - Quantifies Big Blind EV losses, classifies blunder severities, and generates formatted terminal tables and JSON exports.
  - *Validation:* Unit and integration tests in `tests/audit_tests.rs` (4/4 tests passing) and CLI verification (`poker_cli audit`).