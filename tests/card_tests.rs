use poker_analyse::{Card, CardParseError, DeckBitmask, Rank, Suit, RANK_PRIMES};
use std::collections::HashSet;

#[test]
fn test_all_52_cards_integrity_and_bit_layout() {
    let mut seen_raw = HashSet::new();
    let mut seen_indices = HashSet::new();

    for &rank in &Rank::ALL {
        for &suit in &Suit::ALL {
            let card = Card::new(rank, suit);
            let raw = card.raw();

            // 1. Bit uniqueness
            assert!(
                seen_raw.insert(raw),
                "Duplicate raw value 0x{:08X} for {:?}",
                raw,
                card
            );

            // 2. Index integrity: in [0, 52) and unique
            let idx = card.index();
            assert!(idx < 52, "Index {} out of bounds for card {:?}", idx, card);
            assert!(
                seen_indices.insert(idx),
                "Duplicate index {} for card {:?}",
                idx,
                card
            );

            // 3. Index roundtrip
            let reconstructed = Card::from_index(idx).expect("Failed to reconstruct from index");
            assert_eq!(card, reconstructed);

            // 4. Rank and Suit getters
            assert_eq!(card.rank(), rank);
            assert_eq!(card.suit(), suit);

            // 5. Cactus-Kev bit layout verification
            // Bits 0..=5: prime
            let expected_prime = RANK_PRIMES[rank as usize];
            assert_eq!(
                card.prime(),
                expected_prime,
                "Prime mismatch for {:?}: expected {}, got {}",
                card,
                expected_prime,
                card.prime()
            );
            assert_eq!(raw & 0x3F, expected_prime);

            // Bits 8..=11: rank integer
            let rank_int = (raw >> 8) & 0xF;
            assert_eq!(rank_int, rank as u32);

            // Bits 12..=15: suit bitmask (s=0x1, h=0x2, d=0x4, c=0x8)
            let suit_bits = (raw >> 12) & 0xF;
            assert_eq!(suit_bits, suit.mask_bit());
            assert_eq!(card.suit_bitmask(), suit.mask_bit());

            // Bits 16..=28: rank bitmask (1 << (16 + rank))
            let rank_mask_shifted = (raw >> 16) & 0x1FFF;
            assert_eq!(rank_mask_shifted, 1u32 << (rank as u32));
            assert_eq!(card.rank_bitmask(), 1u32 << (rank as u32));

            // Unused bits must be zero: bits 6, 7, 29, 30, 31
            let unused_mask = (1u32 << 6) | (1u32 << 7) | (7u32 << 29);
            assert_eq!(
                raw & unused_mask,
                0,
                "Unused bits set in raw card value: 0x{:08X}",
                raw
            );
        }
    }

    assert_eq!(seen_raw.len(), 52);
    assert_eq!(seen_indices.len(), 52);
}

#[test]
fn test_string_parser_and_display() {
    let test_cases = [
        ("Ah", Rank::Ace, Suit::Hearts),
        ("As", Rank::Ace, Suit::Spades),
        ("Kd", Rank::King, Suit::Diamonds),
        ("Qc", Rank::Queen, Suit::Clubs),
        ("Jh", Rank::Jack, Suit::Hearts),
        ("Ts", Rank::Ten, Suit::Spades),
        ("9d", Rank::Nine, Suit::Diamonds),
        ("8c", Rank::Eight, Suit::Clubs),
        ("7h", Rank::Seven, Suit::Hearts),
        ("6s", Rank::Six, Suit::Spades),
        ("5d", Rank::Five, Suit::Diamonds),
        ("4c", Rank::Four, Suit::Clubs),
        ("3h", Rank::Three, Suit::Hearts),
        ("2s", Rank::Two, Suit::Spades),
    ];

    for (str_rep, expected_rank, expected_suit) in test_cases {
        let card = Card::from_str_exact(str_rep).expect("Failed to parse valid card");
        assert_eq!(card.rank(), expected_rank);
        assert_eq!(card.suit(), expected_suit);

        // Display check
        assert_eq!(card.to_string(), str_rep);

        // Case insensitivity support in from_str_exact
        let lower = str_rep.to_lowercase();
        let from_lower = Card::from_str_exact(&lower).expect("Failed to parse lower case");
        assert_eq!(card, from_lower);
    }
}

