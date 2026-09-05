use core::fmt;
use std::error::Error;

/// Primes associated with each rank (2 to Ace) for Cactus-Kev evaluation.
pub const RANK_PRIMES: [u32; 13] = [2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41];

/// Represents card rank from Deuce (0) through Ace (12).
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum Rank {
    Two = 0,
    Three = 1,
    Four = 2,
    Five = 3,
    Six = 4,
    Seven = 5,
    Eight = 6,
    Nine = 7,
    Ten = 8,
    Jack = 9,
    Queen = 10,
    King = 11,
    Ace = 12,
}

impl Rank {
    pub const ALL: [Rank; 13] = [
        Rank::Two,
        Rank::Three,
        Rank::Four,
        Rank::Five,
        Rank::Six,
        Rank::Seven,
        Rank::Eight,
        Rank::Nine,
        Rank::Ten,
        Rank::Jack,
        Rank::Queen,
        Rank::King,
        Rank::Ace,
    ];

    #[inline(always)]
    pub const fn prime(self) -> u32 {
        RANK_PRIMES[self as usize]
    }

    #[inline(always)]
    pub const fn as_u8(self) -> u8 {
        self as u8
    }

    #[inline]
    pub const fn from_u8(val: u8) -> Result<Self, CardParseError> {
        match val {
            0 => Ok(Rank::Two),
            1 => Ok(Rank::Three),
            2 => Ok(Rank::Four),
            3 => Ok(Rank::Five),
            4 => Ok(Rank::Six),
            5 => Ok(Rank::Seven),
            6 => Ok(Rank::Eight),
            7 => Ok(Rank::Nine),
            8 => Ok(Rank::Ten),
            9 => Ok(Rank::Jack),
            10 => Ok(Rank::Queen),
            11 => Ok(Rank::King),
            12 => Ok(Rank::Ace),
            _ => Err(CardParseError::InvalidRankValue(val)),
        }
    }

    #[inline]
    pub const fn from_char(c: char) -> Result<Self, CardParseError> {
        match c {
            '2' => Ok(Rank::Two),
            '3' => Ok(Rank::Three),
            '4' => Ok(Rank::Four),
            '5' => Ok(Rank::Five),
            '6' => Ok(Rank::Six),
            '7' => Ok(Rank::Seven),
            '8' => Ok(Rank::Eight),
            '9' => Ok(Rank::Nine),
            'T' | 't' => Ok(Rank::Ten),
            'J' | 'j' => Ok(Rank::Jack),
            'Q' | 'q' => Ok(Rank::Queen),
            'K' | 'k' => Ok(Rank::King),
            'A' | 'a' => Ok(Rank::Ace),
            _ => Err(CardParseError::InvalidRankChar(c)),
        }
    }

    #[inline]
    pub const fn as_char(self) -> char {
        match self {
            Rank::Two => '2',
            Rank::Three => '3',
            Rank::Four => '4',
            Rank::Five => '5',
            Rank::Six => '6',
            Rank::Seven => '7',
            Rank::Eight => '8',
            Rank::Nine => '9',
            Rank::Ten => 'T',
            Rank::Jack => 'J',
            Rank::Queen => 'Q',
            Rank::King => 'K',
            Rank::Ace => 'A',
        }
    }
}

impl fmt::Display for Rank {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_char())
    }
}

/// Represents card suit: Spades (0), Hearts (1), Diamonds (2), Clubs (3).
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum Suit {
    Spades = 0,
    Hearts = 1,
    Diamonds = 2,
    Clubs = 3,
}

impl Suit {
    pub const ALL: [Suit; 4] = [
        Suit::Spades,
        Suit::Hearts,
        Suit::Diamonds,
        Suit::Clubs,
    ];

    #[inline(always)]
    pub const fn as_u8(self) -> u8 {
        self as u8
    }

