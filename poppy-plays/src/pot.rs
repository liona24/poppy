//! This module exposes a structure `Pot` which takes care of shared chips.
use crate::ChipCount;

/// A pot which takes care of shared chips.
///
/// It provides means of accessing the total chip count contained.
///
/// Internally it is used to correctly handle split pots and distributing chips
/// to betting players.
#[derive(Debug, Clone)]
pub struct Pot {
    player_bets: Vec<ChipCount>,
    bet_size: ChipCount,
    bet_size_round: ChipCount,
    last_raise_amount: ChipCount,
}

impl Pot {
    /// Create an empty pot for the given number of players
    pub(crate) fn new(num_players: usize) -> Self {
        Self {
            player_bets: vec![0; num_players],
            bet_size: 0,
            bet_size_round: 0,
            last_raise_amount: 0,
        }
    }

    /// Reset the pot. All remaining chips will be silently discarded.
    pub(crate) fn reset(&mut self) {
        for bet in self.player_bets.iter_mut() {
            *bet = 0;
        }
        self.bet_size = 0;
        self.bet_size_round = 0;
        self.last_raise_amount = 0;
    }

    /// Place the given amount of chips from player located at `player_position` into the pot.
    ///
    /// This function returns true if the given bet size is considered a raise.\
    /// Note that in this case a simple bet is also considered a "raise".
    pub(crate) fn place_chips(&mut self, player_position: usize, amount: ChipCount) -> bool {
        self.player_bets[player_position] += amount;
        // the diff may be negative if we are facing an all-in situation
        let diff = self.player_bets[player_position] as i64 - self.total_bet_size() as i64;
        if diff > 0 {
            // Raise
            self.last_raise_amount = diff as ChipCount;
            self.bet_size_round += diff as ChipCount;
            true
        } else {
            // diff == 0
            false
        }
    }

    pub(crate) fn end_bet_round(&mut self) {
        self.bet_size += self.bet_size_round;
        self.bet_size_round = 0;
        self.last_raise_amount = 0;
    }

    /// Distributes the pot between the players located at `player_positions`.
    /// Return the number of chips won for each position given.
    ///
    /// If the pot cannot be evenly distributed the player which is yielded first
    /// receives the remaining chips.
    ///
    /// Usually you will want to distribute to only one player (i.e. because he won).
    /// However if there is a split multiple players are supported.
    ///
    /// If the winning player does not cover the full pot (f.e. player A with a
    /// small pot pushed all-in, followed by player B pushing all-in with a larger
    /// stack and player A won the hand), the rest of the pot has to be distributed
    /// to the remaining players.\
    /// Usually this can be achieved by chaining multiple calls to this method.
    pub(crate) fn distribute(&mut self, player_positions: &[usize]) -> Vec<ChipCount> {
        if self.bet_size_round != 0 {
            self.end_bet_round()
        }

        let player_which_receives_rest = player_positions.first().copied();
        let mut player_positions = player_positions.to_owned();
        player_positions.sort_by_key(|&pos| self.player_bets[pos]);

        let mut n_receivers = player_positions.len() as u32;
        let mut pot_size = 0;

        let mut stacks = vec![0; self.player_bets.len()];

        for pos in player_positions {
            let shared_size = self.player_bets[pos];
            for bet_size in self.player_bets.iter_mut() {
                let actual_size = std::cmp::min(*bet_size, shared_size);
                *bet_size -= actual_size;
                pot_size += actual_size;
            }

            let rest = pot_size % n_receivers;
            stacks[pos] += pot_size / n_receivers;
            // since we are already iterating over the collection the first element should always be present
            stacks[player_which_receives_rest.unwrap()] += rest;
            pot_size -= rest + pot_size / n_receivers;
            n_receivers -= 1;
        }

        stacks
    }

    pub(crate) fn last_raise_amount(&self) -> ChipCount {
        self.last_raise_amount
    }

    /// Calculate the total number of chips contained in the pot
    pub fn total_size(&self) -> ChipCount {
        self.player_bets.iter().sum()
    }

    /// Returns the total number of chips each player has/had to put into the pot to stay in it.
    pub fn total_bet_size(&self) -> ChipCount {
        self.bet_size + self.bet_size_round
    }

    /// Calculate the effective pot size for the player located at the given position
    /// given he bets the specified amount.
    ///
    /// This accounts for all-in situations as `total_size()` does not.
    pub fn effective_total_size(&self, player_position: usize, bet_size: ChipCount) -> ChipCount {
        let eff_bet_size = self
            .player_bets
            .get(player_position)
            .expect("Player position was invalid")
            + bet_size;
        self.player_bets
            .iter()
            .map(|x| std::cmp::min(eff_bet_size, *x))
            .sum::<u32>()
            + bet_size
    }

