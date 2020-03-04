use crate::actions::{Action, PlayerAction};
use crate::board::Board;
use crate::player::Player;
use crate::pot::Pot;
use crate::ChipCount;
use rs_poker::core::Card;

/// Structure to hold state information about one round of poker played which is visible to each player.
#[derive(Debug, Clone)]
pub struct TransparentState {
    /// The current state of the board
    pub board: Board,
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
}

impl TransparentState {
    pub(crate) fn new(
        blind_size: ChipCount,
        dealer_position: usize,
        player_stacks: Vec<ChipCount>,
    ) -> Self {
        Self {
            board: Board::new(),
            actions: Vec::new(),
            pot: Pot::new(player_stacks.len()),
            blind_size,
            dealer_position,
            player_positions: generate_player_positions(dealer_position, player_stacks.len()),
            player_stacks,
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

    pub(crate) fn apply_small_blind<P: Player>(&mut self, players: &mut Vec<P>) {
        let action = self.blind(players, self.player_positions[0], self.blind_size);
        self.actions.push(action);
    }
    pub(crate) fn apply_big_blind<P: Player>(&mut self, players: &mut Vec<P>) {
        let action = self.blind(players, self.player_positions[1], self.blind_size * 2);
        self.actions.push(action);
    }
    pub(crate) fn apply_pre_flop_action<P: Player>(&mut self, players: &mut Vec<P>) -> bool {
        // pre-flop action starts at big blind + 1
        let i = 2 % self.num_players();
        self.bet_round(i, players)
    }
    pub(crate) fn apply_post_flop_action<P: Player>(&mut self, players: &mut Vec<P>) -> bool {
        self.bet_round(0, players)
    }

    pub(crate) fn deal_flop(&mut self, cards: [Card; 3]) {
        self.actions.push(Action::DealFlop(cards));
        self.board.deal_flop(cards);
    }

    pub(crate) fn deal_turn(&mut self, card: Card) {
        self.actions.push(Action::DealTurn(card));
        self.board.deal_turn(card);
    }

    pub(crate) fn deal_river(&mut self, card: Card) {
        self.actions.push(Action::DealRiver(card));
        self.board.deal_river(card);
    }

    pub(crate) fn prepare_next_round(&mut self) {
        self.board.clear();
        self.actions.clear();
        self.pot.reset();
        self.dealer_position = (self.dealer_position + 1) % self.num_players();
        self.player_positions = generate_player_positions(self.dealer_position, self.num_players());
    }

    /// Forces the player at `position` to set a blind of the specified size.
    ///
    /// Takes care of adjusting stack size and pot size. Forces a player All-In if
    /// it has not enough chips available.
    ///
    /// Returns the corresponding action taken
    fn blind<P: Player>(
        &mut self,
        players: &mut Vec<P>,
        position: usize,
        size: ChipCount,
    ) -> Action {
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

    /// Perform one bet round starting with the player having its position at the given index.
    ///
    /// Returns `true` if only one player is left after this bet round (i.e. the round is finished).
    fn bet_round<P: Player>(
        &mut self,
        index_of_starting_position: usize,
        players: &mut Vec<P>,
    ) -> bool {
        let mut i = index_of_starting_position;
        let mut last_raiser = self.player_positions[i];
        loop {
            let pos = self.player_positions[i];
            if self.player_action(pos, &mut players[pos]) {
                last_raiser = pos;
            }
            if let Some(&Action::Fold(_)) = self.actions.last() {
                self.player_positions.remove(i);
            } else {
                i += 1;
            }

            i %= self.player_positions.len();
            if self.player_positions[i] == last_raiser {
                break;
            }
        }

        self.pot.end_bet_round();

        self.player_positions.len() == 1
    }

    /// Setup possible actions for player at the given position.
    ///
    /// This function returns `true` if the action taken can be considered a raise (i.e. Bet, Raise, AllIn which raised)
    fn player_action(&mut self, position: usize, player: &mut impl Player) -> bool {
        let req_bet = self.pot.required_bet_size(position);
        let min_bet = std::cmp::max(self.pot.bet_size_round() * 2, self.blind_size * 2);
        let stack = self.player_stacks[position];

        let mut possible_actions = vec![PlayerAction::AllIn(stack)];

        if req_bet == 0 {
            possible_actions.push(PlayerAction::Check);
        } else {
            possible_actions.push(PlayerAction::Fold);
            if req_bet < stack {
                possible_actions.push(PlayerAction::Call(req_bet));
            }
        }

        if min_bet < stack {
            if self.pot.bet_size_round() == 0 {
                possible_actions.push(PlayerAction::Bet(min_bet));
            } else {
                possible_actions.push(PlayerAction::Raise(min_bet));
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
        self.actions.push(action);

        if let Some(actual_bet_size) = actual_bet_size {
            self.player_stacks[position] -= actual_bet_size;
            self.pot.place_chips(position, actual_bet_size)
        } else {
            false
        }
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
        let mut c2 = contained.to_vec();
        for a in set.iter() {
            let len_before = c2.len();
            c2.retain(|x| x != a);
            if c2.len() != len_before - 1 {
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
        assert!(!state.player_action(0, &mut players[0]));

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
        assert_eq!(
            state.actions.last().expect("Should have last action"),
            &Action::Call(0, 4)
        );
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
        assert!(state.player_action(2, &mut players[2]));

        assert!(set_equal(
            &players[2].last_possible_actions,
            &[
                PlayerAction::Check,
                PlayerAction::Bet(4),
                PlayerAction::AllIn(6)
            ]
        ));
        assert_eq!(state.player_stacks, vec![6, 6, 1]);
        assert_eq!(
            state.actions.last().expect("Should have last action"),
            &Action::Bet(2, 5)
        );
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
        assert!(state.player_action(1, &mut players[1]));

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
        assert_eq!(
            state.actions.last().expect("Should have last action"),
            &Action::Raise(1, 7)
        );
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
        assert!(!state.player_action(0, &mut players[0]));
        assert!(state.player_action(1, &mut players[1]));

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
        assert_eq!(
            &state.actions[state.actions.len() - 3..],
            &[Action::AllIn(0, 4), Action::AllIn(1, 10)]
        );
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
        assert!(!state.player_action(2, &mut players[2]));

        assert_eq!(state.player_stacks, vec![6, 6, 6]);
        assert_eq!(
            state.actions.last().expect("Should have last action"),
            &Action::Check(2)
        );
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
        assert!(!state.player_action(0, &mut players[0]));

        assert_eq!(state.player_stacks, vec![10, 8, 6]);
        assert_eq!(
            state.actions.last().expect("Should have last action"),
            &Action::Fold(0)
        );
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
        assert!(state.player_action(0, &mut players[0]));
        assert!(!state.player_action(0, &mut players[0]));

        assert_eq!(state.player_stacks, vec![0, 8, 6]);
        assert_eq!(
            &state.actions[state.actions.len() - 3..],
            &[Action::Blind(2, 4), Action::AllIn(0, 10)]
        );
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
        assert!(!state.bet_round(0, &mut players));

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
        assert_eq!(&state.player_positions, &[0, 1, 3]);
        assert_eq!(state.num_players(), 3);
        assert_eq!(state.num_players_total(), 4);
    }

    #[test]
    fn test_bet_round_all_but_one_fold() {
        let mut state = TransparentState::new(2, 3, vec![1000, 1000, 30, 1000]);
        let mut players = vec![
            MockPlayer::new(vec![PlayerAction::Check, PlayerAction::Fold]),
            MockPlayer::new(vec![PlayerAction::Bet(6)]),
            MockPlayer::new(vec![PlayerAction::Fold]),
            MockPlayer::new(vec![PlayerAction::Fold]),
        ];
        assert!(state.bet_round(0, &mut players));
        assert_eq!(&state.player_positions, &[1]);
    }

    #[test]
    fn test_apply_pre_flop_action() {
        // we basically only want to test that the correct position starts
        let mut state = TransparentState::new(0, 3, vec![1000, 1000, 30, 1000]);
        let mut players = vec![
            MockPlayer::new(vec![PlayerAction::Fold]),
            MockPlayer::new(vec![PlayerAction::Blind(6), PlayerAction::Call(6)]),
            MockPlayer::new(vec![PlayerAction::Blind(12), PlayerAction::Check]),
            MockPlayer::new(vec![PlayerAction::Fold]),
        ];
        state.apply_small_blind(&mut players);
        state.apply_big_blind(&mut players);
        assert!(!state.apply_pre_flop_action(&mut players));

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
    fn test_apply_pre_flop_action_big_blind_will_be_ignored_if_all_players_fold() {
        let mut state = TransparentState::new(0, 3, vec![1000, 1000, 30, 1000]);
        let mut players = vec![
            MockPlayer::new(vec![PlayerAction::Fold]),
            MockPlayer::new(vec![PlayerAction::Blind(6), PlayerAction::Fold]),
            MockPlayer::new(vec![PlayerAction::Blind(12)]),
            MockPlayer::new(vec![PlayerAction::Fold]),
        ];
        state.apply_small_blind(&mut players);
        state.apply_big_blind(&mut players);
        assert!(state.apply_pre_flop_action(&mut players));

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
        let mut state = TransparentState::new(0, 3, vec![1000, 1000, 30, 1000]);
        let mut players = vec![
            MockPlayer::new(vec![PlayerAction::Check]),
            MockPlayer::new(vec![PlayerAction::Check]),
            MockPlayer::new(vec![PlayerAction::Check]),
            MockPlayer::new(vec![PlayerAction::Check]),
        ];
        assert!(!state.apply_post_flop_action(&mut players));

        assert_eq!(
            &state.actions,
            &[
                Action::Check(1),
                Action::Check(2),
                Action::Check(3),
                Action::Check(0)
            ]
        );
    }

    #[test]
    fn test_deal_cards() {
        let mut state = TransparentState::new(0, 3, vec![1000, 1000, 30, 1000]);
        let c1 = Card {
            value: rs_poker::core::Value::Ace,
            suit: rs_poker::core::Suit::Club,
        };
        let c2 = Card {
            value: rs_poker::core::Value::Ace,
            suit: rs_poker::core::Suit::Diamond,
        };
        let c3 = Card {
            value: rs_poker::core::Value::Ace,
            suit: rs_poker::core::Suit::Spade,
        };
        let c4 = Card {
            value: rs_poker::core::Value::Ace,
            suit: rs_poker::core::Suit::Heart,
        };
        let c5 = Card {
            value: rs_poker::core::Value::Two,
            suit: rs_poker::core::Suit::Heart,
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
        let mut state = TransparentState::new(0, 3, vec![1000, 1000, 30, 1000]);
        let mut players = vec![
            MockPlayer::new(vec![PlayerAction::Fold]),
            MockPlayer::new(vec![PlayerAction::Blind(6), PlayerAction::Fold]),
            MockPlayer::new(vec![PlayerAction::Blind(12)]),
            MockPlayer::new(vec![PlayerAction::Fold]),
        ];
        state.apply_small_blind(&mut players);
        state.apply_big_blind(&mut players);
        state.apply_pre_flop_action(&mut players);

        state.prepare_next_round();

        assert!(state.actions.is_empty());
        assert_eq!(state.pot.total_size(), 0);
        assert!(state.board.all_cards().is_empty());
    }
}