    /// Suit bitmask for Cactus-Kev encoding:
    /// Spades = 0x1, Hearts = 0x2, Diamonds = 0x4, Clubs = 0x8
    #[inline(always)]
    pub const fn mask_bit(self) -> u32 {
        match self {
            Suit::Spades => 0x1,
            Suit::Hearts => 0x2,
            Suit::Diamonds => 0x4,
            Suit::Clubs => 0x8,
        }
    }

    #[inline]
    pub const fn from_u8(val: u8) -> Result<Self, CardParseError> {
        match val {
            0 => Ok(Suit::Spades),
            1 => Ok(Suit::Hearts),
            2 => Ok(Suit::Diamonds),
            3 => Ok(Suit::Clubs),
            _ => Err(CardParseError::InvalidSuitValue(val)),
        }
    }

    #[inline]
    pub const fn from_char(c: char) -> Result<Self, CardParseError> {
        match c {
            's' | 'S' => Ok(Suit::Spades),
            'h' | 'H' => Ok(Suit::Hearts),
            'd' | 'D' => Ok(Suit::Diamonds),
            'c' | 'C' => Ok(Suit::Clubs),
            _ => Err(CardParseError::InvalidSuitChar(c)),
        }
    }

    #[inline]
    pub const fn as_char(self) -> char {
        match self {
            Suit::Spades => 's',
            Suit::Hearts => 'h',
            Suit::Diamonds => 'd',
            Suit::Clubs => 'c',
        }
    }
}

impl fmt::Display for Suit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_char())
    }
}

/// 32-bit packed card encoder following the Cactus-Kev standard representation.
///
/// Layout:
/// +--------+--------+--------+--------+
/// |xxxbbbbb|bbbbbbbb|cdhsrrrr|xxpppppp|
/// +--------+--------+--------+--------+
/// - Bits 00..=05: Prime number associated with rank (2, 3, 5, ..., 41)
/// - Bits 08..=11: Rank integer (0..=12)
/// - Bits 12..=15: Suit bitmask (s=0x1000, h=0x2000, d=0x4000, c=0x8000)
/// - Bits 16..=28: Rank bitmask: (1 << (16 + rank))
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Card(u32);

impl serde::Serialize for Card {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> serde::Deserialize<'de> for Card {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Card::from_str_exact(&s).map_err(serde::de::Error::custom)
    }
}

impl Card {
    /// Constructs a packed 32-bit Card from Rank and Suit.
    #[inline(always)]
    pub const fn new(rank: Rank, suit: Suit) -> Self {
        let r = rank as u32;
        let prime = rank.prime();
        let suit_bit = suit.mask_bit();
        let rank_mask = 1u32 << (16 + r);
        let packed = rank_mask | (suit_bit << 12) | (r << 8) | prime;
        Card(packed)
    }

    /// Wraps a pre-encoded 32-bit raw integer.
    #[inline(always)]
    pub const fn from_raw(raw: u32) -> Self {
        Card(raw)
    }

    /// Returns the underlying 32-bit packed representation.
    #[inline(always)]
    pub const fn raw(self) -> u32 {
        self.0
    }

    /// Returns the rank prime (bits 0..=5).
    #[inline(always)]
    pub const fn prime(self) -> u32 {
        self.0 & 0x3F
    }

    /// Returns the rank of this card (bits 8..=11).
    #[inline(always)]
    pub const fn rank(self) -> Rank {
        let r = ((self.0 >> 8) & 0xF) as u8;
        match Rank::from_u8(r) {
            Ok(rank) => rank,
            Err(_) => unreachable!(),
        }
    }

    /// Returns the suit of this card (derived from suit bits 12..=15).
    #[inline(always)]
    pub const fn suit(self) -> Suit {
        let s_mask = (self.0 >> 12) & 0xF;
        match s_mask {
            0x1 => Suit::Spades,
            0x2 => Suit::Hearts,
            0x4 => Suit::Diamonds,
            0x8 => Suit::Clubs,
            _ => unreachable!(),
        }
    }

