//! This module exposes the main player trait.
use crate::actions::PlayerAction;
use crate::ChipCount;
use crate::TransparentState;

/// A trait to be implemented by anyone who is playing
pub trait Player {
    /// This function gets called when this player is seated at a table
    ///
    /// The position argument is a unique identifier which can be used to access vital information
    /// about this player. Relative information can also be extracted, i.e. `position+1` is the player seated to the left
    ///
    /// The initial stack corresponds to the number of chips this player owns.
    fn init(&mut self, position: usize, initial_stack: ChipCount);

    /// This functions gets called everytime the player is required to act.
    ///
    /// The `state` object can be used to query information about the current state of the game.
    /// Note that each player may want to manage some state for itself.
    /// For example, the first time each round the player may want to query the cards it received or the initial stack sizes etc.
    ///
    /// All the actions that this player can take are listed in `possible_actions`.
    /// The player may then choose one of them and return it. The player may alter parameters for that
    /// action if this action allows it. See the documentation for `PlayerAction` for details.
    fn act(&mut self, state: &TransparentState, possible_actions: &[PlayerAction]) -> PlayerAction;

    /// This function gets called when the player lost all the chips and has to leave the table.
    fn bust(&mut self);
}
