use crate::actions::{Action, PlayerAction};
use crate::board::Board;
use crate::deck::{Card, Deck};
use crate::player::Player;
use crate::pot::Pot;
use crate::ChipCount;
use std::ops::{Deref, DerefMut};

/// Structure to hold state information about one round of poker played which is visible to each player.
#[derive(Debug, Clone)]
pub struct TransparentState {
    /// The current state of the board
    pub board: Board,

    /// The cards each player is holding
    pub hands: Vec<[Card; 2]>,

    /// The actions taken so far in this round.
    pub actions: Vec<Action>,

    /// The pot for this round
    pub pot: Pot,

    /// The current size of the blinds. The small blind is equal to this size.
    /// The big blind is equal to 2 times `blind_size`
    pub blind_size: ChipCount,

    /// The position of the dealer in this round.
    ///
    /// This player may not be involved
    /// in the action anymore (i.e. folded already)
    pub dealer_position: usize,

    /// The collection of player positions.\
    /// It contains all the currently **active** players in the current round.
    ///
    /// When the round starts the first player in this collection corresponds to
    /// the small blind position, followed by the big blind position etc. The final
    /// player corresponds to the one located at the  dealer position.
    ///
    /// After players take actions, the order of players is ensured, however
    /// not all positions may be present anymore.
    pub player_positions: Vec<usize>,

    /// A vector which contains the remaining stacks of all the players.
    ///
    /// It is indexed by player position (or also referenced to as the player id).
    pub player_stacks: Vec<ChipCount>,

    /// Unique identifier for the current round played.
    pub id: usize,
}

/// Convenience structure wrapping a `TransparentState` for replay purposes.
#[derive(Debug, Clone)]
pub struct CheckpointState {
    state: TransparentState,
}

#[derive(Debug, Clone)]
pub(crate) struct BetRoundState {
    index_of_starting_position: usize,
    i: usize,
    last_raiser: Option<usize>,
    done: bool,
}

impl BetRoundState {
    pub(crate) fn done(&self) -> bool {
        self.done
    }
}

impl CheckpointState {
    pub(crate) fn new(state: TransparentState) -> Self {
        Self { state }
    }
}

impl Deref for CheckpointState {
    type Target = TransparentState;

    fn deref(&self) -> &Self::Target {
        &self.state
    }
}

impl DerefMut for CheckpointState {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.state
    }
}

impl TransparentState {
    pub(crate) fn new(
        blind_size: ChipCount,
        dealer_position: usize,
        player_stacks: Vec<ChipCount>,
    ) -> Self {
        let default_card = Card {
            value: crate::deck::card::Value::Ace,
            suit: crate::deck::card::Suit::Club,
        };
        let hands = vec![[default_card, default_card]; player_stacks.len()];

        Self {
            board: Board::new(),
            hands,
            actions: Vec::new(),
            pot: Pot::new(player_stacks.len()),
            blind_size,
            dealer_position,
            player_positions: generate_player_positions(dealer_position, player_stacks.len()),
            player_stacks,
            id: 0,
        }
    }

    /// Returns the total number of players at the table
    pub fn num_players_total(&self) -> usize {
        self.player_stacks.len()
    }

    /// Returns the number of players still playing the most recent hand
    pub fn num_players(&self) -> usize {
        self.player_positions.len()
    }

    /// Pre-emptively reserves the cards for each player from the given deck
    pub(crate) fn prepare_hands(&mut self, d: &mut impl Deck) {
        for &i in self.player_positions.iter() {
            let c1 = d.deal().expect("Deck should contain enough cards");
            let c2 = d.deal().expect("Deck should contain enough cards");
            self.hands[i] = [c1, c2];
        }
    }

    /// Query the cards which were dealt to the player at the given position.
    ///
    /// Use responsibly.
    pub fn query_cards(&self, player_position: usize) -> [Card; 2] {
        self.hands[player_position]
    }

