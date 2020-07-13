use crate::actions::Action;
use crate::deck::{Card, Deck};
use crate::player::Player;
use crate::state::{BetRoundState, TransparentState, CheckpointState};
use std::ops::DerefMut;

/// This enum represents the current stage of the round.
/// It is used for the `Round` structure to hold state information
#[derive(Debug, Clone)]
enum RoundIteratorStage {
    /// The round is about to start
    Init,
    /// The player at the given position is about to receive its cards
    DealHand(usize),
    /// The small blind is about to be placed.
    /// Note that we will deal cards before the blinds.
    SmallBlind,
    /// The big blind is about to be placed.
    /// Note that we will deal cards before the blinds.
    BigBlind,
    /// This represents the state after the blinds have been dealt.
    /// Eventually it ends when either no player is remaining (skips ahead to distribute the pot and sets the stage to `PastEnd`) or
    /// the flop is dealt, which progresses the state to `PostFlop`.
    PostBlind(BetRoundState),
    /// Analogous to `PostBlind`, this represents the state after the flop has been dealt.
    PostFlop(BetRoundState),
    /// Analogous to `PostBlind`, this represents the state after the turn has been dealt.
    PostTurn(BetRoundState),
    /// Analogous to `PostBlind`, this represents the state after the river has been dealt.
    PostRiver(BetRoundState),
    /// The past-end stage, indicating that this round is finished. This stage will loop indefinitely.
    PastEnd,
}

/// Structure to wrap the `TransparentState` into an iterator.
/// Each step taken by the iterator corresponds to one step taken in the round played.
///
/// This iterator object supports freezing for later replay (fast-forward play).
pub struct Round<'a, P: Player, T: DerefMut<Target = TransparentState>> {
    players: &'a mut [P],
    transparent_state: T,
    next_cards: Vec<Card>,
    iterator_stage: RoundIteratorStage,
}

/// This is a checkpoint in a gameplay.
///
/// It is independent of players and tables.
/// You can use it to replay any round at any given time.
pub struct RoundCheckpoint {
    transparent_state: TransparentState,
    next_cards: Vec<Card>,
    iterator_stage: RoundIteratorStage,
}

impl<'a, P: Player, T: DerefMut<Target = TransparentState>> Round<'a, P, T> {
    pub(crate) fn new(
        players: &'a mut [P],
        mut transparent_state: T,
        mut deck: impl Deck,
    ) -> Self {
        transparent_state.prepare_hands(&mut deck);

        // we pre-emptively "fill" the board in order to make serialization less heavy
        let mut next_cards = Vec::with_capacity(5);
        for _ in 0..5 {
            next_cards.push(deck.deal().expect("Deck should contain enough cards"));
        }
        // we will want to preserve order, just for consistency reasons (since we will be popping from back to front)
        next_cards.reverse();

        Self {
            players,
            transparent_state,
            next_cards,
            iterator_stage: RoundIteratorStage::Init,
        }
    }

    /// Clones this state into a `RoundCheckpoint` which allows replay at the current point in time with the currently consumed events.
    pub fn create_checkpoint(&self) -> RoundCheckpoint {
        RoundCheckpoint {
            transparent_state: (*self.transparent_state).clone(),
            next_cards: self.next_cards.clone(),
            iterator_stage: self.iterator_stage.clone(),
        }
    }

    fn end_round(&mut self) -> Action {
        self.iterator_stage = RoundIteratorStage::PastEnd;
        self.transparent_state.end_round()
    }
}

impl<'a, P: Player> Round<'a, P, CheckpointState> {
    pub(crate) fn from_checkpoint(players: &'a mut [P], cp: RoundCheckpoint) -> Self {
        Self {
            players,
            transparent_state: CheckpointState::new(cp.transparent_state),
            next_cards: cp.next_cards,
            iterator_stage: cp.iterator_stage
        }
    }
}

impl<'a, P: Player, T: DerefMut<Target = TransparentState>> Iterator for Round<'a, P, T> {
    type Item = Action;

    /// Progresses the state of the round one step ahead.
    /// All actions taken so far are mirrored into the underlying `TransparentState`
    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.iterator_stage {
            RoundIteratorStage::Init => {
                self.iterator_stage = RoundIteratorStage::DealHand(0);
                Some(self.transparent_state.start_round())
            }
            RoundIteratorStage::DealHand(i) => {
                let i = *i;
                self.iterator_stage = if i + 1 >= self.transparent_state.num_players() {
                    RoundIteratorStage::SmallBlind
                } else {
                    RoundIteratorStage::DealHand(i + 1)
                };
                Some(self.transparent_state.deal_hand(i))
            }
            RoundIteratorStage::SmallBlind => {
                self.iterator_stage = RoundIteratorStage::BigBlind;
                Some(self.transparent_state.apply_small_blind(&mut self.players))
            }
            RoundIteratorStage::BigBlind => {
                self.iterator_stage =
                    RoundIteratorStage::PostBlind(self.transparent_state.init_pre_flop_action());
                Some(self.transparent_state.apply_big_blind(&mut self.players))
            }
            RoundIteratorStage::PostBlind(i) => {
                while !i.done() {
                    let action = self.transparent_state.step_bet_round(i, &mut self.players);
                    if action.is_some() {
                        return action;
                    }
                }

                if self.transparent_state.num_players() == 1 {
                    Some(self.end_round())
                } else {
                    // deal flop
                    self.iterator_stage = RoundIteratorStage::PostFlop(
                        self.transparent_state.init_post_flop_action(),
                    );
                    Some(self.transparent_state.deal_flop([
                        self.next_cards.pop().unwrap(),
                        self.next_cards.pop().unwrap(),
                        self.next_cards.pop().unwrap(),
                    ]))
                }
            }
            RoundIteratorStage::PostFlop(i) => {
                while !i.done() {
                    let action = self.transparent_state.step_bet_round(i, &mut self.players);
                    if action.is_some() {
                        return action;
                    }
                }

                if self.transparent_state.num_players() == 1 {
                    Some(self.end_round())
                } else {
                    // deal turn
                    self.iterator_stage = RoundIteratorStage::PostTurn(
                        self.transparent_state.init_post_flop_action(),
                    );
                    Some(
                        self.transparent_state
                            .deal_turn(self.next_cards.pop().unwrap()),
                    )
                }
            }
            RoundIteratorStage::PostTurn(i) => {
                while !i.done() {
                    let action = self.transparent_state.step_bet_round(i, &mut self.players);
                    if action.is_some() {
                        return action;
                    }
                }

                if self.transparent_state.num_players() == 1 {
                    Some(self.end_round())
                } else {
                    // deal river
                    self.iterator_stage = RoundIteratorStage::PostRiver(
                        self.transparent_state.init_post_flop_action(),
                    );
                    Some(
                        self.transparent_state
                            .deal_river(self.next_cards.pop().unwrap()),
                    )
                }
            }
            RoundIteratorStage::PostRiver(i) => {
                while !i.done() {
                    let action = self.transparent_state.step_bet_round(i, &mut self.players);
                    if action.is_some() {
                        return action;
                    }
                }

                Some(self.end_round())
            }
            RoundIteratorStage::PastEnd => None,
        }
    }
}
