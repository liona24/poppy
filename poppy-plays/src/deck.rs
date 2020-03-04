//! This module provides convenience wrappers around `rs_poker::FlatDeck` in order
//! to provide more control over shuffling
use rs_poker::core::{FlatDeck, Deck};

/// Trait to be implemented by deck generators to be used at a `Table`
pub trait DeckGenerator : Default {
    /// Generate a new shuffled deck
    fn shuffled_deck(&mut self) -> FlatDeck;
}

/// Default deck generator which randomly shuffles a deck using `thread_rng`
pub struct DefaultDeckGenerator;

impl Default for DefaultDeckGenerator {
    fn default() -> Self {
        Self
    }
}

impl DeckGenerator for DefaultDeckGenerator {
    fn shuffled_deck(&mut self) -> FlatDeck {
        let mut deck : FlatDeck = Deck::default().into();
        deck.shuffle();
        deck
    }
}
