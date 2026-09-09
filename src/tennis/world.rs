//! Tennis match world â€” the base game model, headless and deterministic.
//!
//! Phase flow mirrors the game's manager (`ServeSetup â†’ Toss â†’ Rally â†’
//! PointPause â†’ â€¦`, v0.12 `core.hpp` Match::step order):
//!
//! 1. **ServeSetup** â€” server/receiver walk to their stances; settle timer.
//! 2. **Toss** â€” the tossed ball rises to `TOSS_HEIGHT`; the server charges
//!    (swing hold) and strikes near the top of the toss.
//! 3. **Rally** â€” ballistic flight with the net collider; per-shot floor
//!    bounce; faults/out/second-bounce resolve the point.
//! 4. **PointPause** â€” `POINT_PAUSE` hold, then the next serve.
//!
//! The world is game-mode explicit by construction: it lives in
//! `tennis::`, consumes [`TennisCommand`]s, and never touches soccer types.
//! One or both sides can be driven by a graph brain (see [`super::vm`]); a
//! side without a command falls back to the deterministic stock returner.

use bevy::prelude::Vec2;
use bevy::prelude::Vec3;

use super::ball::{BallState, FlightEvent, FlightModel};
use super::court::{self, Side};
use super::params::*;
use super::rng::{SplitMix64, UnityRandom};
use super::score::Score;
use super::shot::solve_shot;
use super::shot_type::ShotType;
use crate::brain::TennisCommand;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    ServeSetup,
    Toss,
    Rally,
    PointPause,
    Finished,
}

/// Why the match ended (batch JSON consumers).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EndReason {
    SetWon(Side),
}

#[derive(Debug, Clone, Copy)]
pub struct TennisPlayer {
    pub side: Side,
    /// Pitch-plane position (X depth, Z width). Y is `PLAYER_GROUND_Y`.
    pub pos: Vec2,
    /// Swing charge in [0, 1] while the swing button is held.
    pub charge: f32,
    pub holding: bool,
    /// Post-swing cooldown remaining.
    pub recover: f32,
    /// Last resolved aim (for sensors/debug).
    pub last_aim: Vec2,
}

impl TennisPlayer {
    fn new(side: Side) -> Self {
        Self {
            side,
            pos: Vec2::new(side.sign() * COURT_LENGTH * 0.5, 0.0),
            charge: 0.0,
            holding: false,
            recover: 0.0,
            last_aim: Vec2::ZERO,
        }
    }

    fn racket_center(&self) -> Vec3 {
        // Racket offset toward the opponent: forward 0.55, lateral 0.45
        // toward the ball side is resolved at contact time; the base probe
        // point is forward + height.
        Vec3::new(
            self.pos.x + self.side.other().sign() * RACKET_FORWARD,
            PLAYER_GROUND_Y + STRIKE_HEIGHT,
            self.pos.y,
        )
    }

    /// Public racket readout for the sensor bridge.
    pub fn racket(&self) -> Vec3 {
        self.racket_center()
    }
}

/// How a side plays when no brain command arrives (deterministic stock bot).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StockBehavior {
    /// Deterministic returner: chase, flat shot to the deep middle.
    Returner,
    /// Idle â€” ball flies past (used for fault/ace tests).
    Idle,
}

pub struct TennisWorld {
    pub seed: u64,
    pub score: Score,
    pub phase: Phase,
    pub phase_t: f32,
    pub tick: u64,
    pub sim_time: f32,
    pub ball: BallState,
    /// Ball is with the server (pre-toss / toss held in hand).
    pub ball_held: bool,
    /// Serve is in flight and has not yet bounced legally.
    pub serve_in_flight: bool,
    /// Serve touched the tape (fault).
    pub serve_taped: bool,
    /// The serve's first bounce happened (receiver may now volley).
    pub serve_bounced: bool,
    pub players: [TennisPlayer; 2], // [Home, Away]
    pub flight: FlightModel,
    pub stock: [StockBehavior; 2],
    vm_rng: [SplitMix64; 2],
    unity: UnityRandom,
    last_shot: [Option<ShotType>; 2],
    last_point_winner: Option<Side>,
    last_was_ace: bool,
    /// Per-point rerolled random aim target (`Random Aim Target` sensor).
    random_aim: Vec3,
    pub end: Option<EndReason>,
}

