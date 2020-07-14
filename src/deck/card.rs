//! This module contains means to represent cards and their respective values and suits
//!
//! ## LICENSE
//! This is adopted from https://github.com/elliottneilclark/rs-poker
//!
//! Copyright 2019 Elliott Clark
//!
//! Licensed under the Apache License, Version 2.0 (the "License");
//! you may not use this file except in compliance with the License.
//! You may obtain a copy of the License at
//!
//! http://www.apache.org/licenses/LICENSE-2.0
//!
//! Unless required by applicable law or agreed to in writing, software
//! distributed under the License is distributed on an "AS IS" BASIS,
//! WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
//! See the License for the specific language governing permissions and
//! limitations under the License.

use std::cmp;
use std::fmt;

/// Card rank or value.
/// This is basically the face value - 2
#[derive(PartialEq, PartialOrd, Eq, Ord, Debug, Clone, Copy, Hash)]
pub enum Value {
    /// 2
    Two = 0,
    /// 3
    Three = 1,
    /// 4
    Four = 2,
    /// 5
    Five = 3,
    /// 6
    Six = 4,
    /// 7
    Seven = 5,
    /// 8
    Eight = 6,
    /// 9
    Nine = 7,
    /// T
    Ten = 8,
    /// J
    Jack = 9,
    /// Q
    Queen = 10,
    /// K
    King = 11,
    /// A
    Ace = 12,
}

/// Constant of all the values.
/// This is what `Value::values()` returns
const VALUES: [Value; 13] = [
    Value::Two,
    Value::Three,
    Value::Four,
    Value::Five,
    Value::Six,
    Value::Seven,
    Value::Eight,
    Value::Nine,
    Value::Ten,
    Value::Jack,
    Value::Queen,
    Value::King,
    Value::Ace,
];

impl Value {
    /// Get all of the `Value`'s that are possible.
    /// This is used to iterate through all possible
    /// values when creating a new deck, or
    /// generating all possible starting hands.
    pub fn values() -> [Self; 13] {
        VALUES
    }

    /// Given a character parse that char into a value.
    /// Case is ignored as long as the char is in the ascii range (It should be).
    /// @returns None if there's no value there.
    ///
    /// # Examples
    ///
    /// ```
    /// use poppy::deck::card::Value;
    /// assert_eq!(Value::Ace, Value::from_char('A').unwrap());
    /// ```
    pub fn from_char(c: char) -> Option<Self> {
        match c.to_ascii_uppercase() {
            'A' => Some(Self::Ace),
            'K' => Some(Self::King),
            'Q' => Some(Self::Queen),
            'J' => Some(Self::Jack),
            'T' => Some(Self::Ten),
            '9' => Some(Self::Nine),
            '8' => Some(Self::Eight),
            '7' => Some(Self::Seven),
            '6' => Some(Self::Six),
            '5' => Some(Self::Five),
            '4' => Some(Self::Four),
            '3' => Some(Self::Three),
            '2' => Some(Self::Two),
            _ => None,
        }
    }

    /// Convert this Value to a char.
    pub fn to_char(self) -> char {
        match self {
            Self::Ace => 'A',
            Self::King => 'K',
            Self::Queen => 'Q',
            Self::Jack => 'J',
            Self::Ten => 'T',
            Self::Nine => '9',
            Self::Eight => '8',
            Self::Seven => '7',
            Self::Six => '6',
            Self::Five => '5',
            Self::Four => '4',
            Self::Three => '3',
            Self::Two => '2',
        }
    }

    /// How card ranks seperate the two values.
    ///
    /// # Examples
    ///
    /// ```
    /// use poppy::deck::card::Value;
    /// assert_eq!(1, Value::Ace.gap(Value::King));
    /// ```
    pub fn gap(self, other: Self) -> u8 {
        let min = cmp::min(self as u8, other as u8);
        let max = cmp::max(self as u8, other as u8);
        max - min
    }
}

/// Enum for the four different suits.
/// While this has support for ordering it's not
/// sensical. The sorting is only there to allow sorting cards.
#[derive(PartialEq, PartialOrd, Eq, Ord, Debug, Clone, Copy, Hash)]
pub enum Suit {
    /// Spades
    Spade = 0,
    /// Clubs
    Club = 1,
    /// Hearts
    Heart = 2,
    /// Diamonds
    Diamond = 3,
}