#[test]
fn test_parser_invalid_inputs() {
    // Length != 2
    assert_eq!(Card::from_str_exact(""), Err(CardParseError::InvalidLength(0)));
    assert_eq!(Card::from_str_exact("A"), Err(CardParseError::InvalidLength(1)));
    assert_eq!(Card::from_str_exact("Ahs"), Err(CardParseError::InvalidLength(3)));

    // Invalid rank
    assert_eq!(
        Card::from_str_exact("1h"),
        Err(CardParseError::InvalidRankChar('1'))
    );
    assert_eq!(
        Card::from_str_exact("Xh"),
        Err(CardParseError::InvalidRankChar('X'))
    );

    // Invalid suit
    assert_eq!(
        Card::from_str_exact("Ax"),
        Err(CardParseError::InvalidSuitChar('x'))
    );
    assert_eq!(
        Card::from_str_exact("A9"),
        Err(CardParseError::InvalidSuitChar('9'))
    );

    // Invalid index
    assert_eq!(Card::from_index(52), Err(CardParseError::InvalidCardIndex(52)));
    assert_eq!(Card::from_index(100), Err(CardParseError::InvalidCardIndex(100)));
}

#[test]
fn test_deck_bitmask_operations() {
    let mut deck = DeckBitmask::empty();
    assert_eq!(deck.count(), 0);
    assert!(deck.is_empty());

    let ah = Card::from_str_exact("Ah").unwrap();
    let ks = Card::from_str_exact("Ks").unwrap();
    let qd = Card::from_str_exact("Qd").unwrap();

    // Add cards
    deck.add_card(ah);
    assert_eq!(deck.count(), 1);
    assert!(deck.has_card(ah));
    assert!(!deck.has_card(ks));

    deck.add_card(ks);
    assert_eq!(deck.count(), 2);
    assert!(deck.has_card(ks));

    // Remove card
    deck.remove_card(ah);
    assert_eq!(deck.count(), 1);
    assert!(!deck.has_card(ah));
    assert!(deck.has_card(ks));

    // Full deck check
    let full = DeckBitmask::full();
    assert_eq!(full.count(), 52);
    assert!(!full.is_empty());
    for &rank in &Rank::ALL {
        for &suit in &Suit::ALL {
            let c = Card::new(rank, suit);
            assert!(full.has_card(c));
        }
    }

    // Dead cards removal
    let mut dead = DeckBitmask::empty();
    dead.add_card(ah);
    dead.add_card(ks);
    dead.add_card(qd);
    assert_eq!(dead.count(), 3);

    let remaining = full.dead_cards(dead);
    assert_eq!(remaining.count(), 49);
    assert!(!remaining.has_card(ah));
    assert!(!remaining.has_card(ks));
    assert!(!remaining.has_card(qd));

    // Operator subtraction
    let sub_remaining = full - dead;
    assert_eq!(sub_remaining, remaining);

    // Bitwise intersection & union
    let union_deck = remaining | dead;
    assert_eq!(union_deck, full);

    let intersect_deck = remaining & dead;
    assert_eq!(intersect_deck, DeckBitmask::empty());
}

#[test]
fn test_deck_bitmask_iterator() {
    let test_cards = ["Ah", "Ks", "2c", "Td", "7s"];
    let mut bitmask = DeckBitmask::empty();
    for &tc in &test_cards {
        bitmask.add_card(Card::from_str_exact(tc).unwrap());
    }

    assert_eq!(bitmask.count(), 5);

    let collected: Vec<Card> = bitmask.into_iter().collect();
    assert_eq!(collected.len(), 5);

    for card in collected {
        assert!(bitmask.has_card(card));
    }
}

#[test]
fn test_deck_from_multi_str() {
    let deck = DeckBitmask::from_str_multi("AhKsQdJc").unwrap();
    assert_eq!(deck.count(), 4);
    assert!(deck.has_card(Card::from_str_exact("Ah").unwrap()));
    assert!(deck.has_card(Card::from_str_exact("Ks").unwrap()));
    assert!(deck.has_card(Card::from_str_exact("Qd").unwrap()));
    assert!(deck.has_card(Card::from_str_exact("Jc").unwrap()));

    let empty = DeckBitmask::from_str_multi("").unwrap();
    assert!(empty.is_empty());

    let err = DeckBitmask::from_str_multi("AhK");
    assert!(err.is_err());
}
