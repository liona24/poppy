use crate::actions::Action;
use crate::deck::{Card, Deck};
use crate::player::Player;
use crate::state::TransparentState;
use crate::ChipCount;
use itertools::{multipeek, Itertools};

/// Exposes variants to handle blind policies, i. e. control when and how much the blind size should be increased.
pub enum BlindPolicy {
    /// Incite that the blinds should never increase.
    NeverIncrease,
}

/// The main entrypoint for playing poker games.
/// A table represents a collection of players and handles the actual game logic.
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
    /// The number of players has to be in the interval [2, 19]
    pub fn new(
        players: impl Iterator<Item = P>,
        stack_size: ChipCount,
        blind_size: ChipCount,
        blind_policy: BlindPolicy,
    ) -> Self {
        let players: Vec<P> = players.collect();
        assert!(players.len() < 20);
        assert!(players.len() > 1);

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

    /// Play one round of poker at this table using the given deck.
    ///
    /// Returns all the actions taken during this round.
    ///
    /// It is expected that the given deck is valid, i. e. contains all cards, is properly shuffled, etc.
    pub fn play_one_round(&mut self, mut deck: impl Deck) -> impl Iterator<Item = Action> {
        let mut state = vec![Action::StartRound {
            id: 0,
            small_blind: self.transparent_state.blind_size,
            big_blind: self.transparent_state.blind_size * 2,
        }];

        for &i in self.transparent_state.player_positions.iter() {
            let c1 = deck.deal().expect("Deck should contain enough cards");
            let c2 = deck.deal().expect("Deck should contain enough cards");
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
                    deck.deal().expect("Deck should contain enough cards"),
                    deck.deal().expect("Deck should contain enough cards"),
                    deck.deal().expect("Deck should contain enough cards"),
                ];

                self.transparent_state.deal_flop(flop);
                self.transparent_state
                    .apply_post_flop_action(&mut self.players)
            }
            || {
                let turn = deck.deal().expect("Deck should contain enough cards");
                self.transparent_state.deal_turn(turn);
                self.transparent_state
                    .apply_post_flop_action(&mut self.players)
            }
            || {
                let river = deck.deal().expect("Deck should contain enough cards");
                self.transparent_state.deal_river(river);
                self.transparent_state
                    .apply_post_flop_action(&mut self.players)
            };

        state.extend(self.transparent_state.actions.drain(..));

        if self.transparent_state.num_players() == 1 {
            // the player left gets the pot
            let pos = *self.transparent_state.player_positions.first().unwrap();
            let win = self
                .transparent_state
                .pot
                .distribute(&self.transparent_state.player_positions)[pos];
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
                for p in positions.into_iter() {
                    let amount = win[p];
                    state.push(Action::Win(p, amount));
                    self.transparent_state.player_stacks[p] += amount;
                }
                if self.transparent_state.pot.is_empty() {
                    break;
                }
            }
        }

        state.push(Action::EndRound);
        self.transparent_state.prepare_next_round();

        state.into_iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::{Action, PlayerAction};
    use crate::deck::card::{Suit, Value};
    use crate::deck::CardCollection;
    use crate::mock::MockPlayer;
    use std::convert::TryInto;

    #[test]
    #[should_panic]
    fn test_init_with_insufficient_number_of_players() {
        let players = vec![
            MockPlayer::new(vec![PlayerAction::Check]), // dealer
        ];
        let _ = Table::new(players.into_iter(), 100, 1, BlindPolicy::NeverIncrease);
    }

    #[test]
    #[should_panic]
    fn test_init_with_too_many_players() {
        let players = vec![MockPlayer::new(vec![PlayerAction::Check]); 20];
        let _ = Table::new(players.into_iter(), 100, 1, BlindPolicy::NeverIncrease);
    }

    #[test]
    fn test_play_all_but_one_fold_pre_flop() {
        let players = vec![
            MockPlayer::new(vec![PlayerAction::Raise(10)]), // dealer
            MockPlayer::new(vec![PlayerAction::Blind(1), PlayerAction::Fold]), // small
            MockPlayer::new(vec![PlayerAction::Blind(2), PlayerAction::Fold]), // big
            MockPlayer::new(vec![PlayerAction::Fold]),
        ];
        let mut table = Table::new(players.into_iter(), 100, 1, BlindPolicy::NeverIncrease);
        let actions: Vec<Action> = table.play_one_round(CardCollection::default()).collect();

        // skip the meta information and card deals at the start
        assert_eq!(
            actions[5..],
            [
                Action::Blind(1, 1),
                Action::Blind(2, 2),
                Action::Fold(3),
                Action::Raise(0, 10),
                Action::Fold(1),
                Action::Fold(2),
                Action::Win(0, 13),
                Action::EndRound
            ]
        );
    }

    #[test]
    fn test_play_all_but_one_fold_before_turn() {
        let players = vec![
            MockPlayer::new(vec![PlayerAction::Raise(10), PlayerAction::Fold]), // dealer
            MockPlayer::new(vec![PlayerAction::Blind(1), PlayerAction::Fold]),  // small
            MockPlayer::new(vec![
                PlayerAction::Blind(2),
                PlayerAction::Call(8),
                PlayerAction::Bet(2),
            ]), // big
            MockPlayer::new(vec![PlayerAction::Fold]),
        ];
        let mut table = Table::new(players.into_iter(), 100, 1, BlindPolicy::NeverIncrease);
        let actions: Vec<Action> = table.play_one_round(CardCollection::default()).collect();

        // skip the meta information and card deals at the start
        assert_eq!(
            actions[5..11],
            [
                Action::Blind(1, 1),
                Action::Blind(2, 2),
                Action::Fold(3),
                Action::Raise(0, 10),
                Action::Fold(1),
                Action::Call(2, 8),
            ]
        );

        // we do not explicitly test which cards are dealt. Just make sure the correct action was emitted.
        let dummy_cards: CardCollection = "AdAsAh".try_into().unwrap();
        let flop = Action::DealFlop(dummy_cards.to_array::<[Card; 3]>());
        assert!(std::mem::discriminant(&actions[11]) == std::mem::discriminant(&flop));

        assert_eq!(
            actions[12..],
            [
                Action::Bet(2, 2),
                Action::Fold(0),
                Action::Win(2, 1 + 2 + 10 + 8 + 2),
                Action::EndRound
            ]
        );
    }

    #[test]
    fn test_play_all_but_one_fold_before_river() {
        let players = vec![
            MockPlayer::new(vec![PlayerAction::Raise(10), PlayerAction::Fold]), // dealer
            MockPlayer::new(vec![
                PlayerAction::Blind(1),
                PlayerAction::Call(9),
                PlayerAction::Check,
                PlayerAction::Call(2),
                PlayerAction::Bet(2),
            ]), // small
            MockPlayer::new(vec![
                PlayerAction::Blind(2),
                PlayerAction::Call(8),
                PlayerAction::Bet(2),
                PlayerAction::Fold,
            ]), // big
            MockPlayer::new(vec![PlayerAction::Fold]),
        ];
        let mut table = Table::new(players.into_iter(), 100, 1, BlindPolicy::NeverIncrease);
        let actions: Vec<Action> = table.play_one_round(CardCollection::default()).collect();

        // skip the meta information and card deals at the start
        assert_eq!(
            actions[5..11],
            [
                Action::Blind(1, 1),
                Action::Blind(2, 2),
                Action::Fold(3),
                Action::Raise(0, 10),
                Action::Call(1, 9),
                Action::Call(2, 8),
            ]
        );

        assert_eq!(
            actions[12..16],
            [
                Action::Check(1),
                Action::Bet(2, 2),
                Action::Fold(0),
                Action::Call(1, 2),
            ]
        );

        let turn = Action::DealTurn(Card::default());
        assert!(std::mem::discriminant(&actions[16]) == std::mem::discriminant(&turn));

        assert_eq!(
            actions[17..],
            [
                Action::Bet(1, 2),
                Action::Fold(2),
                Action::Win(1, 1 + 2 + 10 + 9 + 8 + 2 + 2 + 2),
                Action::EndRound
            ]
        );
    }

    #[test]
    fn test_play_all_but_one_fold_before_showdown() {
        let players = vec![
            MockPlayer::new(vec![PlayerAction::Raise(10), PlayerAction::Fold]), // dealer
            MockPlayer::new(vec![
                PlayerAction::Blind(1),
                PlayerAction::Call(9),
                PlayerAction::Check, // after flop
                PlayerAction::Call(2),
                PlayerAction::Bet(2), // after turn
                PlayerAction::Bet(2), // after river
                PlayerAction::Fold,
            ]), // small
            MockPlayer::new(vec![
                PlayerAction::Blind(2),
                PlayerAction::Call(8),
                PlayerAction::Bet(2),    // after flop
                PlayerAction::Call(2),   // after turn
                PlayerAction::Raise(10), // after river
            ]), // big
            MockPlayer::new(vec![PlayerAction::Fold]),
        ];
        let mut table = Table::new(players.into_iter(), 100, 1, BlindPolicy::NeverIncrease);
        let actions: Vec<Action> = table.play_one_round(CardCollection::default()).collect();

        assert_eq!(actions[17..19], [Action::Bet(1, 2), Action::Call(2, 2),]);

        let river = Action::DealRiver(Card::default());
        assert!(std::mem::discriminant(&actions[19]) == std::mem::discriminant(&river));

        assert_eq!(
            actions[20..],
            [
                Action::Bet(1, 2),
                Action::Raise(2, 10),
                Action::Fold(1),
                Action::Win(2, 1 + 2 + 10 + 9 + 8 + 2 + 2 + 2 + 2 + 2 + 10),
                Action::EndRound
            ]
        );
    }

    #[test]
    fn test_play_showdown_and_one_player_wins() {
        // We abuse a little internal knowledge here
        // Dealing starts at the small blind, 2 cards at a time
        // note that the CardCollection will pop cards in reverse order
        // The following cards will be dealt:
        // 1 -> Ad, 7s; 2 -> Th, Td; 3 -> Ks, Qs; 0 -> 2c, 8c
        // Flop -> 2s, Qd, Tc
        // Turn -> 9d
        // River -> Jc
        // (eventually TT wins with triples)
        let deck: CardCollection = "Jc9dTcQd2s2c8cKsQsTdTh7sAd".try_into().unwrap();
        let players = vec![
            MockPlayer::new(vec![PlayerAction::Raise(10), PlayerAction::Fold]), // dealer
            MockPlayer::new(vec![
                PlayerAction::Blind(1),
                PlayerAction::Call(9),
                PlayerAction::Check, // after flop
                PlayerAction::Call(2),
                PlayerAction::Bet(2), // after turn
                PlayerAction::Bet(2), // after river
                PlayerAction::Call(8),
            ]), // small
            MockPlayer::new(vec![
                PlayerAction::Blind(2),
                PlayerAction::Call(8),
                PlayerAction::Bet(2),    // after flop
                PlayerAction::Call(2),   // after turn
                PlayerAction::Raise(10), // after river
            ]), // big
            MockPlayer::new(vec![PlayerAction::Fold]),
        ];
        let mut table = Table::new(players.into_iter(), 100, 1, BlindPolicy::NeverIncrease);
        let actions: Vec<Action> = table.play_one_round(deck).collect();

        assert_eq!(
            actions[20..],
            [
                Action::Bet(1, 2),
                Action::Raise(2, 10),
                Action::Call(1, 8),
                Action::Win(2, 1 + 2 + 10 + 9 + 8 + 2 + 2 + 2 + 2 + 2 + 10 + 8),
                Action::EndRound
            ]
        );
    }

    #[test]
    fn test_play_showdown_and_two_different_ranks_win() {
        // This test case covers the scenario in which an all-in player wins
        // and the next best hand is winning a tie with a third player
        // We therefor first reduce the stack size of one player to a lower level
        let players = vec![
            MockPlayer::new(vec![
                PlayerAction::Raise(50), // end first round
                PlayerAction::Call(2),
                PlayerAction::Call(78),
                PlayerAction::Call(10), // flop
                PlayerAction::Check,    // turn
                PlayerAction::Check,    // river
            ]),
            MockPlayer::new(vec![
                PlayerAction::Blind(1),
                PlayerAction::Fold, // end first round
                PlayerAction::Fold,
            ]),
            MockPlayer::new(vec![
                PlayerAction::Blind(2),
                PlayerAction::Fold, // end first round
                PlayerAction::Blind(1),
                PlayerAction::Call(1),
                PlayerAction::Call(78),
                PlayerAction::Bet(10), // flop
                PlayerAction::Check,   // turn
                PlayerAction::Check,   // river
            ]),
            MockPlayer::new(vec![
                PlayerAction::Raise(20),
                PlayerAction::Fold, // end first round
                PlayerAction::Blind(2),
                PlayerAction::AllIn(80),
            ]),
        ];
        let mut table = Table::new(players.into_iter(), 100, 1, BlindPolicy::NeverIncrease);
        let _: Vec<_> = table.play_one_round(CardCollection::default()).collect();

        // We abuse a little internal knowledge here
        // Dealing starts at the small blind, 2 cards at a time
        // note that the CardCollection will pop cards in reverse order
        // The following cards will be dealt:
        // 2 -> Ad, 7s; 3 -> Th, Td; 0 -> Ks, Qs; 1 -> 2c, 8c
        // Flop -> 2s, Qd, Tc
        // Turn -> 9d
        // River -> Jc
        // (eventually TT wins with triples)
        let deck: CardCollection = "Jc9dTcQd2s2c8cKsQsTdTh7sAd".try_into().unwrap();
        let actions: Vec<Action> = table.play_one_round(deck).collect();

        let flop = [
            Card {
                value: Value::Two,
                suit: Suit::Spade,
            },
            Card {
                value: Value::Queen,
                suit: Suit::Diamond,
            },
            Card {
                value: Value::Ten,
                suit: Suit::Club,
            },
        ];
        let turn = Card {
            value: Value::Nine,
            suit: Suit::Diamond,
        };
        let river = Card {
            value: Value::Jack,
            suit: Suit::Club,
        };

        assert_eq!(
            actions[5..],
            [
                Action::Blind(2, 1),
                Action::Blind(3, 2),
                Action::Call(0, 2),
                Action::Fold(1),
                Action::Call(2, 1),
                Action::AllIn(3, 80),
                Action::Call(0, 78),
                Action::Call(2, 78),
                Action::DealFlop(flop),
                Action::Bet(2, 10),
                Action::Call(0, 10),
                Action::DealTurn(turn),
                Action::Check(2),
                Action::Check(0),
                Action::DealRiver(river),
                Action::Check(2),
                Action::Check(0),
                Action::Win(3, 80 + 80 + 80),
                Action::Win(0, 20),
                Action::EndRound
            ]
        );
    }

    #[test]
    fn test_play_multiple_rounds() {
        let players = vec![
            MockPlayer::new(vec![PlayerAction::Fold, PlayerAction::Fold]), // dealer
            MockPlayer::new(vec![
                PlayerAction::Blind(1),
                PlayerAction::Fold, // end first round
                PlayerAction::Fold,
            ]), // small
            MockPlayer::new(vec![
                PlayerAction::Blind(2), // end first round
                PlayerAction::Blind(1),
                PlayerAction::Fold,
            ]), // big
            MockPlayer::new(vec![PlayerAction::Fold, PlayerAction::Blind(2)]),
        ];
        let mut table = Table::new(players.into_iter(), 100, 1, BlindPolicy::NeverIncrease);

        // implicit check, since mock players will panic if they cannot handle demands
        let _: Vec<_> = table.play_one_round(CardCollection::default()).collect();
        let _: Vec<_> = table.play_one_round(CardCollection::default()).collect();
    }

    #[test]
    fn test_play_round_id_is_unique() {}
}