    /// Resets the internal state, progresses the dealer position and prepares the next round
    pub(crate) fn reset(&mut self) {
        self.dealer_position = (self.dealer_position + 1) % self.num_players_total();
        self.board.clear();
        self.actions.clear();
        self.pot.reset();
        self.player_positions =
            generate_player_positions(self.dealer_position, self.num_players_total());
        self.actions.clear();
        self.id += 1;
    }

    /// Deals the prepared hand to the player with the given id
    pub(crate) fn deal_hand(&mut self, i: usize) -> Action {
        let pos = self.player_positions[i];
        self.mirrored_action(Action::DealHand(pos, self.hands[pos]))
    }

    /// Emits an `Action::StartRound`
    pub(crate) fn start_round(&mut self) -> Action {
        self.mirrored_action(Action::StartRound {
            id: self.id,
            small_blind: self.blind_size,
            big_blind: self.blind_size * 2,
        })
    }

    pub(crate) fn apply_small_blind<P: Player>(&mut self, players: &mut [P]) -> Action {
        let action = self.blind(players, self.player_positions[0], self.blind_size);
        self.mirrored_action(action)
    }

    pub(crate) fn apply_big_blind<P: Player>(&mut self, players: &mut [P]) -> Action {
        let action = self.blind(players, self.player_positions[1], self.blind_size * 2);
        self.mirrored_action(action)
    }

    /// Create a state object which can be used in `step_bet_round` until the bet round finished
    ///
    /// This method shall be used for betting **before** the flop has been dealt.
    pub(crate) fn init_pre_flop_action(&self) -> BetRoundState {
        // pre-flop action starts at big blind + 1
        let i = 2 % self.num_players();
        BetRoundState {
            i,
            index_of_starting_position: i,
            last_raiser: None,
            done: false,
        }
    }

    /// Create a state object which can be used in `step_bet_round` until the bet round finished
    ///
    /// This method shall be used for betting **after** the flop has been dealt.
    pub(crate) fn init_post_flop_action(&self) -> BetRoundState {
        BetRoundState {
            i: 0,
            index_of_starting_position: 0,
            last_raiser: None,
            done: false,
        }
    }

    /// Continue a betting round which is represented by the given `state`.
    ///
    /// This is basically a poor man's iterator.
    /// It shall be exhausted until `state.done() == true`.
    ///
    /// The actual action taken is returned.
    /// This may be `None` if a player has to be skipped.
    pub(crate) fn step_bet_round<P: Player>(
        &mut self,
        state: &mut BetRoundState,
        players: &mut [P],
    ) -> Option<Action> {
        if state.done() {
            return None;
        }

        let pos = self.player_positions[state.i];

        let (action, is_raise) = self.player_action(pos, &mut players[pos]);
        if is_raise {
            state.last_raiser = Some(pos);
        }
        if let Some(Action::Fold(_)) = action {
            self.player_positions.remove(state.i);
            if state.last_raiser.is_none() && state.i == state.index_of_starting_position {
                // this is the special case when pre-flop players only either fold or call to the big-blind
                state.i %= self.num_players();
                state.index_of_starting_position = state.i;
                if self.num_players() == 1 {
                    state.done = true;
                    self.pot.end_bet_round();
                }
                return Some(self.mirrored_action(action.unwrap()));
            }
        } else {
            state.i += 1;
        }

        state.i %= self.num_players();
        if Some(self.player_positions[state.i]) == state.last_raiser
            || (state.last_raiser.is_none() && state.i == state.index_of_starting_position)
            || self.num_players() == 1
        {
            self.pot.end_bet_round();
            state.done = true;
        }

        if let Some(action) = action {
            Some(self.mirrored_action(action))
        } else {
            None
        }
    }

    pub(crate) fn deal_flop(&mut self, cards: [Card; 3]) -> Action {
        self.board.deal_flop(cards);
        self.mirrored_action(Action::DealFlop(cards))
    }

