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
/// Sets to win the match (best-of-3, literal tennis rules: first to 2 sets).
pub const SETS_TO_WIN_MATCH: u32 = 2;

/// Match format. The simulator ENFORCES `Default` (official rules: 4-point
/// games with a 2-point lead, 2-game short sets, best-of-3) on every world
/// built with `Score::new` / `TennisWorld::new`. A custom config ONLY takes
/// effect when explicitly loaded (`MatchRules::load_json` +
/// `Score::with_rules` / `TennisWorld::new_with_rules`, e.g. 1-set ladders).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MatchRules {
    pub points_to_win_game: u32,
    pub games_to_win_set: u32,
    pub sets_to_win: u32,
}

impl Default for MatchRules {
    fn default() -> Self {
        Self {
            points_to_win_game: POINTS_TO_WIN_GAME,
            games_to_win_set: GAMES_TO_WIN_SET,
            sets_to_win: SETS_TO_WIN_MATCH,
        }
    }
}

impl MatchRules {
    /// Explicit custom config. Every count must be >= 1 (a zero would make
    /// the format unwinnable/unlosable); anything else fails LOUDLY.
    pub fn custom(
        points_to_win_game: u32,
        games_to_win_set: u32,
        sets_to_win: u32,
    ) -> Result<Self, String> {
        for (name, v) in [
            ("points_to_win_game", points_to_win_game),
            ("games_to_win_set", games_to_win_set),
            ("sets_to_win", sets_to_win),
        ] {
            if v < 1 {
                return Err(format!("MatchRules::{name} must be >= 1, got {v}"));
            }
        }
        Ok(Self {
            points_to_win_game,
            games_to_win_set,
            sets_to_win,
        })
    }

