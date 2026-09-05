use crate::card::{Card, CardParseError};
use core::fmt;
use core::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign, Not, Sub, SubAssign};

/// Mask of 52 ones representing all 52 cards in a standard deck.
pub const FULL_DECK_MASK: u64 = (1u64 << 52) - 1;

/// A 64-bit bitboard representation of a deck or subset of cards.
/// Bit `i` corresponds to the card with `card.index() == i` (for `0 <= i < 52`).
#[derive(Copy, Clone, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct DeckBitmask(u64);

impl DeckBitmask {
    /// Creates an empty deck bitmask (0 cards).
    #[inline(always)]
    pub const fn empty() -> Self {
        DeckBitmask(0)
    }

    /// Creates a full deck containing all 52 cards.
    #[inline(always)]
    pub const fn full() -> Self {
        DeckBitmask(FULL_DECK_MASK)
    }

    /// Creates a DeckBitmask directly from raw u64, masking out bits >= 52.
    #[inline(always)]
    pub const fn from_u64(raw: u64) -> Self {
        DeckBitmask(raw & FULL_DECK_MASK)
    }

    /// Returns the raw 64-bit integer representation.
    #[inline(always)]
    pub const fn as_u64(self) -> u64 {
        self.0
    }

    /// Adds a card to the deck bitmask.
    #[inline(always)]
    pub fn add_card(&mut self, card: Card) {
        self.0 |= 1u64 << card.index();
    }

    /// Removes a card from the deck bitmask.
    #[inline(always)]
    pub fn remove_card(&mut self, card: Card) {
        self.0 &= !(1u64 << card.index());
    }

    /// Checks whether the deck bitmask contains the specified card.
    #[inline(always)]
    pub const fn has_card(self, card: Card) -> bool {
        (self.0 & (1u64 << card.index())) != 0
    }

    /// Alias for `has_card`.
    #[inline(always)]
    pub const fn contains(self, card: Card) -> bool {
        self.has_card(card)
    }

    /// Returns a new bitmask with dead/blocked cards removed from this set (`self & !dead`).
    #[inline(always)]
    pub const fn dead_cards(self, dead: DeckBitmask) -> DeckBitmask {
        DeckBitmask(self.0 & !dead.0)
    }

    /// Returns the number of cards currently present in the bitmask.
    #[inline(always)]
    pub const fn count(self) -> u32 {
        self.0.count_ones()
    }

    /// Checks whether the bitmask is empty (0 cards).
    #[inline(always)]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Creates a non-allocating iterator yielding all `Card`s in the bitmask.
    #[inline(always)]
    pub const fn iter(self) -> DeckIterator {
        DeckIterator { mask: self.0 }
    }

    /// Constructs a DeckBitmask from a slice of cards without heap allocations.
    #[inline]
    pub fn from_cards(cards: &[Card]) -> Self {
        let mut deck = Self::empty();
        for &c in cards {
            deck.add_card(c);
        }
        deck
    }

    /// Parses a sequence of concatenated 2-character card strings (e.g. "AhKsQd").
    pub fn from_str_multi(s: &str) -> Result<Self, CardParseError> {
        let trimmed = s.trim();
        if trimmed.is_empty() {
            return Ok(Self::empty());
        }
        if !trimmed.len().is_multiple_of(2) {
            return Err(CardParseError::InvalidLength(trimmed.len()));
        }

        let mut deck = Self::empty();
        for chunk in trimmed.as_bytes().as_chunks::<2>().0 {
            let rank_ch = chunk[0] as char;
            let suit_ch = chunk[1] as char;
            let card = Card::from_chars(rank_ch, suit_ch)?;
            deck.add_card(card);
        }
        Ok(deck)
    }
}

impl BitOr for DeckBitmask {
    type Output = Self;
    #[inline(always)]
    fn bitor(self, rhs: Self) -> Self::Output {
        DeckBitmask(self.0 | rhs.0)
    }
}

impl BitOrAssign for DeckBitmask {
    #[inline(always)]
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl BitAnd for DeckBitmask {
    type Output = Self;
    #[inline(always)]
    fn bitand(self, rhs: Self) -> Self::Output {
        DeckBitmask(self.0 & rhs.0)
    }
}

impl BitAndAssign for DeckBitmask {
    #[inline(always)]
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0;
    }
}

impl BitXor for DeckBitmask {
    type Output = Self;
    #[inline(always)]
    fn bitxor(self, rhs: Self) -> Self::Output {
        DeckBitmask(self.0 ^ rhs.0)
    }
}

impl BitXorAssign for DeckBitmask {
    #[inline(always)]
    fn bitxor_assign(&mut self, rhs: Self) {
        self.0 ^= rhs.0;
    }
}

impl Not for DeckBitmask {
    type Output = Self;
    #[inline(always)]
    fn not(self) -> Self::Output {
        DeckBitmask((!self.0) & FULL_DECK_MASK)
    }
}

impl Sub for DeckBitmask {
    type Output = Self;
    #[inline(always)]
    fn sub(self, rhs: Self) -> Self::Output {
        self.dead_cards(rhs)
    }
}

impl SubAssign for DeckBitmask {
    #[inline(always)]
    fn sub_assign(&mut self, rhs: Self) {
        self.0 &= !rhs.0;
    }
}

/// Zero-allocation iterator yielding `Card` items from a `DeckBitmask`.
#[derive(Copy, Clone, Debug)]
pub struct DeckIterator {
    mask: u64,
}

impl Iterator for DeckIterator {
    type Item = Card;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.mask == 0 {
            None
        } else {
            let trailing = self.mask.trailing_zeros() as usize;
            // Clear lowest bit: mask &= mask - 1
            self.mask &= self.mask - 1;
            Card::from_index(trailing).ok()
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let count = self.mask.count_ones() as usize;
        (count, Some(count))
    }
}

impl ExactSizeIterator for DeckIterator {}

impl IntoIterator for DeckBitmask {
    type Item = Card;
    type IntoIter = DeckIterator;

    #[inline(always)]
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl fmt::Debug for DeckBitmask {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DeckBitmask(count={}, [", self.count())?;
        let mut first = true;
        for card in *self {
            if !first {
                write!(f, ", ")?;
            }
            write!(f, "{}", card)?;
            first = false;
        }
        write!(f, "])")
    }
}

impl fmt::Display for DeckBitmask {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for card in *self {
            write!(f, "{}", card)?;
        }
        Ok(())
    }
}
