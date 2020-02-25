//! TODO crate docs
#![warn(missing_docs)]
#![deny(unsafe_code)]

/// A unit for counting chips.
///
/// This should be considered as "the number of chips of minimal value".
/// This crate abstracts all associated values of chips. The only unit used
/// is this `ChipCount`
pub type ChipCount = u32;

pub mod actions;
mod board;
mod pot;
mod player;
mod state;
mod table;

pub use board::Board;
pub use pot::Pot;
pub use player::Player;
pub use table::{Table, BlindPolicy};
pub use state::TransparentState;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