/// All of the `Suit`'s. This is what `Suit::suits()` returns.
const SUITS: [Suit; 4] = [Suit::Spade, Suit::Club, Suit::Heart, Suit::Diamond];

/// Impl of Suit
///
/// This is just here to provide a list of all `Suit`'s.
impl Suit {
    /// Provide all the Suit's that there are.
    pub fn suits() -> [Self; 4] {
        SUITS
    }

    /// Given a character that represents a suit try and parse that char.
    /// If the char can represent a suit return it.
    ///
    /// # Examples
    ///
    /// ```
    /// use poppy::deck::card::Suit;
    /// let s = Suit::from_char('s');
    /// assert_eq!(Some(Suit::Spade), s);
    /// ```
    ///
    /// ```
    /// use poppy::deck::card::Suit;
    /// let s = Suit::from_char('X');
    /// assert_eq!(None, s);
    /// ```
    pub fn from_char(s: char) -> Option<Self> {
        match s.to_ascii_lowercase() {
            'd' => Some(Self::Diamond),
            's' => Some(Self::Spade),
            'h' => Some(Self::Heart),
            'c' => Some(Self::Club),
            _ => None,
        }
    }

    /// This Suit to a character.
    pub fn to_char(self) -> char {
        match self {
            Self::Diamond => 'd',
            Self::Spade => 's',
            Self::Heart => 'h',
            Self::Club => 'c',
        }
    }
}

/// The main struct of this library.
/// This is a carrier for Suit and Value combined.
#[derive(PartialEq, PartialOrd, Eq, Ord, Debug, Clone, Copy, Hash)]
pub struct Card {
    /// The face value of this card.
    pub value: Value,
    /// The suit of this card.
    pub suit: Suit,
}

impl fmt::Display for Card {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}{}", self.value.to_char(), self.suit.to_char())
    }
}

impl Default for Card {
    /// This is a pure convenience implementation. The default card is ace of spades.
    fn default() -> Self {
        Self {
            value: Value::Ace,
            suit: Suit::Spade,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem;

    #[test]
    fn test_constructor() {
        let c = Card {
            value: Value::Three,
            suit: Suit::Spade,
        };
        assert_eq!(Suit::Spade, c.suit);
        assert_eq!(Value::Three, c.value);
    }

    #[test]
    fn test_compare() {
        let c1 = Card {
            value: Value::Three,
            suit: Suit::Spade,
        };
        let c2 = Card {
            value: Value::Four,
            suit: Suit::Spade,
        };
        let c3 = Card {
            value: Value::Four,
            suit: Suit::Club,
        };

        // Make sure that the values are ordered
        assert!(c1 < c2);
        assert!(c2 > c1);
        // Make sure that suit is used.
        assert!(c3 > c2);
    }

    #[test]
    fn test_value_cmp() {
        assert!(Value::Two < Value::Ace);
        assert!(Value::King < Value::Ace);
        assert_eq!(Value::Two, Value::Two);
    }

    #[test]
    fn test_size_card() {
        // Card should be really small. Hopefully just two u8's
        assert!(mem::size_of::<Card>() <= 2);
    }

    #[test]
    fn test_size_suit() {
        // One byte for Suit
        assert!(mem::size_of::<Suit>() <= 1);
    }

    #[test]
    fn test_size_value() {
        // One byte for Value
        assert!(mem::size_of::<Value>() <= 1);
    }

    #[test]
    fn test_gap() {
        // test on gap
        assert!(1 == Value::Ace.gap(Value::King));
        // test no gap at the high end
        assert!(0 == Value::Ace.gap(Value::Ace));
        // test no gap at the low end
        assert!(0 == Value::Two.gap(Value::Two));
        // Test one gap at the low end
        assert!(1 == Value::Two.gap(Value::Three));
        // test that ordering doesn't matter
        assert!(1 == Value::Three.gap(Value::Two));
        // Test things that are far apart
        assert!(12 == Value::Ace.gap(Value::Two));
        assert!(12 == Value::Two.gap(Value::Ace));
    }
}
