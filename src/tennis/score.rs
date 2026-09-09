//! Tennis scoring + serve rotation — v0.14 `TennisScore` semantics
//! (`recovered_scoring_v014.hpp` / `evidence/parity-v014-serve/scoring-v014.md`)
//! with the v0.12 model's set length (`games_to_win = 2`).
//!
//! Game: first side to `POINTS_TO_WIN_GAME` (4) points with a 2-point lead —
//! the raw counts carry 15/30/40 implicitly. Serve rotates every game:
//! `server = (first_server + games_home + games_away) % 2`.

use super::court::Side;

pub const POINTS_TO_WIN_GAME: u32 = 4;
pub const GAMES_TO_WIN_SET: u32 = 2;

#[derive(Debug, Clone, PartialEq)]
pub struct Score {
    /// Raw point counts this game (0..=4+, display 15/30/40 is the game's UI).
    pub points: [u32; 2],
    pub games: [u32; 2],
    pub sets: [u32; 2],
    /// 0 = home, 1 = away. Drawn or decided from the seed.
    pub first_server: usize,
    /// Serve number within the current point (1st/2nd).
    pub serve_number: u32,
    /// Lifetime counters (sensors expose these).
    pub aces: [u32; 2],
    pub faults: [u32; 2],
    pub double_faults: [u32; 2],
    pub fouls: [u32; 2],
    pub outs: [u32; 2],
    pub charged_shots: [u32; 2],
}

impl Score {
    pub fn new(first_server: usize) -> Self {
        Self {
            points: [0, 0],
            games: [0, 0],
            sets: [0, 0],
            first_server,
            serve_number: 1,
            aces: [0; 2],
            faults: [0; 2],
            double_faults: [0; 2],
            fouls: [0; 2],
            outs: [0; 2],
            charged_shots: [0; 2],
        }
    }

    /// Server of the current point: rotates every completed game.
    pub fn server(&self) -> Side {
        let games_total = (self.games[0] + self.games[1]) as usize;
        if (self.first_server + games_total) % 2 == 0 {
            Side::Home
        } else {
            Side::Away
        }
    }

    /// Receiver of the current point.
    pub fn receiver(&self) -> Side {
        self.server().other()
    }

    /// Ad court when an odd number of points has been played in this game
    /// (`points_played_in_game & 1`, matching the recovered stance getters).
    pub fn ad_court(&self) -> bool {
        (self.points[0] + self.points[1]) % 2 == 1
    }

    /// Award a point to `side`. Returns true when the game (and possibly the
    /// set/match) completed and serve should rotate.
    pub fn award_point(&mut self, side: Side) -> bool {
        let i = side as usize;
        self.points[i] += 1;
        self.serve_number = 1;
        if self.points[i] >= POINTS_TO_WIN_GAME
            && self.points[i] as i32 - self.points[1 - i] as i32 >= 2
        {
            self.games[i] += 1;
            let game_won = true;
            if self.games[i] >= GAMES_TO_WIN_SET {
                self.sets[i] += 1;
                self.games = [0, 0];
            }
            self.points = [0, 0];
            return game_won;
        }
        false
    }

    /// Deuce: both sides at 3+ with equal points.
    pub fn is_deuce(&self) -> bool {
        self.points[0] >= 3 && self.points[0] == self.points[1]
    }

    /// Advantage: 4+ points with a one-point lead.
    pub fn has_advantage(&self, side: Side) -> bool {
        let i = side as usize;
        self.points[i] >= 4 && self.points[i] == self.points[1 - i] + 1
    }

    /// Game point for `side`: one more point wins the game.
    pub fn is_game_point(&self, side: Side) -> bool {
        let i = side as usize;
        if self.points[i] < 3 {
            return false;
        }
        self.points[i] as i32 - self.points[1 - i] as i32 >= 1
    }

    /// Break point: the *receiver* is at game point.
    pub fn is_break_point(&self) -> bool {
        self.is_game_point(self.receiver())
    }

