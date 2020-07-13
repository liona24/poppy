//! TODO crate docs
#![warn(missing_docs)]
#![deny(unsafe_code)]

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
mod player;
mod pot;
mod state;
mod table;
mod play;

pub use board::Board;
pub use player::Player;
pub use pot::Pot;
pub use state::{TransparentState, CheckpointState};
pub use table::{BlindPolicy, Table};
pub use play::{Round, RoundCheckpoint};

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