    /// Returns the suit bitmask (0x1, 0x2, 0x4, or 0x8).
    #[inline(always)]
    pub const fn suit_bitmask(self) -> u32 {
        (self.0 >> 12) & 0xF
    }

    /// Returns the rank bitmask shifted to 0..12 (1 << rank).
    #[inline(always)]
    pub const fn rank_bitmask(self) -> u32 {
        (self.0 >> 16) & 0x1FFF
    }

    /// Returns the unique card index in [0, 52).
    /// Indexed as: `rank * 4 + suit`.
    #[inline(always)]
    pub const fn index(self) -> usize {
        (self.rank() as usize) * 4 + (self.suit() as usize)
    }

    /// Reconstructs a Card from a unique index in [0, 52).
    #[inline]
    pub const fn from_index(idx: usize) -> Result<Self, CardParseError> {
        if idx >= 52 {
            return Err(CardParseError::InvalidCardIndex(idx));
        }
        let r = (idx / 4) as u8;
        let s = (idx % 4) as u8;
        let rank = match Rank::from_u8(r) {
            Ok(rank) => rank,
            Err(e) => return Err(e),
        };
        let suit = match Suit::from_u8(s) {
            Ok(suit) => suit,
            Err(e) => return Err(e),
        };
        Ok(Card::new(rank, suit))
    }

    /// Parses a 2-character card representation without heap allocation (e.g. "Ah", "2c").
    #[inline]
    pub const fn from_chars(rank_ch: char, suit_ch: char) -> Result<Self, CardParseError> {
        let rank = match Rank::from_char(rank_ch) {
            Ok(r) => r,
            Err(e) => return Err(e),
        };
        let suit = match Suit::from_char(suit_ch) {
            Ok(s) => s,
            Err(e) => return Err(e),
        };
        Ok(Card::new(rank, suit))
    }

    /// Parses a 2-character string with zero heap allocations.
    #[inline]
    pub fn from_str_exact(s: &str) -> Result<Self, CardParseError> {
        let bytes = s.as_bytes();
        if bytes.len() != 2 {
            return Err(CardParseError::InvalidLength(s.len()));
        }
        Self::from_chars(bytes[0] as char, bytes[1] as char)
    }
}

impl TryFrom<&str> for Card {
    type Error = CardParseError;

    #[inline]
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Card::from_str_exact(value)
    }
}

impl core::str::FromStr for Card {
    type Err = CardParseError;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Card::from_str_exact(s)
    }
}

impl fmt::Display for Card {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", self.rank().as_char(), self.suit().as_char())
    }
}

impl fmt::Debug for Card {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Card({}{} | raw: 0x{:08X})",
            self.rank().as_char(),
            self.suit().as_char(),
            self.0
        )
    }
}

/// Errors occurring during card string or index parsing.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum CardParseError {
    InvalidLength(usize),
    InvalidRankChar(char),
    InvalidSuitChar(char),
    InvalidRankValue(u8),
    InvalidSuitValue(u8),
    InvalidCardIndex(usize),
}

impl fmt::Display for CardParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CardParseError::InvalidLength(len) => {
                write!(f, "Expected 2-character card string, got length {}", len)
            }
            CardParseError::InvalidRankChar(c) => write!(f, "Invalid card rank character: '{}'", c),
            CardParseError::InvalidSuitChar(c) => write!(f, "Invalid card suit character: '{}'", c),
            CardParseError::InvalidRankValue(v) => write!(f, "Invalid card rank integer: {}", v),
            CardParseError::InvalidSuitValue(v) => write!(f, "Invalid card suit integer: {}", v),
            CardParseError::InvalidCardIndex(i) => {
                write!(f, "Invalid card index: {} (must be in 0..52)", i)
            }
        }
    }
}

impl Error for CardParseError {}
