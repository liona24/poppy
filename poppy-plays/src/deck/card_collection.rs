use std::collections::HashSet;
use std::convert::TryFrom;
use std::ops::Deref;

use super::card::{Suit, Value};
use super::{Card, Deck, Rankable};

#[derive(Debug, Clone)]
pub struct CardCollection {
    cards: Vec<Card>,
}

impl CardCollection {
    pub(crate) fn shuffle(&mut self, random_source: impl Fn(usize) -> usize) {
        for i in (1..self.len()).rev() {
            self.cards.swap(i, random_source(i + 1));
        }
    }
}

impl Default for CardCollection {
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
    /// use poppy_plays::deck::CardCollection;
    /// let hand : Result<CardCollection, _> = "AdKd".try_into();
    /// assert!(hand.is_ok());
    /// ```
    ///
    /// Anything that can't be parsed will return an error.
    ///
    /// ```
    /// use std::convert::TryInto;
    /// use poppy_plays::deck::CardCollection;
    /// let hand : Result<CardCollection, _> = "AdKx".try_into();
    /// assert!(hand.is_err());
    /// ```
    fn try_from(s: &str) -> Result<Self, Self::Error> {
        // Get the chars iterator.
        let mut chars = s.chars();
        // Where we will put the cards
        //
        // We make the assumption that the hands will have 2 plus five cards.
        let mut cards: HashSet<Card> = HashSet::new();

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
                if !cards.insert(c) {
                    // If this card is already in the set then error out.
                    return Err(format!("This card has already been added {}", c));
                }
            }
        }

        if chars.next() != None {
            return Err(String::from("Extra un-used chars found."));
        }

        Ok(Self {
            cards: cards.into_iter().collect(),
        })
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
