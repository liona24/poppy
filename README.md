# poppy

[![Build Status](https://dev.azure.com/ackermlion/poppy/_apis/build/status/liona24.poppy?branchName=master)](https://dev.azure.com/ackermlion/poppy/_build/latest?definitionId=3&branchName=master)
[![codecov](https://codecov.io/gh/liona24/poppy/branch/master/graph/badge.svg)](https://codecov.io/gh/liona24/poppy)

## Introduction

This is a simple library which can be used to implement no-limit Texas Hold'em poker gameplay in rust.
Originally built on top of [rs_poker](https://crates.io/crates/rs-poker) the projects diverged quite a lot eventually resulting in a stand-alone library.
There are no dependencies required, though adding serialization support is planned (as a feature).

The gameplay is built as an iterator.
The main design goals were a) being able to present only the valid actions at each point in time to each player b) eventually being able to support simple logging functionality and c) being able to replay rounds starting at any point in time with different players etc.

You can take a look at the simple example below:
```rust
use poppy::prelude::*;

#[derive(Debug, Clone)]
struct PlayerType;

impl Player for PlayerType {
    fn init(&mut self, _position: usize, _initial_stack: ChipCount) {
        // intitialize some internal state if needed.
    }

    fn act(
        &mut self,
        _state: &TransparentState,
        possible_actions: &[PlayerAction],
    ) -> PlayerAction {
        // main interaction callback
        // use `state` to retrieve information about game state and choose any of the actions possible
        // we will just use a "random" one:

        assert!(!possible_actions.is_empty());
        possible_actions[0]
    }

    fn bust(&mut self) {
        // callback to de-init this player, called when this player has no chips left
        println!(":(");
    }
}

fn main() {
    let players = vec![PlayerType {}; 12];
    let stack_size = 100;
    let blind_size = 1;
    let mut table = Table::new(
        players.into_iter(),
        stack_size,
        blind_size,
        BlindPolicy::NeverIncrease,
    );

    // You can shuffle decks as you want to.
    // We will use a default un-sorted deck here.
    let deck = deck::CardCollection::default();
    let round_iter = table.play_one_round(deck);

    for action_taken in round_iter {
        println!("{:?}", action_taken);
    }
}
```