    pub(crate) fn deal_turn(&mut self, card: Card) -> Action {
        self.board.deal_turn(card);
        self.mirrored_action(Action::DealTurn(card))
    }

    pub(crate) fn deal_river(&mut self, card: Card) -> Action {
        self.board.deal_river(card);
        self.mirrored_action(Action::DealRiver(card))
    }

    pub(crate) fn end_round(&mut self) -> Action {
        if self.num_players() == 1 {
            // the player left gets the pot
            let pos = *self.player_positions.first().unwrap();
            let win = self.pot.distribute(&self.player_positions)[pos];
            self.player_stacks[pos] += win;

            Action::Win(vec![(pos, win)])
        } else {
            // prepare showdown
            let mut ranked_hands = Vec::new();
            for &i in self.player_positions.iter() {
                ranked_hands.push((self.board.rank_hand(self.hands[i]), i))
            }
            ranked_hands.sort_by_key(|x| x.0.clone());
            let mut wins = Vec::new();

            while let Some((rank, pos)) = ranked_hands.pop() {
                let mut positions = vec![pos];
                while !ranked_hands.is_empty() && ranked_hands.last().unwrap().0 == rank {
                    positions.push(ranked_hands.pop().unwrap().1);
                }

                let won_amounts = self.pot.distribute(&positions);
                for p in positions.into_iter() {
                    let amount = won_amounts[p];
                    wins.push((p, amount));
                    self.player_stacks[p] += amount;
                }

                if self.pot.is_empty() {
                    break;
                }
            }

            Action::Win(wins)
        }
    }

    /// Forces the player at `position` to set a blind of the specified size.
    ///
    /// Takes care of adjusting stack size and pot size. Forces a player All-In if
    /// it has not enough chips available.
    ///
    /// Returns the corresponding action taken
    fn blind<P: Player>(&mut self, players: &mut [P], position: usize, size: ChipCount) -> Action {
        let actual_bet_size;
        let player_action = if self.player_stacks[position] <= size {
            actual_bet_size = self.player_stacks[position];
            PlayerAction::AllIn(self.player_stacks[position])
        } else {
            actual_bet_size = size;
            PlayerAction::Blind(size)
        };

        self.pot.place_chips(position, actual_bet_size);

        // we ignore the return value as there is only one possible action anyway
        // we could consider checking back in order to ensure that players are implemented correctly
        players[position].act(&self, &[player_action]);
        let action_taken =
            Action::from_player_action(player_action, position, self.player_stacks[position]);

        self.player_stacks[position] -= actual_bet_size;
        action_taken
    }

    /// Setup possible actions for player at the given position.
    ///
    /// This function returns a pair of the action taken (if any) and a boolean indicating if the action taken can be considered a raise (i.e. Bet, Raise, AllIn which raised).
    /// The only case in which no action is taken, is if the player at the given position does not have any chips left.
    fn player_action(
        &mut self,
        position: usize,
        player: &mut impl Player,
    ) -> (Option<Action>, bool) {
        let stack = self.player_stacks[position];
        if stack == 0 {
            return (None, false);
        }

        let req_bet = self.pot.required_bet_size(position);
        let min_raise = std::cmp::max(self.pot.last_raise_amount(), self.blind_size * 2) + req_bet;

        let mut possible_actions = vec![PlayerAction::AllIn(stack)];

        if req_bet == 0 {
            possible_actions.push(PlayerAction::Check);
        } else {
            possible_actions.push(PlayerAction::Fold);
            if req_bet < stack {
                possible_actions.push(PlayerAction::Call(req_bet));
            }
        }

        if min_raise < stack {
            if req_bet == 0 {
                possible_actions.push(PlayerAction::Bet(min_raise));
            } else {
                possible_actions.push(PlayerAction::Raise(min_raise));
            }
        }

        let action = player.act(&self, &possible_actions);
        let action = Action::from_player_action(action, position, stack);

        let actual_bet_size = match action {
            Action::Bet(_, c) | Action::Raise(_, c) | Action::Call(_, c) | Action::AllIn(_, c) => {
                Some(c)
            }
            _ => None,
        };

        let is_raise = if let Some(actual_bet_size) = actual_bet_size {
            self.player_stacks[position] -= actual_bet_size;
            self.pot.place_chips(position, actual_bet_size)
        } else {
            false
        };

        (Some(action), is_raise)
    }