    /// Calculates the number of chips the player at the given position is required to bet to stay in the pot.
    pub fn required_bet_size(&self, player_position: usize) -> ChipCount {
        self.total_bet_size()
            - self
                .player_bets
                .get(player_position)
                .expect("Player position was invalid")
    }

    /// Get the bet size for the current round, i. e. the highest bet any player has put into the pot in the current round.
    /// One round is corresponding to the actions taken between any two deal actions.
    /// This value resets to zero after *any* cards have been dealt.
    pub fn bet_size_round(&self) -> ChipCount {
        self.bet_size_round
    }

    /// Check whether the total pot size is zero
    pub fn is_empty(&self) -> bool {
        self.total_size() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_place_chips() {
        let mut pot = Pot::new(3);
        assert!(pot.place_chips(0, 10));
        assert!(!pot.place_chips(1, 10));
    }

    #[test]
    fn test_total_size() {
        let mut pot = Pot::new(3);
        pot.place_chips(0, 10);
        pot.place_chips(1, 10);
        pot.place_chips(1, 5);

        assert_eq!(pot.total_size(), 25);
    }

    #[test]
    fn test_required_bet_size() {
        let mut pot = Pot::new(2);
        pot.place_chips(1, 5);
        pot.place_chips(0, 10);

        assert_eq!(pot.required_bet_size(0), 0);
        assert_eq!(pot.required_bet_size(1), 5);
    }

    #[test]
    fn test_bet_size_round() {
        let mut pot = Pot::new(3);
        pot.place_chips(0, 10);
        pot.place_chips(1, 5);

        assert_eq!(pot.bet_size_round(), 10);
        pot.place_chips(1, 10);
        assert_eq!(pot.bet_size_round(), 15);
    }

    #[test]
    fn test_effective_size() {
        let mut pot = Pot::new(3);
        pot.place_chips(0, 10);
        pot.place_chips(1, 5);
        pot.place_chips(2, 10);

        assert_eq!(pot.effective_total_size(1, 0), 15);
        assert_eq!(pot.effective_total_size(0, 0), 25);
        assert_eq!(pot.effective_total_size(1, 5), 30);
    }

    #[test]
    fn test_reset() {
        let mut pot = Pot::new(3);
        pot.place_chips(0, 10);
        pot.place_chips(1, 5);

        pot.reset();

        assert_eq!(pot.bet_size, 0);
        assert!(pot.player_bets.iter().all(|&x| x == 0));
    }

    #[test]
    fn test_distribute_one_player_split_pot() {
        let mut pot = Pot::new(3);
        pot.place_chips(0, 10);
        pot.place_chips(1, 5);
        pot.place_chips(2, 10);

        let stacks = pot.distribute(&[1]);
        assert_eq!(stacks, [0, 15, 0]);
    }

    #[test]
    fn test_distribute_one_player_full_pot() {
        let mut pot = Pot::new(3);
        pot.place_chips(0, 15);
        pot.place_chips(1, 15);
        pot.place_chips(2, 11);

        let stacks = pot.distribute(&[1]);
        assert_eq!(stacks, [0, 41, 0]);
    }

    #[test]
    fn test_distribute_multiple_players_full_pot() {
        let mut pot = Pot::new(3);
        pot.place_chips(0, 15);
        pot.place_chips(1, 15);
        pot.place_chips(2, 11);

        let stacks = pot.distribute(&[1, 0]);
        assert_eq!(stacks, [20, 21, 0]);
    }

    #[test]
    fn test_distribute_multiple_split_pot() {
        let mut pot = Pot::new(3);
        pot.place_chips(0, 15);
        pot.place_chips(1, 15);
        pot.place_chips(2, 11);

        let stacks = pot.distribute(&[0, 2]);
        assert_eq!(stacks, [25, 0, 16]);
    }

    #[test]
    fn test_distribute_chain() {
        let mut pot = Pot::new(3);
        pot.place_chips(0, 15);
        pot.place_chips(1, 15);
        pot.place_chips(2, 11);

        let stacks: Vec<_> = pot
            .distribute(&[2])
            .into_iter()
            .zip(pot.distribute(&[1]).into_iter())
            .map(|(x, y)| x + y)
            .collect();
        assert_eq!(stacks, [0, 8, 33]);
    }

    #[test]
    fn test_last_raise_amount() {
        let mut pot = Pot::new(3);
        assert_eq!(pot.last_raise_amount(), 0);
        pot.place_chips(0, 5);
        assert_eq!(pot.last_raise_amount(), 5);
        pot.place_chips(1, 11);
        assert_eq!(pot.last_raise_amount(), 6);
        pot.place_chips(2, 11);
        assert_eq!(pot.last_raise_amount(), 6);
    }
}
