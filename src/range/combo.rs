use crate::card::{Card, Rank, Suit};
use crate::deck::DeckBitmask;

/// Total number of unique 2-card starting hand combinations in Texas Hold'em: 52 * 51 / 2 = 1326.
pub const NUM_HOLE_COMBOS: usize = 1326;

/// Lookup tables for bidirectional $O(1)$ mapping between card pairs and combo indices.
pub struct ComboTables {
    /// Maps combo index in [0, 1326) to the ordered pair of Cards (card1 < card2).
    pub combo_to_cards: [(Card, Card); NUM_HOLE_COMBOS],
    /// Maps pair of card indices [c1][c2] in [0, 52) to the combo index in [0, 1326).
    pub card_pair_to_combo: [[u16; 52]; 52],
    /// Maps combo index to the corresponding 64-bit DeckBitmask with exactly 2 bits set.
    pub combo_to_mask: [DeckBitmask; NUM_HOLE_COMBOS],
}

const fn init_combo_tables() -> ComboTables {
    let dummy_card = Card::from_raw(0);
    let mut combo_to_cards = [(dummy_card, dummy_card); NUM_HOLE_COMBOS];
    let mut card_pair_to_combo = [[0u16; 52]; 52];
    let mut combo_to_mask = [DeckBitmask::empty(); NUM_HOLE_COMBOS];

    let mut idx = 0usize;
    let mut c1 = 0usize;
    while c1 < 52 {
        let r1 = c1 / 4;
        let s1 = c1 % 4;
        let card1 = Card::new(Rank::ALL[r1], Suit::ALL[s1]);

        let mut c2 = c1 + 1;
        while c2 < 52 {
            let r2 = c2 / 4;
            let s2 = c2 % 4;
            let card2 = Card::new(Rank::ALL[r2], Suit::ALL[s2]);

            combo_to_cards[idx] = (card1, card2);
            card_pair_to_combo[c1][c2] = idx as u16;
            card_pair_to_combo[c2][c1] = idx as u16;

            let mask = (1u64 << c1) | (1u64 << c2);
            combo_to_mask[idx] = DeckBitmask::from_u64(mask);

            idx += 1;
            c2 += 1;
        }
        c1 += 1;
    }

    ComboTables {
        combo_to_cards,
        card_pair_to_combo,
        combo_to_mask,
    }
}

/// Static, L1-cache resident combo tables generated at compile time (~30 KB total).
pub static COMBO_TABLES: ComboTables = init_combo_tables();

/// Returns the unique combo index in [0, 1326) for the given pair of cards.
#[inline(always)]
pub fn cards_to_combo(c1: Card, c2: Card) -> usize {
    COMBO_TABLES.card_pair_to_combo[c1.index()][c2.index()] as usize
}

/// Returns the pair of cards corresponding to the given combo index in [0, 1326).
#[inline(always)]
pub fn combo_to_cards(index: usize) -> (Card, Card) {
    COMBO_TABLES.combo_to_cards[index]
}

/// Returns the DeckBitmask with the 2 bits set for the given combo index.
#[inline(always)]
pub fn combo_to_bitmask(index: usize) -> DeckBitmask {
    COMBO_TABLES.combo_to_mask[index]
}

/// Canonical classification for the 169 starting hand categories.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum CanonicalHand {
    /// Pocket pair (e.g. AA, KK): 6 combinations.
    Pair(Rank),
    /// Suited cards with r1 > r2 (e.g. AKs, QJs): 4 combinations.
    Suited(Rank, Rank),
    /// Offsuit cards with r1 > r2 (e.g. AKo, QJo): 12 combinations.
    Offsuit(Rank, Rank),
}

impl CanonicalHand {
    /// Returns the slice or vector of combo indices in [0, 1326) belonging to this canonical hand.
    pub fn combo_indices(self) -> Vec<usize> {
        let mut combos = Vec::new();
        match self {
            CanonicalHand::Pair(rank) => {
                // 6 combinations: 4 choose 2 suits
                for s1 in 0..4 {
                    for s2 in (s1 + 1)..4 {
                        let c1 = Card::new(rank, Suit::ALL[s1]);
                        let c2 = Card::new(rank, Suit::ALL[s2]);
                        combos.push(cards_to_combo(c1, c2));
                    }
                }
            }
            CanonicalHand::Suited(high_rank, low_rank) => {
                // 4 combinations: same suit for both cards
                for s in 0..4 {
                    let suit = Suit::ALL[s];
                    let c1 = Card::new(high_rank, suit);
                    let c2 = Card::new(low_rank, suit);
                    combos.push(cards_to_combo(c1, c2));
                }
            }
            CanonicalHand::Offsuit(high_rank, low_rank) => {
                // 12 combinations: different suits
                for s1 in 0..4 {
                    for s2 in 0..4 {
                        if s1 != s2 {
                            let c1 = Card::new(high_rank, Suit::ALL[s1]);
                            let c2 = Card::new(low_rank, Suit::ALL[s2]);
                            combos.push(cards_to_combo(c1, c2));
                        }
                    }
                }
            }
        }
        combos
    }

    /// Number of natural combinations for this canonical hand (6 for pairs, 4 for suited, 12 for offsuit).
    #[inline(always)]
    pub const fn num_combos(self) -> usize {
        match self {
            CanonicalHand::Pair(_) => 6,
            CanonicalHand::Suited(_, _) => 4,
            CanonicalHand::Offsuit(_, _) => 12,
        }
    }
}