impl TennisWorld {
    pub fn new(seed: u64) -> Self {
        // First server: vm stream draw mod 2 (v0.12 model policy).
        let mut home_rng = SplitMix64::new(seed ^ 0x83a2_f347);
        let away_rng = SplitMix64::new(seed ^ 0x157c_c26d);
        let first_server = home_rng.below(2) as usize;
        let unity = UnityRandom::new(seed as u32);
        let mut w = Self {
            seed,
            score: Score::new(first_server),
            phase: Phase::ServeSetup,
            phase_t: 0.0,
            tick: 0,
            sim_time: 0.0,
            ball: BallState::new(Vec3::ZERO, Vec3::ZERO, ShotType::Flat, 0.0),
            ball_held: true,
            serve_in_flight: false,
            serve_taped: false,
            serve_bounced: false,
            players: [TennisPlayer::new(Side::Home), TennisPlayer::new(Side::Away)],
            flight: FlightModel,
            stock: [StockBehavior::Returner; 2],
            vm_rng: [home_rng, away_rng],
            unity,
            last_shot: [None; 2],
            last_point_winner: None,
            last_was_ace: false,
            random_aim: Vec3::ZERO,
            end: None,
        };
        w.setup_serve();
        w
    }

    /// Side helpers: index 0 = Home, 1 = Away.
    fn idx(side: Side) -> usize {
        side as usize
    }

    pub fn player(&self, side: Side) -> &TennisPlayer {
        &self.players[Self::idx(side)]
    }

    pub fn last_shot(&self, side: Side) -> Option<ShotType> {
        self.last_shot[Self::idx(side)]
    }

    /// Index-based read for the sensor bridge ([0] = Home, [1] = Away).
    pub fn last_shot_idx(&self, idx: usize) -> Option<ShotType> {
        self.last_shot[idx]
    }

    pub fn last_point_winner(&self) -> Option<Side> {
        self.last_point_winner
    }

    /// Deterministic stock returner for a side without a brain.
    fn stock_command(&mut self, side: Side) -> TennisCommand {
        match self.stock[Self::idx(side)] {
            StockBehavior::Idle => TennisCommand::default(),
            StockBehavior::Returner => {
                let me = &self.players[Self::idx(side)];
                let ball_xz = Vec2::new(self.ball.pos.x, self.ball.pos.z);
                let in_range = self.ball_in_strike_range(side);
                let serving_this = self.phase == Phase::Toss && self.score.server() == side;
                let serving_setup = matches!(self.phase, Phase::ServeSetup | Phase::Toss)
                    && self.score.server() == side;
                let aim = if serving_this {
                    // Serves must go to the diagonal box, not deep.
                    court::legal_serve_target(self.score.receiver(), self.score.ad_court())
                } else {
                    court::default_aim_target(side)
                };
                let target = if serving_setup {
                    // Hold the stance while setting up / tossing — do not
                    // chase the ball held in hand.
                    court::serve_stance(self.score.server(), self.score.ad_court())
                } else if self.serve_in_flight && !self.serve_bounced {
                    court::receive_stance(self.score.server(), self.score.ad_court())
                } else {
                    // Intercept: line up with the ball's X on my side.
                    let x = ball_xz.x.abs() * me.side.sign();
                    Vec2::new(x, ball_xz.y.clamp(-6.0, 6.0))
                };
                // Swing model: hold builds charge; releasing (swing=false)
                // strikes. While serving in range, release at ~35% charge so
                // the toss is struck on descent instead of held forever.
                let swing = if serving_this {
                    in_range && me.charge < 0.35
                } else {
                    in_range
                };
                TennisCommand {
                    move_or_aim: if in_range || serving_this { aim } else { target },
                    swing,
                    shot_type: 2.0, // Flat
                    sprint: true,
                }
            }
        }
    }

    fn ball_in_strike_range(&self, side: Side) -> bool {
        let p = self.player(side);
        if self.recover_pending(side) {
            return false;
        }
        let racket = p.racket_center();
        self.ball.pos.distance(racket) <= STRIKE_RADIUS
    }

    fn recover_pending(&self, side: Side) -> bool {
        self.players[Self::idx(side)].recover > 0.0
    }

    /// Advance one fixed-dt tick. `None` command â†’ stock behavior.
    pub fn step(&mut self, commands: [Option<TennisCommand>; 2]) {
        if self.end.is_some() {
            return;
        }
        self.tick += 1;
        self.sim_time += FIXED_DT;
        self.phase_t += FIXED_DT;

        let cmds = [
            commands[0].unwrap_or_else(|| self.stock_command(Side::Home)),
            commands[1].unwrap_or_else(|| self.stock_command(Side::Away)),
        ];

        self.step_players(&cmds);
        match self.phase {
            Phase::ServeSetup => self.step_serve_setup(&cmds),
            Phase::Toss => self.step_toss(&cmds),
            Phase::Rally => self.step_flight(&cmds),
            Phase::PointPause => {
                if self.phase_t >= POINT_PAUSE {
                    self.setup_serve();
                }
            }
            Phase::Finished => {}
        }
    }

