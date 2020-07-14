use std::convert::TryFrom;
use std::ops::Deref;

use super::card::{Suit, Value};
use super::{Card, Deck, Rankable};

/// A convenience struct holding a collection of cards.
#[derive(Debug, Clone)]
pub struct CardCollection {
    cards: Vec<Card>,
}

impl CardCollection {
    /// Shuffle this card collection using the given random number generator.
    ///
    /// `rng(x)` should return a random number in range `[0, x)`
    pub fn shuffle(&mut self, rng: impl Fn(usize) -> usize) {
        for i in (1..self.len()).rev() {
            self.cards.swap(i, rng(i + 1));
        }
    }

    /// Copies this card collection into an fixed size array.
    ///
    /// Panics if the sizes do not match.
    #[cfg(test)]
    pub(crate) fn to_array<A: Default + AsMut<[Card]>>(&self) -> A {
        let mut a = A::default();
        <A as AsMut<[Card]>>::as_mut(&mut a).clone_from_slice(&self);
        a
    }
}

impl Default for CardCollection {
    /// Return a default deck consisting of 52 cards (13 values * 4 suits).
    fn default() -> Self {
        let mut cards = Vec::new();
        for v in &Value::values() {
            for s in &Suit::suits() {
                cards.push(Card {
                    value: *v,
                    suit: *s,
                });
            }
        }
        CardCollection { cards }
    }
}

impl From<Vec<Card>> for CardCollection {
    fn from(cards: Vec<Card>) -> Self {
        Self { cards }
    }
}

impl From<&[Card]> for CardCollection {
    fn from(cards: &[Card]) -> Self {
        Self {
            cards: cards.to_vec(),
        }
    }
}

impl TryFrom<&str> for CardCollection {
    type Error = String;

    /// Parse cards from str
    ///
    /// # Examples
    ///
    /// ```
    /// use std::convert::TryInto;
    /// use poppy::deck::CardCollection;
    /// let hand : Result<CardCollection, _> = "AdKd".try_into();
    /// assert!(hand.is_ok());
    /// ```
    ///
    /// Anything that can't be parsed will return an error.
    ///
    /// ```
    /// use std::convert::TryInto;
    /// use poppy::deck::CardCollection;
    /// let hand : Result<CardCollection, _> = "AdKx".try_into();
    /// assert!(hand.is_err());
    /// ```
    fn try_from(s: &str) -> Result<Self, Self::Error> {
        // Get the chars iterator.
        let mut chars = s.chars();
        // Where we will put the cards
        //
        // We make the assumption that the hands will have 2 plus five cards.
        let mut cards = Vec::new();

        // Keep looping until we explicitly break
        loop {
            // Now try and get a char.
            let vco = chars.next();
            // If there was no char then we are done.
            if vco == None {
                break;
            } else {
                // If we got a value char then we should get a
                // suit.
                let sco = chars.next();
                // Now try and parse the two chars that we have.
                let v = vco
                    .and_then(Value::from_char)
                    .ok_or_else(|| format!("Couldn't parse value {}", vco.unwrap_or('?')))?;
                let s = sco
                    .and_then(Suit::from_char)
                    .ok_or_else(|| format!("Couldn't parse suit {}", sco.unwrap_or('?')))?;

                let c = Card { value: v, suit: s };
                if cards.iter().any(|&card| card == c) {
                    // If this card is already in the set then error out.
                    return Err(format!("This card has already been added {}", c));
                } else {
                    cards.push(c);
                }
            }
        }

        if chars.next() != None {
            return Err(String::from("Extra un-used chars found."));
        }

        Ok(Self { cards })
    }
}

impl Rankable for CardCollection {
    fn cards(&self) -> &[Card] {
        &self.cards
    }
}

impl Deck for CardCollection {
    fn deal(&mut self) -> Option<Card> {
        self.cards.pop()
    }

    fn is_empty(&self) -> bool {
        self.cards.is_empty()
    }
}

impl Deref for CardCollection {
    type Target = [Card];

    fn deref(&self) -> &Self::Target {
        &self.cards
    }
}
