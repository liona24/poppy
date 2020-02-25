//! This module contains actions for players and the game itself.
//!
//! The game uses `Action`s to log progress. Everything what happens at the
//! virtual poker table can be reconstructed using a stream of actions.
//!
//! This module also exposes a higher level abstraction of so-called `PlayerAction`s,
//! which are a player's way of interacting.
use crate::ChipCount;
use rs_poker::core::Card;

/// An `Action` is internally used to alter the game state. Using a stream of
/// actions each round of poker played can be recovered completely.
///
/// You may want to use these for logging purposes.
///
/// Usually the first argument corresponds to position the player who has caused the action (or who can be associated with this action) resides.
///
#[derive(Debug, Copy, Clone)]
pub enum Action {
    /// Indicates the start of the round.
    StartRound {
        /// An unique identifier for this round.
        id: usize,
        /// The size of the big blind for this round.
        big_blind: ChipCount,
        /// The size of the small blind for this round.
        small_blind: ChipCount,
    },
    /// Indicates that the blind size increased by the associated chip count.
    IncreaseBlind(ChipCount),
    /// Indicates that the player at the given location paid a blind of the given size.
    Blind(usize, ChipCount),
    /// Indicates that the player at given location was dealt the given hand.
    DealHand(usize, [Card; 2]),
    /// Indicates that the given cards were dealt as flop cards.
    DealFlop([Card; 3]),
    /// Indicates that the given card was played as the turn card.
    DealTurn(Card),
    /// Indicates that the given card was played as the river card.
    DealRiver(Card),
    /// Indicates that the player at the given location checked.
    Check(usize),
    /// Indicates that the player at the given location called for the given amount of chips.
    Call(usize, ChipCount),
    /// Indicates that the player at the given location raised for the given amount of chips.
    Raise(usize, ChipCount),
    /// Indicates that the player at the given location pushed all-in for the given amount of chips.
    AllIn(usize, ChipCount),
    /// Indicates that the player at the given location placed a bet of the given amount of chips.
    Bet(usize, ChipCount),
    /// Indicates that the player at the given location folded his hand.
    Fold(usize),
    /// Indicates that the player at the given location won the given amount of chips.
    Win(usize, ChipCount),
    /// Indicates that one round ended.
    EndRound,
}

/// An action a player can cause.
///
/// This represents the means of interaction of a player and the game.
///
/// In order to interact with the game the player is represented with a set of `PlayerAction`s.
/// The player may choose any and (after validation) the given action will be performed.
///
/// For some actions `ChipCounts` are associated. Depending on context they
/// usually represent the minimum number of chips required to perform that action.
#[derive(Debug, Copy, Clone)]
pub enum PlayerAction {
    /// Indicates that the player has to pay a blind of the given size.
    ///
    /// The given chip count should not be increased.
    Blind(ChipCount),
    /// Indicates that the player may check.
    Check,
    /// Indicates that the player may call the given amount.
    ///
    /// The given chip count should not be increased.
    Call(ChipCount),
    /// Indicates that the player may raise by the given minimum amount.
    ///
    /// The given chip count may be increased.\
    /// If it is greater or equal to the causing player's stack it will be
    /// auto-converted into an `AllIn(stack_size)`
    Raise(ChipCount),
    /// Indicates that the player may push all-in.
    ///
    /// The given chip count should not be increased.
    AllIn(ChipCount),
    /// Indicates that the player may place a bet of the given minimum size.
    ///
    /// The given chip count may be increased.
    Bet(ChipCount),
    /// Indicates that the player may fold its hand.
    Fold,
}

macro_rules! validated {
    ($stack:expr, $pos:expr, $bet:expr, $variant:tt) => {
        if $bet >= $stack {
            Action::AllIn($pos, $stack)
        } else {
            Action::$variant($pos, $bet)
        }
    };
}

impl Action {
    pub(crate) fn from_player_action(player_action: PlayerAction, player_position: usize, player_stack: ChipCount) -> Self {
        match player_action {
            PlayerAction::Blind(c) => validated!(player_stack, player_position, c, Blind),
            PlayerAction::Check => Action::Check(player_position),
            PlayerAction::Call(c) => validated!(player_stack, player_position, c, Call),
            PlayerAction::Raise(c) => validated!(player_stack, player_position, c, Raise),
            PlayerAction::AllIn(c) => validated!(player_stack, player_position, c, AllIn),
            PlayerAction::Bet(c) => validated!(player_stack, player_position, c, Bet),
            PlayerAction::Fold => Action::Fold(player_position),
        }
    }
}
