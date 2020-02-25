use crate::actions::{Action, PlayerAction};
use crate::board::Board;
use crate::pot::Pot;
use crate::ChipCount;
use crate::player::Player;
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