    fn mirrored_action(&mut self, a: Action) -> Action {
        self.actions.push(a.clone());
        a
    }
}

fn generate_player_positions(dealer_position: usize, num_players: usize) -> Vec<usize> {
    (0..num_players)
        .map(|x| (x + 1 + dealer_position) % num_players)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock::MockPlayer;

    fn set_equal<T: PartialEq + Clone>(c1: &[T], c2: &[T]) -> bool {
        let mut c2 = c2.to_vec();
        for a in c1.iter() {
            let len_before = c2.len();
            c2.retain(|x| x != a);
            if c2.len() != len_before - 1 {
                return false;
            }
        }

        c2.is_empty()
    }

    fn set_contains<T: PartialEq + Clone>(set: &[T], contained: &[T]) -> bool {
        let mut set = set.to_vec();
        for a in contained.iter() {
            let len_before = set.len();
            set.retain(|x| x != a);
            if set.len() != len_before - 1 {
                return false;
            }
        }

        true
    }

    #[test]
    fn test_generate_player_positions() {
        assert_eq!(generate_player_positions(1, 3), vec![2, 0, 1]);
    }

    #[test]
    fn test_blind() {
        let mut state = TransparentState::new(2, 0, vec![10, 10, 10]);
        let mut players = vec![
            MockPlayer::new(vec![]),
            MockPlayer::new(vec![PlayerAction::Blind(2)]),
            MockPlayer::new(vec![]),
        ];
        assert_eq!(state.blind(&mut players, 1, 2), Action::Blind(1, 2));
        assert_eq!(state.pot.total_bet_size(), 2);
        assert_eq!(state.player_stacks, vec![10, 8, 10]);
    }

    #[test]
    fn test_blind_if_allin() {
        let mut state = TransparentState::new(2, 0, vec![10, 1, 10]);
        let mut players = vec![
            MockPlayer::new(vec![]),
            MockPlayer::new(vec![PlayerAction::AllIn(2)]),
            MockPlayer::new(vec![]),
        ];
        assert_eq!(state.blind(&mut players, 1, 2), Action::AllIn(1, 1));
        assert_eq!(state.pot.total_bet_size(), 1);
        assert_eq!(state.player_stacks, vec![10, 0, 10]);
    }

    #[test]
    fn test_small_blind() {
        let mut state = TransparentState::new(2, 2, vec![10, 10, 10]);
        let mut players = vec![
            MockPlayer::new(vec![PlayerAction::Blind(2)]),
            MockPlayer::new(vec![]),
            MockPlayer::new(vec![]),
        ];
        state.apply_small_blind(&mut players);
    }

    #[test]
    fn test_big_blind() {
        let mut state = TransparentState::new(2, 1, vec![10, 10, 10]);
        let mut players = vec![
            MockPlayer::new(vec![PlayerAction::Blind(4)]),
            MockPlayer::new(vec![]),
            MockPlayer::new(vec![]),
        ];
        state.apply_big_blind(&mut players);
    }

    #[test]
    fn test_player_action_call() {
        let mut state = TransparentState::new(2, 0, vec![10, 10, 10]);
        let mut players = vec![
            MockPlayer::new(vec![PlayerAction::Call(4)]),
            MockPlayer::new(vec![PlayerAction::Blind(2)]), // Small
            MockPlayer::new(vec![PlayerAction::Blind(4)]), // Big
        ];
        state.apply_small_blind(&mut players);
        state.apply_big_blind(&mut players);
        let (action, is_raise) = state.player_action(0, &mut players[0]);
        assert!(!is_raise);

        assert!(set_equal(
            &players[0].last_possible_actions,
            &[
                PlayerAction::Fold,
                PlayerAction::Call(4),
                PlayerAction::Raise(8),
                PlayerAction::AllIn(10)
            ]
        ));
        assert_eq!(state.player_stacks, vec![6, 8, 6]);
        assert_eq!(action, Some(Action::Call(0, 4)));
        assert_eq!(state.pot.total_size(), 10);
    }

    #[test]
    fn test_player_action_bet() {
        let mut state = TransparentState::new(2, 0, vec![10, 10, 10]);
        let mut players = vec![
            MockPlayer::new(vec![PlayerAction::Call(4)]),
            MockPlayer::new(vec![PlayerAction::Blind(2), PlayerAction::Call(2)]), // Small
            MockPlayer::new(vec![PlayerAction::Blind(4), PlayerAction::Bet(5)]),  // Big
        ];
        state.apply_small_blind(&mut players);
        state.apply_big_blind(&mut players);
        state.player_action(0, &mut players[0]);
        state.player_action(1, &mut players[1]);
        let (action, is_raise) = state.player_action(2, &mut players[2]);
        assert!(is_raise);

        assert!(set_equal(
            &players[2].last_possible_actions,
            &[
                PlayerAction::Check,
                PlayerAction::Bet(4),
                PlayerAction::AllIn(6)
            ]
        ));
        assert_eq!(state.player_stacks, vec![6, 6, 1]);
        assert_eq!(action, Some(Action::Bet(2, 5)));
        assert_eq!(state.pot.total_size(), 17);
    }

    #[test]
    fn test_player_action_raise() {
        let mut state = TransparentState::new(2, 0, vec![10, 10, 10]);
        let mut players = vec![
            MockPlayer::new(vec![PlayerAction::Call(4)]),
            MockPlayer::new(vec![PlayerAction::Blind(2), PlayerAction::Raise(7)]), // Small
            MockPlayer::new(vec![PlayerAction::Blind(4)]),                         // Big
        ];
        state.apply_small_blind(&mut players);
        state.apply_big_blind(&mut players);
        state.player_action(0, &mut players[0]);
        let (action, is_raise) = state.player_action(1, &mut players[1]);
        assert!(is_raise);

        assert!(set_equal(
            &players[1].last_possible_actions,
            &[
                PlayerAction::Fold,
                PlayerAction::Call(2),
                PlayerAction::Raise(6),
                PlayerAction::AllIn(8)
            ]
        ));
        assert_eq!(state.player_stacks, vec![6, 1, 6]);
        assert_eq!(action, Some(Action::Raise(1, 7)),);
        assert_eq!(state.pot.total_size(), 17);
    }

    #[test]
    fn test_player_action_allin() {
        let mut state = TransparentState::new(2, 0, vec![4, 10, 10]);
        let mut players = vec![
            MockPlayer::new(vec![PlayerAction::AllIn(4)]),
            MockPlayer::new(vec![PlayerAction::Blind(2), PlayerAction::Raise(10)]), // Small
            MockPlayer::new(vec![PlayerAction::Blind(4)]),                          // Big
        ];
        state.apply_small_blind(&mut players);
        state.apply_big_blind(&mut players);
        let (first_action, first_is_raise) = state.player_action(0, &mut players[0]);
        let (secnd_action, secnd_is_raise) = state.player_action(1, &mut players[1]);

        assert!(!first_is_raise);
        assert!(secnd_is_raise);

        assert!(set_equal(
            &players[0].last_possible_actions,
            &[PlayerAction::Fold, PlayerAction::AllIn(4)]
        ));
        assert!(set_equal(
            &players[1].last_possible_actions,
            &[
                PlayerAction::Fold,
                PlayerAction::Call(2),
                PlayerAction::Raise(6),
                PlayerAction::AllIn(8)
            ]
        ));

        assert_eq!(state.player_stacks, vec![0, 0, 6]);
        assert_eq!(first_action, Some(Action::AllIn(0, 4)));
        assert_eq!(secnd_action, Some(Action::AllIn(1, 8)));
        assert_eq!(state.pot.total_size(), 18);
    }

    #[test]
    fn test_player_action_check() {
        let mut state = TransparentState::new(2, 0, vec![10, 10, 10]);
        let mut players = vec![
            MockPlayer::new(vec![PlayerAction::Call(4)]),
            MockPlayer::new(vec![PlayerAction::Blind(2), PlayerAction::Call(2)]), // Small
            MockPlayer::new(vec![PlayerAction::Blind(4), PlayerAction::Check]),   // Big
        ];
        state.apply_small_blind(&mut players);
        state.apply_big_blind(&mut players);
        state.player_action(0, &mut players[0]);
        state.player_action(1, &mut players[1]);
        let (action, is_raise) = state.player_action(2, &mut players[2]);
        assert!(!is_raise);

        assert_eq!(state.player_stacks, vec![6, 6, 6]);
        assert_eq!(action, Some(Action::Check(2)));
        assert_eq!(state.pot.total_size(), 12);
    }

    #[test]
    fn test_player_action_fold() {
        let mut state = TransparentState::new(2, 0, vec![10, 10, 10]);
        let mut players = vec![
            MockPlayer::new(vec![PlayerAction::Fold]),
            MockPlayer::new(vec![PlayerAction::Blind(2)]), // Small
            MockPlayer::new(vec![PlayerAction::Blind(4)]), // Big
        ];
        state.apply_small_blind(&mut players);
        state.apply_big_blind(&mut players);
        let (action, is_raise) = state.player_action(0, &mut players[0]);
        assert!(!is_raise);

        assert_eq!(state.player_stacks, vec![10, 8, 6]);
        assert_eq!(action, Some(Action::Fold(0)));
        assert_eq!(state.pot.total_size(), 6);
    }

    #[test]
    fn test_player_action_ignores_player_if_allin() {
        let mut state = TransparentState::new(2, 0, vec![10, 10, 10]);
        let mut players = vec![
            MockPlayer::new(vec![PlayerAction::AllIn(10)]),
            MockPlayer::new(vec![PlayerAction::Blind(2)]), // Small
            MockPlayer::new(vec![PlayerAction::Blind(4)]), // Big
        ];
        state.apply_small_blind(&mut players);
        state.apply_big_blind(&mut players);
        let (first_action, first_is_raise) = state.player_action(0, &mut players[0]);
        let (secnd_action, secnd_is_raise) = state.player_action(0, &mut players[0]);
        assert!(first_is_raise);
        assert!(!secnd_is_raise);

        assert_eq!(state.player_stacks, vec![0, 8, 6]);
        assert_eq!(first_action, Some(Action::AllIn(0, 10)));
        assert!(secnd_action.is_none());
        assert_eq!(state.pot.total_size(), 16);
    }

    #[test]
    fn test_min_bet_size() {
        let mut state = TransparentState::new(2, 0, vec![1000, 1000, 1000]);
        let mut players = vec![
            MockPlayer::new(vec![PlayerAction::Call(9), PlayerAction::Raise(6 + 6 + 1)]),
            MockPlayer::new(vec![PlayerAction::Bet(4), PlayerAction::Raise(5 + 5 + 1)]), // Small
            MockPlayer::new(vec![PlayerAction::Raise(4 + 4 + 1), PlayerAction::Call(6)]), // Big
        ];
        state.player_action(1, &mut players[1]); // Bet(4)
        assert!(set_contains(
            &players[1].last_possible_actions,
            &[PlayerAction::Bet(4)]
        ));

        state.player_action(2, &mut players[2]); // Raise(4+4+1)
        assert!(set_contains(
            &players[2].last_possible_actions,
            &[PlayerAction::Raise(8)]
        ));

        state.player_action(0, &mut players[0]); // Call(9)

        state.player_action(1, &mut players[1]); // Raise(5+5+1)
        assert!(set_contains(
            &players[1].last_possible_actions,
            &[PlayerAction::Raise(10)]
        ));

        state.player_action(2, &mut players[2]); // Call(6)

        state.player_action(0, &mut players[0]); // Raise(6+6+1)
        assert!(set_contains(
            &players[0].last_possible_actions,
            &[PlayerAction::Raise(12)]
        ));
    }

    #[test]
    fn test_bet_round_with_remaining_players_after() {
        let mut state = TransparentState::new(2, 3, vec![1000, 1000, 30, 1000]);
        let mut players = vec![
            MockPlayer::new(vec![
                PlayerAction::Check,
                PlayerAction::Raise(12),
                PlayerAction::Raise(48),
            ]), // Small
            MockPlayer::new(vec![PlayerAction::Bet(6), PlayerAction::Fold]), // Big
            MockPlayer::new(vec![PlayerAction::Call(6), PlayerAction::AllIn(24)]),
            MockPlayer::new(vec![
                PlayerAction::Call(6),
                PlayerAction::Call(24),
                PlayerAction::Call(24),
            ]),
        ];
        let mut s = state.init_post_flop_action();
        while !s.done {
            state.step_bet_round(&mut s, &mut players);
        }
        assert!(state.num_players() > 0);

        assert_eq!(
            &state.actions,
            &[
                Action::Check(0),
                Action::Bet(1, 6),
                Action::Call(2, 6),
                Action::Call(3, 6),
                Action::Raise(0, 12),
                Action::Fold(1),
                Action::AllIn(2, 24),
                Action::Call(3, 24),
                Action::Raise(0, 48),
                Action::Call(3, 24)
            ]
        );
        assert_eq!(&state.player_positions, &[0, 2, 3]);
        assert_eq!(state.num_players(), 3);
        assert_eq!(state.num_players_total(), 4);
    }

    #[test]
    fn test_bet_round_all_but_one_fold() {
        let mut state = TransparentState::new(3, 3, vec![1000, 1000, 30, 1000]);
        let mut players = vec![
            MockPlayer::new(vec![PlayerAction::Check, PlayerAction::Fold]),
            MockPlayer::new(vec![PlayerAction::Bet(6)]),
            MockPlayer::new(vec![PlayerAction::Fold]),
            MockPlayer::new(vec![PlayerAction::Fold]),
        ];
        let mut s = state.init_post_flop_action();
        while !s.done {
            state.step_bet_round(&mut s, &mut players);
        }
        assert_eq!(&state.player_positions, &[1]);
    }

    #[test]
    fn test_apply_pre_flop_action() {
        // we basically only want to test that the correct position starts
        let mut state = TransparentState::new(6, 0, vec![1000, 1000, 30, 1000]);
        let mut players = vec![
            MockPlayer::new(vec![PlayerAction::Fold]),
            MockPlayer::new(vec![PlayerAction::Blind(6), PlayerAction::Call(6)]),
            MockPlayer::new(vec![PlayerAction::Blind(12), PlayerAction::Check]),
            MockPlayer::new(vec![PlayerAction::Fold]),
        ];
        state.apply_small_blind(&mut players);
        state.apply_big_blind(&mut players);

        let mut s = state.init_pre_flop_action();
        while !s.done {
            state.step_bet_round(&mut s, &mut players);
        }
        assert!(state.num_players() > 1);

        assert_eq!(
            &state.actions,
            &[
                Action::Blind(1, 6),
                Action::Blind(2, 12),
                Action::Fold(3),
                Action::Fold(0),
                Action::Call(1, 6),
                Action::Check(2)
            ]
        );
    }

    #[test]
    fn test_big_blind_will_be_ignored_if_all_players_fold() {
        let mut state = TransparentState::new(6, 0, vec![1000, 1000, 30, 1000]);
        let mut players = vec![
            MockPlayer::new(vec![PlayerAction::Fold]),
            MockPlayer::new(vec![PlayerAction::Blind(6), PlayerAction::Fold]),
            MockPlayer::new(vec![PlayerAction::Blind(12)]),
            MockPlayer::new(vec![PlayerAction::Fold]),
        ];
        state.apply_small_blind(&mut players);
        state.apply_big_blind(&mut players);

        let mut s = state.init_pre_flop_action();
        while !s.done {
            println!("{:?}", state.step_bet_round(&mut s, &mut players));
        }
        assert!(state.num_players() == 1);

        assert_eq!(
            &state.actions,
            &[
                Action::Blind(1, 6),
                Action::Blind(2, 12),
                Action::Fold(3),
                Action::Fold(0),
                Action::Fold(1)
            ]
        );
    }

    #[test]
    fn test_apply_post_flop_action() {
        // we basically only want to test that the correct position starts
        let mut state = TransparentState::new(0, 2, vec![1000, 1000, 30, 1000]);
        let mut players = vec![
            MockPlayer::new(vec![PlayerAction::Check]),
            MockPlayer::new(vec![PlayerAction::Check]),
            MockPlayer::new(vec![PlayerAction::Check]),
            MockPlayer::new(vec![PlayerAction::Check]),
        ];
        let mut s = state.init_post_flop_action();
        while !s.done {
            state.step_bet_round(&mut s, &mut players);
        }
        assert!(state.num_players() > 1);

        assert_eq!(
            &state.actions,
            &[
                Action::Check(3),
                Action::Check(0),
                Action::Check(1),
                Action::Check(2)
            ]
        );
    }

    #[test]
    fn test_deal_cards() {
        let mut state = TransparentState::new(0, 3, vec![1000, 1000, 30, 1000]);
        let c1 = Card {
            value: crate::deck::card::Value::Ace,
            suit: crate::deck::card::Suit::Club,
        };
        let c2 = Card {
            value: crate::deck::card::Value::Ace,
            suit: crate::deck::card::Suit::Diamond,
        };
        let c3 = Card {
            value: crate::deck::card::Value::Ace,
            suit: crate::deck::card::Suit::Spade,
        };
        let c4 = Card {
            value: crate::deck::card::Value::Ace,
            suit: crate::deck::card::Suit::Heart,
        };
        let c5 = Card {
            value: crate::deck::card::Value::Two,
            suit: crate::deck::card::Suit::Heart,
        };
        state.deal_flop([c1, c2, c3]);
        state.deal_turn(c4);
        state.deal_river(c5);

        assert_eq!(
            &state.actions,
            &[
                Action::DealFlop([c1, c2, c3]),
                Action::DealTurn(c4),
                Action::DealRiver(c5)
            ]
        );
        assert_eq!(state.board.all_cards(), &[c1, c2, c3, c4, c5]);
    }

    #[test]
    fn test_reset_state() {
        let mut state = TransparentState::new(3, 0, vec![1000, 1000, 30, 1000]);
        let mut players = vec![
            MockPlayer::new(vec![PlayerAction::Fold]),
            MockPlayer::new(vec![PlayerAction::Blind(6), PlayerAction::Fold]),
            MockPlayer::new(vec![PlayerAction::Blind(12)]),
            MockPlayer::new(vec![PlayerAction::Fold]),
        ];
        state.apply_small_blind(&mut players);
        state.apply_big_blind(&mut players);
        let mut s = state.init_pre_flop_action();
        while !s.done {
            state.step_bet_round(&mut s, &mut players);
        }

        state.end_round();
        state.reset();

        assert!(state.actions.is_empty());
        assert_eq!(state.pot.total_size(), 0);
        assert!(state.board.all_cards().is_empty());
        assert_eq!(state.player_positions, [2, 3, 0, 1]);
    }
}