    fn step_players(&mut self, cmds: &[TennisCommand; 2]) {
        for i in 0..2 {
            let side = if i == 0 { Side::Home } else { Side::Away };
            let serving_here = self.phase == Phase::ServeSetup
                || (self.phase == Phase::Toss && self.score.server() == side);
            let mut p = self.players[i];
            if p.recover > 0.0 {
                p.recover -= FIXED_DT;
            }
            // Movement toward the command target, clamped to legal ground.
            let target = court::clamp_player_position(side, cmds[i].move_or_aim, serving_here, self.score.ad_court());
            let speed = if cmds[i].sprint { SPRINT_SPEED } else { WALK_SPEED };
            let d = target - p.pos;
            let dist = d.length();
            if dist > 1e-3 {
                let step = (speed * FIXED_DT).min(dist);
                p.pos += d / dist * step;
            }
            // Swing charge model: hold builds charge, release swings.
            if cmds[i].swing && p.recover <= 0.0 {
                if !p.holding {
                    p.holding = true;
                    p.charge = 0.0;
                }
                p.charge = (p.charge + FIXED_DT / CHARGE_WINDUP).min(1.0);
            } else if p.holding {
                // Release this tick â€” contact is resolved in the phase steps.
                p.holding = false;
                self.on_swing_release(i, cmds[i], p.charge);
                p.recover = SWING_SECONDS + RECOVER_SECONDS;
                p.charge = 0.0;
            }
            self.players[i] = p;
        }
    }

    /// Swing release: strike if the ball is in range (phase-dependent rules).
    fn on_swing_release(&mut self, i: usize, cmd: TennisCommand, q: f32) {
        let side = if i == 0 { Side::Home } else { Side::Away };
        if self.phase != Phase::Toss && self.phase != Phase::Rally {
            return;
        }
        if !self.ball_in_strike_range(side) {
            return;
        }
        // Only the server may strike the tossed ball. A receiver swinging at
        // it fouls (v0.14 OnBallStruck pre-bounce receiver → server point).
        if self.phase == Phase::Toss && self.score.server() != side {
            self.score.record_foul(side);
            self.resolve_point(self.score.server(), PointReason::Foul);
            return;
        }
        // Receiver may not volley a serve before its bounce (v0.14
        // MustLetServeBounce / OnBallStruck foul).
        if self.serve_in_flight && !self.serve_bounced && self.score.receiver() == side {
            self.score.record_foul(side);
            self.resolve_point(self.score.server(), PointReason::Foul);
            return;
        }
        let serving_this_contact = self.phase == Phase::Toss && self.score.server() == side;
        let shot = resolve_shot_type(cmd.shot_type, &mut self.vm_rng[Self::idx(side)]);
        let target = if cmd.move_or_aim == Vec2::ZERO {
            if serving_this_contact {
                court::legal_serve_target(self.score.receiver(), self.score.ad_court())
            } else {
                court::default_aim_target(side)
            }
        } else {
            cmd.move_or_aim
        };
        let from = self.ball.pos;
        let target3 = Vec3::new(target.x, BOUNCE_FLOOR_Y, target.y);
        let vel = solve_shot(from, target3, shot, q, &self.flight);
        self.ball = BallState::new(from, vel, shot, q);
        self.players[i].last_aim = target;
        self.last_shot[Self::idx(side)] = Some(shot);
        if q >= 0.75 && matches!(shot, ShotType::Topspin | ShotType::Flat) {
            self.score.record_charged(side);
        }
        if serving_this_contact {
            self.serve_in_flight = true;
            self.serve_taped = false;
            self.serve_bounced = false;
            self.phase = Phase::Rally;
        }
    }

    fn setup_serve(&mut self) {
        let server = self.score.server();
        let ad = self.score.ad_court();
        self.players[Self::idx(server)].pos = court::serve_stance(server, ad);
        self.players[Self::idx(server.other())].pos = court::receive_stance(server, ad);
        self.random_aim = self.reroll_random_aim();
        self.ball_held = true;
        self.serve_in_flight = false;
        self.serve_taped = false;
        self.serve_bounced = false;
        self.last_was_ace = false;
        self.ball = BallState::new(
            Vec3::new(
                self.players[Self::idx(server)].pos.x,
                PLAYER_GROUND_Y + 1.0,
                self.players[Self::idx(server)].pos.y,
            ),
            Vec3::ZERO,
            ShotType::Flat,
            0.0,
        );
        self.phase = Phase::ServeSetup;
        self.phase_t = 0.0;
    }

