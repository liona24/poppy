use crate::actions::Action;
use crate::deck::{Card, CardCollection, Deck};
use crate::player::Player;
use crate::state::TransparentState;
use crate::ChipCount;
use itertools::{multipeek, Itertools};

pub enum BlindPolicy {
    NeverIncrease,
}

pub struct Table<P> {
    players: Vec<P>,
    blind_policy: BlindPolicy,
    transparent_state: TransparentState,
    last_cards: Vec<[Card; 2]>,
}

impl<P: Player> Table<P> {
    /// Initialize a new table with the given players.
    ///
    /// Each player is assigned an initial stack of the given size.\
    /// The game starts with the given blind size.\
    /// In order to increase blind levels a `BlindPolicy` should be specified.
    ///
    /// The first dealer position will be the first player yielded by the iterator.
    /// After that the players will be seated in order of appearence.
    ///
    /// The number of players has to be in the interval [2, 22]
    pub fn new(
        players: impl Iterator<Item = P>,
        stack_size: ChipCount,
        blind_size: ChipCount,
        blind_policy: BlindPolicy,
    ) -> Self {
        let players: Vec<P> = players.collect();
        assert!(players.len() < 20);

        let stack_sizes = vec![stack_size; players.len()];

        // each player receives a dummy hand of AA. Not that it matters
        let default_card = Card {
            value: crate::deck::card::Value::Ace,
            suit: crate::deck::card::Suit::Club,
        };
        let last_cards = vec![[default_card, default_card]; players.len()];

        Self {
            players,
            blind_policy,
            transparent_state: TransparentState::new(blind_size, 0, stack_sizes),
            last_cards,
        }
    }

    pub fn play_one_round(
        &mut self,
        random_source: impl Fn(usize) -> usize,
    ) -> impl Iterator<Item = Action> {
        let mut state = vec![Action::StartRound {
            id: 0,
            small_blind: self.transparent_state.blind_size,
            big_blind: self.transparent_state.blind_size * 2,
        }];
        let mut deck = CardCollection::default();
        deck.shuffle(random_source);

        for &i in self.transparent_state.player_positions.iter() {
            let c1 = deck.deal().unwrap();
            let c2 = deck.deal().unwrap();
            // note that we do not mirror the cards to the round state
            state.push(Action::DealHand(i, [c1, c2]));
            self.last_cards[i] = [c1, c2];
            self.players[i].receive_cards(c1, c2);
        }

        self.transparent_state.apply_small_blind(&mut self.players);
        self.transparent_state.apply_big_blind(&mut self.players);

        let _ = self
            .transparent_state
            .apply_pre_flop_action(&mut self.players)
            || {
                let flop = [
                    deck.deal().unwrap(),
                    deck.deal().unwrap(),
                    deck.deal().unwrap(),
                ];

                self.transparent_state.deal_flop(flop);
                self.transparent_state
                    .apply_post_flop_action(&mut self.players)
            }
            || {
                let turn = deck.deal().unwrap();
                self.transparent_state.deal_turn(turn);
                self.transparent_state
                    .apply_post_flop_action(&mut self.players)
            }
            || {
                let river = deck.deal().unwrap();
                self.transparent_state.deal_river(river);
                self.transparent_state
                    .apply_post_flop_action(&mut self.players)
            };

        if self.transparent_state.num_players() == 1 {
            // the player left gets the pot
            let pos = *self.transparent_state.player_positions.first().unwrap();
            let win = *self
                .transparent_state
                .pot
                .distribute(&self.transparent_state.player_positions)
                .first()
                .unwrap();
            state.push(Action::Win(pos, win));
            self.transparent_state.player_stacks[pos] += win;
        } else {
            // Showdown
            let mut ranked_hands = Vec::new();
            for &i in self.transparent_state.player_positions.iter() {
                ranked_hands.push((
                    self.transparent_state.board.rank_hand(self.last_cards[i]),
                    i,
                ))
            }
            ranked_hands.sort_by_key(|x| std::cmp::Reverse(x.clone().0));
            let mut ranked_hands = multipeek(ranked_hands.into_iter());

            while let Some((best_rank, _)) = ranked_hands.peek().cloned() {
                let positions: Vec<_> = ranked_hands
                    .peeking_take_while(|(rank, _)| rank == &best_rank)
                    .map(|(_, i)| i)
                    .collect();
                let win = self.transparent_state.pot.distribute(&positions);
                for (p, w) in positions.into_iter().zip(win.into_iter()) {
                    state.push(Action::Win(p, w));
                    self.transparent_state.player_stacks[p] += w;
                }
                if self.transparent_state.pot.is_empty() {
                    break;
                }
            }
        }

        state.extend(self.transparent_state.actions.drain(..));
        state.push(Action::EndRound);
        self.transparent_state.prepare_next_round();

        state.into_iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::{Action, PlayerAction};
    use crate::mock::MockPlayer;

    #[test]
    fn test_play_one_round() {
        let players = vec![
            MockPlayer::new(vec![PlayerAction::Check]), // dealer
            MockPlayer::new(vec![PlayerAction::Check]), // small
            MockPlayer::new(vec![PlayerAction::Check]), // big
            MockPlayer::new(vec![PlayerAction::Check]),
        ];
        let mut table = Table::new(players.into_iter(), 100, 1, BlindPolicy::NeverIncrease);

        // TODO
    }
}
