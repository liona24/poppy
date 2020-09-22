//! This is a simple library which can be used to implement no-limit Texas Hold'em poker gameplay in rust.
//! Originally built on top of [rs_poker](https://crates.io/crates/rs-poker) the projects diverged quite
//! a lot eventually resulting in a stand-alone library.
//! There are no dependencies required, though adding serialization support is planned (as a feature).
//!
//!
//! The gameplay is built as an iterator.
//! The main design goals were
//! a) being able to present only the valid actions at each point in time to each player
//! b) eventually being able to support simple logging functionality and
//! c) being able to replay rounds starting at any point in time with different players etc.
//!
//! ### Example:
//! ```rust
//! use poppy::prelude::*;
//!
//! #[derive(Debug, Clone)]
//! struct PlayerType;
//!
//! impl Player for PlayerType {
//!     fn init(&mut self, _position: usize, _initial_stack: ChipCount) {
//!         // intitialize some internal state if needed.
//!     }
//!
//!     fn act(
//!         &mut self,
//!         _state: &TransparentState,
//!         possible_actions: &[PlayerAction],
//!     ) -> PlayerAction {
//!         // main interaction callback
//!         // use `state` to retrieve information about game state and choose any of the actions possible
//!         // we will just use a "random" one:
//!
//!         assert!(!possible_actions.is_empty());
//!         possible_actions[0]
//!     }
//!
//!     fn bust(&mut self) {
//!         // callback to de-init this player, called when this player has no chips left
//!         println!(":(");
//!     }
//! }
//!
//! fn main() {
//!     let players = vec![PlayerType {}; 12];
//!     let stack_size = 100;
//!     let blind_size = 1;
//!     let mut table = Table::new(
//!         players.into_iter(),
//!         stack_size,
//!         blind_size,
//!         BlindPolicy::NeverIncrease,
//!     );
//!
//!     // You can shuffle decks as you want to.
//!     // We will use a default un-sorted deck here.
//!     let deck = deck::CardCollection::default();
//!     let mut round_iter = table.play_one_round(deck);
//!
//!     for action_taken in &mut round_iter {
//!         println!("{:?}", action_taken);
//!     }
//! }
//! ```
#![warn(missing_docs)]
#![deny(unsafe_code)]
// I cannot figure out how to explicitly allow for the example only, so I guess we live like this now
#![allow(clippy::needless_doctest_main)]

/// A unit for counting chips.
///
/// This should be considered as "the number of chips of minimal value".
/// This crate abstracts all associated values of chips. The only unit used
/// is this `ChipCount`
pub type ChipCount = u32;

#[cfg(test)]
mod mock;

pub mod actions;
mod board;
pub mod deck;
mod play;
mod player;
mod pot;
mod state;
mod table;

pub use board::Board;
pub use play::{Round, RoundCheckpoint};
pub use player::Player;
pub use pot::Pot;
pub use state::{CheckpointState, TransparentState};
pub use table::{BlindPolicy, Table};

pub mod prelude {
    //! Module containing common imports required for basic usage.
    pub use super::{
        actions::PlayerAction, deck, BlindPolicy, ChipCount, Player, Table, TransparentState,
    };
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
