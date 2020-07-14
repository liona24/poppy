//! This module provides types and enums to represent cards and collections thereof.
pub mod card;
mod card_collection;
mod rank;

pub use card::Card;
pub use card_collection::CardCollection;
pub use rank::{Rank, Rankable};

/// A trait representing a default card deck.
pub trait Deck {
    /// Deal a card from the deck.
    /// The card should never be dealt again.
    fn deal(&mut self) -> Option<Card>;
    /// Check whether this deck is empty.
    fn is_empty(&self) -> bool;
}