    /// Match point: one more game wins the (modeled) set.
    pub fn is_match_point(&self, side: Side) -> bool {
        let i = side as usize;
        self.games[i] + 1 >= GAMES_TO_WIN_SET && self.is_game_point(side)
    }

    pub fn is_winning(&self, side: Side) -> bool {
        let i = side as usize;
        self.points[i] > self.points[1 - i]
    }

    /// First fault of the current serve: re-serve.
    pub fn fault(&mut self, server: Side) {
        self.faults[server as usize] += 1;
        self.serve_number = 2;
    }

    /// Double fault: receiver takes the point.
    pub fn double_fault(&mut self, server: Side) -> bool {
        self.faults[server as usize] += 1;
        self.double_faults[server as usize] += 1;
        self.serve_number = 1;
        self.award_point(server.other())
    }

    pub fn record_ace(&mut self, server: Side) {
        self.aces[server as usize] += 1;
    }

    pub fn record_out(&mut self, side: Side) {
        self.outs[side as usize] += 1;
    }

    pub fn record_foul(&mut self, side: Side) {
        self.fouls[side as usize] += 1;
    }

    pub fn record_charged(&mut self, side: Side) {
        self.charged_shots[side as usize] += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn game_requires_four_points_and_two_lead() {
        let mut s = Score::new(0);
        assert_eq!(s.server(), Side::Home);
        for _ in 0..3 {
            s.award_point(Side::Home);
        }
        // 40-0 is not a game yet.
        assert_eq!(s.points, [3, 0]);
        assert!(s.award_point(Side::Home));
        assert_eq!(s.points, [0, 0]);
        assert_eq!(s.games, [1, 0]);
    }

    #[test]
    fn deuce_and_advantage() {
        let mut s = Score::new(0);
        for _ in 0..3 {
            s.award_point(Side::Home);
            s.award_point(Side::Away);
        }
        assert!(s.is_deuce());
        assert!(!s.is_game_point(Side::Home));
        s.award_point(Side::Home);
        assert!(s.has_advantage(Side::Home));
        assert!(s.is_game_point(Side::Home));
        assert!(!s.award_point(Side::Away)); // back to deuce, game continues
        assert!(s.is_deuce());
        s.award_point(Side::Home);
        assert!(s.award_point(Side::Home));
        assert_eq!(s.games, [1, 0]);
    }

    #[test]
    fn serve_rotates_every_game() {
        let mut s = Score::new(0);
        assert_eq!(s.server(), Side::Home);
        s.award_point(Side::Home);
        assert_eq!(s.server(), Side::Home);
        s.award_point(Side::Home);
        s.award_point(Side::Home);
        assert!(s.award_point(Side::Home)); // 4-0: game
        assert_eq!(s.server(), Side::Away);
    }

    #[test]
    fn set_ends_after_two_games_and_break_point_logic() {
        // The model's GAMES_TO_WIN_SET = 2: two games make a set.
        let mut s = Score::new(0);
        while !s.award_point(Side::Home) {}
        while !s.award_point(Side::Home) {}
        assert_eq!(s.sets, [1, 0]);
        assert_eq!(s.games, [0, 0]);

        let mut s = Score::new(0);
        while !s.award_point(Side::Away) {}
        // Away serves next, so Home receiver at game point = break point.
        assert_eq!(s.server(), Side::Away);
        s.award_point(Side::Home);
        s.award_point(Side::Home);
        s.award_point(Side::Home);
        assert!(s.is_break_point());
    }

    #[test]
    fn fault_then_double_fault() {
        let mut s = Score::new(0);
        s.fault(Side::Home);
        assert_eq!(s.serve_number, 2);
        assert_eq!(s.faults, [1, 0]);
        assert!(!s.double_fault(Side::Home)); // points 0-1: game continues
        assert_eq!(s.double_faults, [1, 0]);
        assert_eq!(s.points, [0, 1]);
        assert_eq!(s.serve_number, 1);
    }
}
