use crate::actions::PlayerAction;
use crate::{ChipCount, Player, TransparentState};
use rs_poker::core::Card;
use std::collections::VecDeque;

#[derive(Debug)]
pub(crate) struct MockPlayer {
    pub(crate) cards: Option<[Card; 2]>,
    pub(crate) position: Option<usize>,
    pub(crate) next_actions: VecDeque<PlayerAction>,
    pub(crate) busted: bool,
    pub(crate) last_possible_actions: Vec<PlayerAction>,
}

impl MockPlayer {
    pub(crate) fn new(next_actions: Vec<PlayerAction>) -> Self {
        let next_actions = VecDeque::from(next_actions);
        Self {
            position: None,
            cards: None,
            busted: false,
            next_actions,
            last_possible_actions: Vec::new(),
        }
    }
}

impl Player for MockPlayer {
    fn init(&mut self, position: usize, _initial_stack: ChipCount) {
        self.position = Some(position);
    }

    fn receive_cards(&mut self, c1: Card, c2: Card) {
        self.cards = Some([c1, c2]);
    }

    fn act(
        &mut self,
        _state: &TransparentState,
        possible_actions: &[PlayerAction],
    ) -> PlayerAction {
        self.last_possible_actions = possible_actions.to_vec();
        let action_taken = self
            .next_actions
            .pop_front()
            .expect("Should have valid next action");
        assert!(possible_actions
            .iter()
            .any(|a| std::mem::discriminant(a) == std::mem::discriminant(&action_taken)));

        action_taken
    }

    fn bust(&mut self) {
        self.busted = true;
    }
}