    fn step_serve_setup(&mut self, _cmds: &[TennisCommand; 2]) {
        // Receiver must be settled (receiver_delay 0.6) before the toss.
        if self.phase_t >= 0.6 {
            self.toss();
        }
    }

    /// Launch the toss (v0.14 `RequestServeToss`): straight up from the hand,
    /// `speed = max(sqrt(d * (g+g)), 11.5)` with the release at toss height.
    fn toss(&mut self) {
        let server = self.score.server();
        let hand = Vec3::new(
            self.players[Self::idx(server)].pos.x,
            PLAYER_GROUND_Y + TOSS_HEIGHT,
            self.players[Self::idx(server)].pos.y,
        );
        let d = (COURT_Y + TOSS_HEIGHT + 1.0) - hand.y;
        let speed = ((d * (GRAVITY + GRAVITY)).sqrt()).max(TOSS_MIN_SPEED);
        self.ball = BallState::new(hand, Vec3::new(0.0, speed, 0.0), ShotType::Flat, 0.0);
        self.ball_held = false;
        self.phase = Phase::Toss;
        self.phase_t = 0.0;
    }

    fn step_toss(&mut self, _cmds: &[TennisCommand; 2]) {
        let _ = self.flight.step(&mut self.ball, FIXED_DT, false);
        let server = self.score.server();
        // Recatch when the tossed ball falls back below hand height (v0.14).
        if self.ball.vel.y < 0.0 && self.ball.pos.y < PLAYER_GROUND_Y + 1.05 {
            // Never tossed in time: re-arm (serve clock policy).
            self.toss();
            return;
        }
        // Ball hanging in the strike window: server may swing (handled by
        // on_swing_release via the Toss phase). Serve clock timeout â†’ re-serve.
        if self.phase_t >= SERVE_CLOCK {
            let _ = server;
            self.setup_serve();
        }
    }

    fn step_flight(&mut self, _cmds: &[TennisCommand; 2]) {
        let event = self.flight.step(&mut self.ball, FIXED_DT, self.serve_in_flight);

        match event {
            FlightEvent::NetTape if self.serve_in_flight => {
                self.serve_taped = true;
                self.resolve_serve_fault();
            }
            FlightEvent::Bounce => {
                if self.serve_in_flight {
                    self.on_serve_bounce();
                } else {
                    self.on_rally_bounce();
                }
            }
            _ => {}
        }

        if self.phase == Phase::Rally && !self.serve_in_flight {
            // Ball rolled dead or flew out of the arena: point resolution.
            let x = self.ball.pos.x;
            let z = self.ball.pos.z;
            if x.abs() > COURT_LENGTH || z.abs() > COURT_DOUBLES_WIDTH {
                let last_striker = self.last_shot[0].is_some().then_some(Side::Home);
                let _ = last_striker;
                self.resolve_out_of_arena();
            }
        }
    }

    fn on_serve_bounce(&mut self) {
        let receiver = self.score.receiver();
        let ad = self.score.ad_court();
        let landing = Vec2::new(self.ball.pos.x, self.ball.pos.z);
        let cross_x = landing.x * receiver.sign() > 0.0; // landed on receiver side
        if !cross_x || !court::is_serve_in(receiver, ad, landing.x, landing.y) {
            self.resolve_serve_fault();
            return;
        }
        self.serve_in_flight = false;
        self.serve_bounced = true;
        self.ball.bounces = 0;
    }

    fn on_rally_bounce(&mut self) {
        // Second bounce on the same side kills the point: the side that did
        // NOT last strike wins. First bounce out of court: last striker loses.
        let landing = Vec2::new(self.ball.pos.x, self.ball.pos.z);
        let striker = self.last_striker();
        let in_court = court::is_in_court_xz(landing.x, landing.y);
        if !in_court {
            // Out: point to the receiver of that shot.
            if let Some(s) = striker {
                self.score.record_out(s);
                self.resolve_point(s.other(), PointReason::Out);
            }
            return;
        }
        if self.ball.bounces >= 2 {
            if let Some(s) = striker {
                self.resolve_point(s.other(), PointReason::DoubleBounce);
            }
        }
    }

