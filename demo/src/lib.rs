mod utils;
extern crate js_sys;

use std::cell::{RefCell, Ref, RefMut};

use wasm_bindgen::prelude::*;
use poppy::prelude::*;

// When the `wee_alloc` feature is enabled, use `wee_alloc` as the global
// allocator.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[wasm_bindgen]
extern {
    fn alert(s: &str);

    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

fn rand_int(max: usize) -> usize {
    (js_sys::Math::random() * (max as f64)) as usize
}

// TODO: This model should be constructed somewhere in the js world and get polled for view updates
// also, upon player interaction the `chosen_action` field has to be set accordingly

#[derive(Debug)]
struct Model {
    player_stacks: Vec<ChipCount>,
    chosen_action: Option<PlayerAction>,
    cards: Option<[poppy::deck::Card; 2]>,
    board: Option<poppy::Board>,
    possible_actions: Vec<PlayerAction>,
    player_busted: bool
}

impl Model {
    fn new() -> Self {
        Self {
            player_stacks: Vec::new(),
            chosen_action: None,
            cards: None,
            board: None,
            possible_actions: Vec::new(),
            player_busted: false,
        }
    }

    fn poll<'a>() -> Ref<'a, Self> {
        loop {
            std::thread::sleep(std::time::Duration::from_millis(1));
            {
                if let Ok(x) = g_model.try_borrow() {
                    break x;
                }
            }
        }
    }

    fn poll_mut<'a>() -> RefMut<'a, Self> {
        loop {
            std::thread::sleep(std::time::Duration::from_millis(1));
            {
                if let Ok(x) = g_model.try_borrow_mut() {
                    break x;
                }
            }
        }
    }
}

static g_model: RefCell<Model> = RefCell::new(Model::new());

#[derive(Debug, Clone, Copy)]
enum InteractionType {
    Ai,
    Human,
}

#[derive(Debug, Clone)]
struct APlayer {
    position: usize,
    busted: bool,
    interaction_type: InteractionType,
}

impl APlayer {
    fn new(interaction_type: InteractionType) -> Self {
        Self {
            position: 0,
            busted: false,
            interaction_type
        }
    }
}

impl Player for APlayer {

    fn init(&mut self, position: usize, initial_stack: ChipCount) {
        let model = Model::poll_mut();
        model.player_stacks[position] = initial_stack;

        self.position = position;
        self.busted = false;
    }

    fn act(&mut self, state: &TransparentState, possible_actions: &[PlayerAction]) -> PlayerAction {
        match self.interaction_type {
            InteractionType::Ai => {
                // simulate some thinking
                let sleep_dur = (rand_int(750) + 250) as u64;
                std::thread::sleep(std::time::Duration::from_millis(sleep_dur));

                possible_actions[rand_int(possible_actions.len())]
            },
            InteractionType::Human => {
                // update model state
                {
                    let model = Model::poll_mut();
                    if model.cards.is_none() {
                        model.cards = Some(state.query_cards(self.position));
                    }
                    model.possible_actions = possible_actions.to_vec();
                    model.chosen_action = None;
                }

                // poll for interaction
                loop {
                    {
                        let model = Model::poll();
                        if let Some(action) = model.chosen_action {
                            break action;
                        }
                    }
                }
            }
        }
    }

    fn bust(&mut self) {
        self.busted = true;
        if let InteractionType::Human = self.interaction_type {
            let model = Model::poll_mut();
            model.player_busted = true;
        }
    }
}

#[wasm_bindgen]
pub struct Game {
    table : Table<APlayer>,
}

#[wasm_bindgen]
impl Game {
    pub fn new(num_players: usize) -> Self {
        let mut players = vec![APlayer::new(InteractionType::Ai); num_players - 1];
        players.push(APlayer::new(InteractionType::Human));
        let table = Table::new(players.into_iter(), 100, 1, BlindPolicy::NeverIncrease);

        Self {
            table,
        }
    }

    pub fn play_one_round(&mut self) {
        let mut deck = deck::CardCollection::default();
        deck.shuffle(rand_int);

        let mut round = self.table.play_one_round(deck);

        loop {
            let action = round.next();

            if action.is_none() {
                break;
            }

            log(&format!("{:?}", action));

            {
                let state = round.inspect_state();
                /*
                let mut model = poll_mut!(self.model);
                model.board = Some(state.board.clone());
                */

                // TODO: update player betting behaviour and pot sizes
            }
        }
    }
}

#[wasm_bindgen]
pub fn greet() {
    alert("Hello, demo!");
}
