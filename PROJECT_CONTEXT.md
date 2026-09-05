# Project Context: Tournament Poker Decision & Analytics Engine

## 1. Purpose & Scope
This project is an advanced, research-oriented analytics engine designed to evaluate tournament poker decisions (MTTs) under low latency (<50ms).
**Game Variant:** Exclusively No-Limit Texas Hold'em (NLHE). No Omaha or mixed-game abstractions are required. Hand evaluation is strictly 7-card hold'em rules (best 5 out of 2 hole cards + 5 board cards).
Key focus areas:
1. **Preflop Primacy:** Accurate ICM (Independent Chip Model) transformations over precomputed blueprint matrices.
2. **Exploitative Decision Making:** Best-Response optimization against empirical opponent deviations (MDA / Bayesian updating) rather than unexploitable pure GTO.
3. **Multiway Handling:** Rapid postflop heuristics and Monte Carlo bitboard runouts capable of handling 3+ active players.

---

## 2. System Architecture Pipeline
[ GameState Input (JSON / CLI / Struct) ]
│
▼
┌────────────────────────────────────────────────────────┐
│ Stage 1: Bitboard State Tracker                        │
│ - Dead card masking (Hero cards + Board cards)         │
│ - Pot odds and effective stack quantification          │
└────────────────────────────────────────────────────────┘
│
┌─────────┴─────────┐
│ (Preflop)         │ (Postflop)
▼                   ▼
┌─────────────────┐ ┌────────────────────────────────────┐
│ Stage 2A: ICM   │ │ Stage 2B: Opponent Modeling        │
│ & Preflop Table │ │ - 1326-combo Bayesian weight update│
│ - Malmuth-      │ │ - Filter range by action tendencies │
│   Harville      │ └────────────────────────────────────┘
│ - Risk Premium  │                  │
└─────────────────┘                  ▼
│          ┌────────────────────────────────────┐
│          │ Stage 3: Multiway Equity Evaluator │
│          │ - 7-card bitboard lookup / SIMD    │
│          │ - Fast dead-card Monte Carlo       │
│          └────────────────────────────────────┘
│                           │
└─────────┬─────────────────┘
▼
┌────────────────────────────────────────────────────────┐
│ Stage 4: Best-Response EV Maximizer                    │
│ - Compute EV(Fold), EV(Call), EV(Bet_Sizing)           │
│ - Select argmax EV and format DecisionReport           │
└────────────────────────────────────────────────────────┘
│
▼
[ Decision Report Output ]



---

## 3. Data Representation Standards

### 3.1 Card Encoding (32-bit Integer)
Every card is packed into an unsigned 32-bit integer:
- **Bits 31-16 (Rank Bitmask):** `(1 << rank) << 16` (Allows fast union checking with `|`).
- **Bits 15-12 (Suit Mask):** One-hot: `Spades = 0x1, Hearts = 0x2, Diamonds = 0x4, Clubs = 0x8`.
- **Bits 11-8 (Rank Index):** Integer `0..12` (`2 = 0, ..., A = 12`).
- **Bits 7-0 (Prime Key):** Assigned primes: `{2:2, 3:3, 4:5, 5:7, 6:11, 7:13, 8:17, 9:19, T:23, J:29, Q:31, K:37, A:41}`.

### 3.2 Deck & Hand Masking (64-bit Bitboard)
- A set of cards (e.g., dead cards, hero hand, board) is stored as a `u64`.
- Card index: `idx = rank * 4 + suit_index` (from 0 to 51).
- Card mask: `1ULL << idx`.
- Collision check: `(dead_mask & card_mask) != 0`.

---

## 4. Mathematical Formulations

### 4.1 Malmuth-Harville ICM (Memoized Bitmask DP)
For $N$ players with stacks $S = [s_0, \dots, s_{N-1}]$ and payouts $V = [v_0, \dots, v_{M-1}]$:
- State bitmask $M \in [0, 2^N - 1]$ denotes eliminated/finished positions.
- Step transition:
  $$P(i \mid M) = \frac{s_i}{\sum_{j \notin M} s_j}$$

### 4.2 Best-Response Expected Value (EV)
- $\text{EV}(\text{Fold}) = 0.0$
- $\text{EV}(\text{Call}) = (E_{\text{Hero}} \times \text{FinalPot}) - \text{CallCost}$
- $\text{EV}(\text{Bet}_S) = P(\text{Fold}) \times \text{CurrentPot} + P(\text{Call}) \times [E_{\text{Hero vs CallRange}} \times (\text{CurrentPot} + 2S) - S]$