    fn last_striker(&self) -> Option<Side> {
        // The side whose shot is currently in flight.
        if self.ball.pos.x < 0.0 {
            // Ball over/bouncing on the home half: away struck last.
            Some(Side::Away).filter(|_| self.last_shot[1].is_some() || self.last_shot[0].is_some())
        } else {
            Some(Side::Home).filter(|_| self.last_shot[0].is_some() || self.last_shot[1].is_some())
        }
    }

    fn resolve_out_of_arena(&mut self) {
        let striker = self.last_striker();
        if let Some(s) = striker {
            self.score.record_out(s);
            self.resolve_point(s.other(), PointReason::Out);
        } else {
            self.resolve_point(self.score.receiver(), PointReason::Out);
        }
    }

    fn resolve_serve_fault(&mut self) {
        let server = self.score.server();
        if self.score.serve_number == 1 {
            self.score.fault(server);
            self.setup_serve();
        } else {
            let game_won = self.score.double_fault(server);
            self.after_point(server.other(), PointReason::DoubleFault, game_won, false);
        }
    }

    /// Strike-based point (rally outcome or foul).
    fn resolve_point(&mut self, winner: Side, reason: PointReason) {
        let game_won = self.score.award_point(winner);
        let ace = reason == PointReason::Ace;
        if ace {
            self.score.record_ace(winner);
        }
        self.after_point(winner, reason, game_won, ace);
    }

    fn after_point(&mut self, winner: Side, _reason: PointReason, game_won: bool, _ace: bool) {
        self.last_point_winner = Some(winner);
        if self.score.sets[Self::idx(winner)] > 0 && game_won {
            self.end = Some(EndReason::SetWon(winner));
            self.phase = Phase::Finished;
            return;
        }
        self.phase = Phase::PointPause;
        self.phase_t = 0.0;
    }

    fn reroll_random_aim(&mut self) -> Vec3 {
        // v0.14 RerollRandomShotAndAim order: x range, then z range, over the
        // opponent court (RandomRangeInt(0,7) picks a random SHOT for the
        // Shot: Random option; the aim covers the legal court area).
        let x = self.unity.range(2.0, COURT_LENGTH * 0.48);
        let z = self.unity.range(-COURT_SINGLES_WIDTH * 0.4, COURT_SINGLES_WIDTH * 0.4);
        Vec3::new(x, BOUNCE_FLOOR_Y, z)
    }

    pub fn random_aim_target(&self) -> Vec3 {
        self.random_aim
    }

    /// Predicted first/second bounce for the sensors (None when dead).
    pub fn predicted_bounce(&self, nth: usize) -> Option<(f32, f32, f32)> {
        let mut probe = self.ball;
        probe.bounces = 0;
        self.flight
            .predict_landing(&mut probe, nth)
            .map(|l| (l.x, l.z, l.w))
    }

    // ---- Sensor-facing readouts (see api.rs) ----

    pub fn ball_speed(&self) -> f32 {
        self.ball.vel.length()
    }

    pub fn shot_dropdown_value(&self, _side: Side, label_index: usize) -> f32 {
        // "Shot: X" floats enumerate their own dropdown position.
        label_index as f32
    }

    pub fn last_shot_dropdown(&self, side: Side) -> f32 {
        self.last_shot[Self::idx(side)]
            .map(|s| s as usize as f32)
            .unwrap_or(-1.0)
    }

    pub fn is_charging(&self, side: Side) -> bool {
        self.players[Self::idx(side)].holding
    }

    pub fn charge_pct(&self, side: Side) -> f32 {
        self.players[Self::idx(side)].charge
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PointReason {
    DoubleBounce,
    Out,
    Foul,
    DoubleFault,
    Ace,
}

/// Resolve the controller's `Float1` shot dropdown value (data.py order:
/// 0 Topspin, 1 Slice, 2 Flat, 3 Drop, 4 Lob, 5 Curve Left, 6 Curve Right,
/// 7/other = Random via the VM stream â€” `rng.next() % 7`).
fn resolve_shot_type(value: f32, rng: &mut SplitMix64) -> ShotType {
    match value.round() as i32 {
        0 => ShotType::Topspin,
        1 => ShotType::Slice,
        2 => ShotType::Flat,
        3 => ShotType::Drop,
        4 => ShotType::Lob,
        5 => ShotType::CurveLeft,
        6 => ShotType::CurveRight,
        _ => {
            let raw = rng.below(7) as i32;
            ShotType::from_physical(raw)
        }
    }
}
