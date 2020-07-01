//! This module provides convenience wrappers around `rs_poker::FlatDeck` in order
//! to provide more control over shuffling
pub mod card;
mod card_collection;
mod rank;

pub use card::Card;
pub use card_collection::CardCollection;
pub use rank::{Rank, Rankable};

pub trait Deck {
    fn deal(&mut self) -> Option<Card>;
    fn is_empty(&self) -> bool;
}
