use crate::deck::{Card, Rank, Rankable};

/// A structure representing the current state of the board, i. e. the public cards
#[derive(Debug, Copy, Clone)]
pub struct Board {
    cards: [Card; 7],
    n: usize,
}

impl Board {
    pub(crate) fn new() -> Self {
        let default_card = Card {
            value: crate::deck::card::Value::Ace,
            suit: crate::deck::card::Suit::Club,
        };
        Self {
            cards: [
                default_card,
                default_card,
                default_card,
                default_card,
                default_card,
                default_card,
                default_card,
            ],
            n: 0,
        }
    }

    /// Inspect the first three cards of the board if they are dealt already.
    ///
    /// This method is guaranteed to return a slice of size 3 if the cards are dealt.
    pub fn flop(&self) -> Option<&[Card]> {
        if self.n >= 3 {
            Some(&self.cards[..3])
        } else {
            None
        }
    }

    /// Inspect the fourth card of the board if it is dealt already.
    pub fn turn(&self) -> Option<Card> {
        if self.n >= 4 {
            Some(self.cards[3])
        } else {
            None
        }
    }

    /// Inspect the fifth card of the board if it is dealt already.
    pub fn river(&self) -> Option<Card> {
        if self.n >= 5 {
            Some(self.cards[4])
        } else {
            None
        }
    }

    /// Get a slice over all the cards which are currently on the board
    pub fn all_cards(&self) -> &[Card] {
        &self.cards[..self.n]
    }

    pub(crate) fn deal_flop(&mut self, cards: [Card; 3]) {
        debug_assert_eq!(self.n, 0);

        self.cards[0] = cards[0];
        self.cards[1] = cards[1];
        self.cards[2] = cards[2];
        self.n = 3;
    }
    pub(crate) fn deal_turn(&mut self, card: Card) {
        debug_assert_eq!(self.n, 3);

        self.cards[3] = card;
        self.n = 4;
    }
    pub(crate) fn deal_river(&mut self, card: Card) {
        debug_assert_eq!(self.n, 4);

        self.cards[4] = card;
        self.n = 5;
    }
    pub(crate) fn clear(&mut self) {
        self.n = 0;
    }
    pub(crate) fn rank_hand(&mut self, hand: [Card; 2]) -> Rank {
        debug_assert_eq!(self.n, 5);

        self.cards[5] = hand[0];
        self.cards[6] = hand[1];
        self.rank()
    }
}

impl Default for Board {
    fn default() -> Self {
        Board::new()
    }
}

impl Rankable for Board {
    fn cards(&self) -> &[Card] {
        &self.cards
    }
}
