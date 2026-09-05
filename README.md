# PokerAnalyse 🃏⚡

> **High-Performance Tournament Poker Decision & Analytics Engine**  
> Written in pure Rust with sub-millisecond evaluation, Malmuth-Harville ICM, Bayesian opponent modeling, real-time WebSocket streaming, and a glassmorphic Heads-Up HUD / Post-Game Auditor.

---

## 🚀 Key Capabilities

- **Ultra-Fast 7-Card Evaluator**:
  - Cactus-Kev $O(1)$ lookup algorithm combined with unrolled $\binom{7}{5} = 21$ combination evaluation.
  - Achieves **46+ Million hands/second** on a single CPU core.
- **Malmuth-Harville Dynamic Programming ICM**:
  - Exact monetary equity calculation across up to 10 players.
  - Generates pairwise risk matrices and bubble factor risk premiums in **< 8 microseconds**.
- **1,326-Combo Bayesian Opponent Modeling**:
  - Full combinatoric weight matrix with dead-card blocker removal (**0.56 µs/pass**).
  - Dynamic Bayesian range tightening based on board texture and opponent action profiles.
- **Argmax EV Differential Solver**:
  - Evaluates candidate actions (`Fold`, `Check`, `Call`, `Bet 33%`, `Bet 75%`, `All-In`).
  - Factors in fold equity, opponent calling ranges, and risk premiums to output optimal recommendations.
- **Real-Time Heads-Up Dashboard (Web HUD)**:
  - Modern dark glassmorphic UI built in Vanilla HTML/CSS/JS (`crates/dashboard-ui`).
  - Interactive **13×13 Range Matrix Heatmap** with dynamic Hero combo highlighting.
  - Live **EV Differential Bars** with best action callout.
  - Animated **Pot Odds vs. Required Equity SVG Dial**.
  - **Interactive Step-by-Step Evaluator**: Manual pacing controls (`[Passo Seguinte]`, `[Anterior]`, `[Auto-Play]`) and keyboard shortcuts (`Space` / `→` / `←`) to study the bot's decisions street-by-street.
- **End-to-End Post-Game Session Auditor**:
  - Replays full tournament log files from PokerStars and online rooms.
  - Quantifies cumulative EV losses (chips and Big Blinds).
  - Classifies leaks into `Blunder` ( $\ge 3.0$ BB), `Mistake` ($1.0 - 3.0$ BB), `Inaccuracy` ($0.25 - 1.0$ BB), and `Optimal` (< 0.25 BB).
  - Generates formatted terminal reports and machine-readable JSON exports.

---

## 📦 Project Architecture

```
PokerAnalyse/
├── crates/
│   └── dashboard-ui/          # Modern Web HUD & Post-Game Reviewer
│       ├── index.html         # HUD & Session Auditor views
│       ├── dashboard.css      # Glassmorphism design system & animations
│       └── dashboard.js       # Range matrix, EV cards, SVG dial & playback controls
├── src/
│   ├── api_bridge/            # WebSocket streaming server & live file watcher
│   ├── audit/                 # Session auditor & leak severity analysis
│   ├── card.rs                # 32-bit packed card primitive & 64-bit deck bitboard
│   ├── eval/                  # Cactus-Kev 5-card & 7-card hold'em evaluator
│   ├── icm/                   # Malmuth-Harville DP calculator & risk premium
│   ├── ingestion/             # Hand parser & deterministic replay adapter
│   ├── orchestrator/          # Multiway side pot & hand state machine
│   ├── range/                 # 1,326-combo matrix & Bayesian tendency filter
│   ├── sim/                   # Multiway Monte Carlo & argmax EV engine
│   ├── lib.rs                 # Core library exports
│   └── main.rs                # CLI entry point (`poker_cli`)
├── tests/                     # 45 integration, unit, and benchmark tests
├── sample_tournament.txt      # Multi-hand tournament log for auditing
└── ROADMAP.md                 # Complete 5-phase project implementation roadmap
```

---

## 🛠️ CLI Usage & Commands

### 1. Run Session Audit on a Tournament Log
Analyze all hands played, identify leaks, and view EV loss breakdowns:
```powershell
$env:CARGO_INCREMENTAL=0; cargo run --bin poker_cli -- audit sample_tournament.txt
```
To export the audit as structured JSON:
```powershell
$env:CARGO_INCREMENTAL=0; cargo run --bin poker_cli -- audit sample_tournament.txt --json
```

### 2. Launch Interactive Live Streaming Demo & Web HUD
Spawns the embedded HTTP & WebSocket server on port 9001:
```powershell
$env:CARGO_INCREMENTAL=0; cargo run --bin poker_cli -- demo --port 9001
```
Open **`http://127.0.0.1:9001/`** in your browser.

**Evaluation Controls in the HUD:**
- **`[ ⏭ Passo Seguinte ]`** (or **`Space`** / **`→`**): Advance to the next decision node.
- **`[ ⏮ Anterior ]`** (or **`←`**): Step back to the previous decision node.
- **`[ ▶ Auto-Play ]`**: Toggle automated playback (4 seconds per decision).

### 3. Live Directory File Watcher
Monitor a live hand history folder written by poker clients in real time:
```powershell
$env:CARGO_INCREMENTAL=0; cargo run --bin poker_cli -- serve --port 9001 --watch ./hand_history
```

---

## 🧪 Testing & Validation

All 45 automated integration, unit, and benchmark tests pass cleanly across all five project phases:

```powershell
$env:CARGO_INCREMENTAL=0; cargo test
```

```text
test result: ok. 45 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

---

## 📜 License
MIT License. Developed for research, tournament poker analysis, and optimal decision modeling.