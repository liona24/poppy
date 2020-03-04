use crate::actions::Action;
use crate::deck::{DeckGenerator, DefaultDeckGenerator};
use crate::player::Player;
use crate::state::TransparentState;
use crate::ChipCount;
use itertools::{multipeek, Itertools};
use rs_poker::core::{Card, FlatDeck};

pub enum BlindPolicy {
    NeverIncrease,
}

pub struct Table<P, G: DeckGenerator = DefaultDeckGenerator> {
    players: Vec<P>,
    blind_policy: BlindPolicy,
    transparent_state: TransparentState,
    last_cards: Vec<[Card; 2]>,
    deck_generator: G,
}

impl<P: Player> Table<P, DefaultDeckGenerator> {

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
    ///
    /// The method provides a shortcut for `Table::<P, DefaultDeckGenerator>::new`
    pub fn create_default(
        players: impl Iterator<Item = P>,
        stack_size: ChipCount,
        blind_size: ChipCount,
        blind_policy: BlindPolicy,
    ) -> Self {
        Table::<P, DefaultDeckGenerator>::new(players, stack_size, blind_size, blind_policy)
    }
}

// progress_hook: Option<Box<dyn Fn(&TransparentState)>>,

impl<P: Player, G: DeckGenerator> Table<P, G> {
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
            value: rs_poker::core::Value::Ace,
            suit: rs_poker::core::Suit::Club,
        };
        let last_cards = vec![[default_card, default_card]; players.len()];

        Self {
            players,
            blind_policy,
            transparent_state: TransparentState::new(blind_size, 0, stack_sizes),
            last_cards,
            deck_generator: G::default(),
        }
    }

    pub fn play_one_round(&mut self) -> impl Iterator<Item = Action> {
        let mut state = vec![Action::StartRound {
            id: 0,
            small_blind: self.transparent_state.blind_size,
            big_blind: self.transparent_state.blind_size * 2,
        }];
        let mut deck: FlatDeck = self.deck_generator.shuffled_deck();

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