    /// Load a custom config from JSON. Missing keys fall back to official
    /// rules; present keys go through the same >= 1 validation:
    /// `{"points_to_win_game":4,"games_to_win_set":2,"sets_to_win":1}`.
    pub fn load_json(path: &std::path::Path) -> Result<Self, String> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| format!("MatchRules: cannot read {}: {e}", path.display()))?;
        // Tolerate a UTF-8 BOM (Windows editors prepend one; it is not JSON).
        let v: serde_json::Value = serde_json::from_str(text.trim_start_matches('\u{FEFF}'))
            .map_err(|e| format!("MatchRules: bad JSON in {}: {e}", path.display()))?;
        let get = |key: &str, fallback: u32| -> Result<u32, String> {
            match &v[key] {
                serde_json::Value::Null => Ok(fallback),
                serde_json::Value::Number(n) => n.as_u64().and_then(|x| u32::try_from(x).ok()).ok_or_else(|| {
                    format!("MatchRules::{key} must be a positive integer")
                }),
                _ => Err(format!("MatchRules::{key} must be a positive integer")),
            }
        };
        Self::custom(
            get("points_to_win_game", POINTS_TO_WIN_GAME)?,
            get("games_to_win_set", GAMES_TO_WIN_SET)?,
            get("sets_to_win", SETS_TO_WIN_MATCH)?,
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Score {
    /// Raw point counts this game (0..=4+, display 15/30/40 is the game's UI).
    pub points: [u32; 2],
    /// Games in the CURRENT set (reset per set — display only).
    pub games: [u32; 2],
    pub sets: [u32; 2],
    /// Serve-rotation clock: total games completed this match, NEVER reset.
    /// Real tennis rotates the serve every game across set boundaries, so
    /// `server()` reads this, not `games` (which restarts at 0 each set —
    /// that pinned every set to open with `first_server`, wrong after any
    /// odd-game set such as 2-1).
    pub total_games: u32,
    /// 0 = home, 1 = away. Drawn or decided from the seed.
    pub first_server: usize,
    /// Enforced match format (official rules unless a custom config was
    /// explicitly loaded — see `MatchRules`).
    pub rules: MatchRules,
    /// Serve-clock refusal: when set, this side serves the current game
    /// instead of the rotation (`award_serve`), and normal rotation resumes
    /// when the game ends. Mirrors the game's `serveDeadlineTick` expiry
    /// (the 5 s `ServeCountdownDisplay` runs out → the opponent is awarded
    /// the serve).
    pub serve_override: Option<usize>,
    /// How many times each side lost the serve to the clock.
    pub serve_forfeits: [u32; 2],
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
    /// Official rules, enforced by default.
    pub fn new(first_server: usize) -> Self {
        Self::with_rules(first_server, MatchRules::default())
    }

    /// Custom-config world (explicit opt-in only).
    pub fn with_rules(first_server: usize, rules: MatchRules) -> Self {
        Self {
            points: [0, 0],
            games: [0, 0],
            sets: [0, 0],
            total_games: 0,
            first_server,
            rules,
            serve_override: None,
            serve_forfeits: [0; 2],
            serve_number: 1,
            aces: [0; 2],
            faults: [0; 2],
            double_faults: [0; 2],
            fouls: [0; 2],
            outs: [0; 2],
            charged_shots: [0; 2],
        }
    }

    /// Server of the current point: the refusal override when set, else the
    /// every-game rotation `(first_server + total_games) % 2`, CONTINUOUS
    /// across sets (real game: set 2 opens with whoever's turn it is, which
    /// flips the opener after an odd-game set like 2-1).
    pub fn server(&self) -> Side {
        if let Some(i) = self.serve_override {
            return if i == 0 { Side::Home } else { Side::Away };
        }
        if (self.first_server + self.total_games as usize) % 2 == 0 {
            Side::Home
        } else {
            Side::Away
        }
    }

    /// Award the serve to `side` for the rest of the current game (serve-clock
    /// refusal). Rotation returns when the game ends (`award_point` clears the
    /// override). Does not move the score — only who serves.
    pub fn award_serve(&mut self, side: Side) {
        self.serve_override = Some(side as usize);
        self.serve_forfeits[side.other() as usize] += 1;
        self.serve_number = 1;
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
    /// set/match) completed and serve should rotate. Thresholds come from
    /// the enforced `rules`, never hard-coded counts. Standard tennis both
    /// levels: games need 4 points with a 2-point lead (deuce/advantage),
    /// sets need `games_to_win_set` with a 2-GAME lead — 2-1 wins nothing,
    /// play continues until someone leads by 2 (advantage-set shape).
    pub fn award_point(&mut self, side: Side) -> bool {
        let i = side as usize;
        self.points[i] += 1;
        self.serve_number = 1;
        if self.points[i] >= self.rules.points_to_win_game
            && self.points[i] as i32 - self.points[1 - i] as i32 >= 2
        {
            self.games[i] += 1;
            self.total_games += 1;
            let game_won = true;
            if self.games[i] >= self.rules.games_to_win_set
                && self.games[i] as i32 - self.games[1 - i] as i32 >= 2
            {
                self.sets[i] += 1;
                self.games = [0, 0];
            }
            self.points = [0, 0];
            // A refusal-awarded serve lasts only the current game.
            self.serve_override = None;
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

    /// Match over: either side reached `sets_to_win` (best-of-3 default =
    /// first to 2; the 3rd set only exists as the 1-1 decider).
    pub fn match_winner(&self) -> Option<Side> {
        if self.sets[0] >= self.rules.sets_to_win {
            Some(Side::Home)
        } else if self.sets[1] >= self.rules.sets_to_win {
            Some(Side::Away)
        } else {
            None
        }
    }

    /// True MATCH point for `side`: one more point wins the game, that game
    /// wins the set, and that set wins the match (all thresholds from the
    /// enforced `rules`, never hard-coded counts).
    pub fn is_match_point(&self, side: Side) -> bool {
        let i = side as usize;
        if !self.is_game_point(side) {
            return false;
        }
        if self.games[i] + 1 < self.rules.games_to_win_set {
            return false;
        }
        self.sets[i] + 1 >= self.rules.sets_to_win
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
    fn serve_rotation_continues_across_sets() {
        // Real game: rotation never restarts at a set boundary. After an
        // even-game set (2-0) the next set opens with the same server;
        // after an odd-game set (2-1) it flips.
        let mut s = Score::new(0);
        assert_eq!(s.server(), Side::Home);
        while !s.award_point(Side::Home) {} // game 1
        assert_eq!(s.server(), Side::Away);
        while !s.award_point(Side::Home) {} // game 2: set 2-0 Home
        assert_eq!(s.sets, [1, 0]);
        assert_eq!(s.games, [0, 0]); // display resets ...
        assert_eq!(s.server(), Side::Home); // ... but rotation continues (2 games: even)
        while !s.award_point(Side::Away) {} // game 3
        assert_eq!(s.server(), Side::Away);
        while !s.award_point(Side::Away) {} // game 4: set 0-2 Away
        assert_eq!(s.sets, [1, 1]);
        while !s.award_point(Side::Away) {} // game 5
        assert_eq!(s.server(), Side::Away); // total=5, odd -> Away serves
        while !s.award_point(Side::Home) {} // game 6
        assert_eq!(s.server(), Side::Home); // total=6, even -> Home serves
        assert_eq!(s.total_games, 6);
    }

    #[test]
    fn default_rules_are_official_best_of_3() {
        // Enforcement pin: what the sim runs unless a custom config loads.
        assert_eq!(
            MatchRules::default(),
            MatchRules {
                points_to_win_game: 4,
                games_to_win_set: 2,
                sets_to_win: 2,
            }
        );
        assert!(MatchRules::custom(4, 2, 0).is_err());
        assert!(MatchRules::custom(0, 2, 2).is_err());
    }

    #[test]
    fn custom_rules_shorten_the_format() {
        // Explicit opt-in only: 1-set match ends at the first set.
        let mut s = Score::with_rules(0, MatchRules::custom(4, 2, 1).unwrap());
        while !s.award_point(Side::Home) {}
        while !s.award_point(Side::Home) {}
        assert_eq!(s.sets, [1, 0]);
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
    fn serve_clock_forfeit_awards_the_serve_to_the_opponent() {
        let mut s = Score::new(0);
        assert_eq!(s.server(), Side::Home);
        s.award_serve(Side::Away);
        assert_eq!(s.server(), Side::Away);
        assert_eq!(s.serve_override, Some(1));
        assert_eq!(s.serve_forfeits, [1, 0]);
        // The award lasts only the current game; rotation resumes after it.
        while !s.award_point(Side::Home) {}
        assert_eq!(s.serve_override, None);
    }

    #[test]
    fn match_point_means_match_not_set() {
        // Set point at 0-0 sets is NOT a match point; at 1-0 it is.
        let mut s = Score::new(0);
        while !s.award_point(Side::Home) {} // game 1
        s.award_point(Side::Home);
        s.award_point(Side::Home);
        s.award_point(Side::Home); // 40-0, one game from the set ...
        assert!(s.is_game_point(Side::Home));
        assert!(!s.is_match_point(Side::Home)); // ... but sets are 0-0
        assert_eq!(s.match_winner(), None);
        let mut t = Score::new(0);
        t.sets = [1, 0];
        t.games = [1, 0];
        t.points = [3, 0];
        assert!(t.is_match_point(Side::Home));
        t.award_point(Side::Home); // set 2-0, match 2-0
        assert_eq!(t.match_winner(), Some(Side::Home));
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
