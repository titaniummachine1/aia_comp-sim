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
    /// Stamina meter [0, 1] (`Stamina.OnSimulationTick`): drains while
    /// sprinting, regenerates while standing, walking is neutral. Feeds the
    /// `Self/Opponent Stamina Pct` inputs; NOT reset at point boundaries.
    pub stamina: f32,
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
            stamina: 1.0,
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
    /// Seconds into the current serve phase (ServeSetup + Toss, **not** reset by
    /// a recatch re-toss). When it reaches [`params::SERVE_CLOCK`] the serve is
    /// forfeited to the opponent (game `serveDeadlineTick` /
    /// `<ServeCountdownDisplay>`).
    pub serve_clock_t: f32,
    /// v0.15+: interpolate the ball between frames, so contact uses a swept
    /// segment test (a fast ball must not skip the strike window in one tick).
    /// Default `false` = v0.14 end-of-tick point test.
    interpolate_ball: bool,
    /// Ball position at the start of this tick — the swept-contact segment start.
    pub ball_prev: Vec3,
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
    /// Shots played in the current rally (both sides) — drives fatigue.
    pub rally_hits: i32,
    /// Serve aim latched on Toss entry (v0.14 `ServeAimHint`
    /// semantics): the live wires during ServeSetup are the bot's stance /
    /// rally-default output, NOT the strike aim (game truth: titanium54's
    /// final_aim reads best-point junk at setup entry, switches to the
    /// serve branch (7.0, 0.0) once serving goes live, all serves land
    /// (7.0077, 0.0)). Validated into the legal diagonal box at
    /// latch time; `None` → `legal_serve_target` fallback at the strike.
    serve_aim_latch: Option<Vec2>,
    /// Side that struck the ball most recently; cleared on bounce. Blocks
    /// double hits (a swing-pulse re-striking the just-served/just-hit ball
    /// mid-flight — the game never produces those faults).
    strike_lock: Option<Side>,
    /// Last striker, tracked exactly (set on every strike, cleared only at
    /// point setup). Unlike `strike_lock` this survives the bounce: drives
    /// `Ball Incoming` (true iff the OPPONENT struck last and the ball is
    /// still un-bounced) and out-of-arena attribution (position heuristic
    /// is fallback only).
    hit_by: Option<Side>,
    /// Raw command aims per side ([0]=Home, [1]=Away) — diagnostics for the
    /// aim-semantics investigation (trace only).
    pub last_cmd_aim: [Vec2; 2],
    /// Latched rally strike aim per side. Only opponent-court outputs count
    /// as shot aims: positioning/chase outputs (own half, ball position —
    /// what brains emit while receiving) must not become strike targets, or
    /// every return duds. Mirrors the game's latched on-hit aim (same
    /// mechanism as the captured deep `ServeAimHint` at serve time).
    rally_aim_latch: [Option<Vec2>; 2],
    /// Aim model: when true (`AIA_AIM_MODEL=separate`), rally + serve
    /// latches read the pre-gate aim request (`TennisCommand.aim`,
    /// resolved through `Controller <- AutoMove(V32) <- AutoAim`) while
    /// movement keeps using `move_or_aim`. DEFAULT false (legacy wire):
    /// measured 2026-09-11 on the 78-row game set with the current sim,
    /// separate scored 18/78 vs legacy 24/78 — the native-bot aim path is
    /// not yet modelled faithfully (AutoAim clamp missing), so legacy stays
    /// the default until that lands. Compiler bots that split walk/aim via
    /// t.aim() set AIA_AIM_MODEL=separate.
    separate_aim: bool,
    /// Swing gate: when true, the world holds the swing through the approach
    /// whenever the phase allows (see `step_players`), reproducing the game's
    /// 0.6-1.2 s approach holds. **Default ON** since the per-tick channel diff
    /// (HANDOFF section 20) matches the game (`SwingHeld` fires, `ChargePct`
    /// mean 0.28); without the gate the sim's `ChargePct` is pegged 0.000.
    /// `AIA_SWING_MODEL=legacy` (or `off`/`false`/`0`) restores the old default
    /// so the recorded sweep verdicts stay reproducible.
    swing_hold_gate: bool,
}

impl TennisWorld {
    pub fn new(seed: u64) -> Self {
        // First server: vm stream draw mod 2 (v0.12 model policy).
        // Override for calibration: AIA_FIRST_SERVER=home/away forces the
        // opener (the game's load-side assignment is still unpinned: it
        // strongly favors the left/home player across restarts with rare
        // flips, matching neither a fair coin nor seed parity).
        let mut home_rng = SplitMix64::new(seed ^ 0x83a2_f347);
        let away_rng = SplitMix64::new(seed ^ 0x157c_c26d);
        let first_server = match std::env::var("AIA_FIRST_SERVER").as_deref() {
            Ok("home") => 0,
            Ok("away") => 1,
            _ => home_rng.below(2) as usize,
        };
        let unity = UnityRandom::new(seed as u32);
        let mut w = Self {
            seed,
            score: Score::new(first_server),
            phase: Phase::ServeSetup,
            phase_t: 0.0,
            tick: 0,
            sim_time: 0.0,
            ball: BallState::new(Vec3::ZERO, Vec3::ZERO, ShotType::Flat.game_arg(), 0.0),
            ball_held: true,
            serve_in_flight: false,
            serve_taped: false,
            serve_bounced: false,
            serve_clock_t: 0.0,
            interpolate_ball: false,
            ball_prev: Vec3::ZERO,
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
            rally_hits: 0,
            serve_aim_latch: None,
            strike_lock: None,
            hit_by: None,
            last_cmd_aim: [Vec2::ZERO; 2],
            rally_aim_latch: [None; 2],
            separate_aim: std::env::var("AIA_AIM_MODEL").as_deref() == Ok("separate"),
            swing_hold_gate: !matches!(
                std::env::var("AIA_SWING_MODEL").as_deref(),
                Ok("legacy") | Ok("off") | Ok("false") | Ok("0")
            ),
        };
        w.setup_serve();
        w
    }

    /// Build a world for an explicit game version. v0.15+ turns on the
    /// frame-interpolated (swept) ball contact; v0.14 keeps the point test.
    /// [`Self::new`] stays v0.14 so existing callers/results are unchanged.
    pub fn for_spec(seed: u64, spec: crate::mode::GameSpec) -> Self {
        let mut w = Self::new(seed);
        w.interpolate_ball = spec.version.interpolates_ball();
        w
    }

    /// Aim source for the latches: the pre-gate request when the separate
    /// aim model is on and the graph provides one, else the legacy wire.
    fn latch_candidate(&self, cmd: &TennisCommand) -> Vec2 {
        if self.separate_aim {
            cmd.aim.unwrap_or(cmd.move_or_aim)
        } else {
            cmd.move_or_aim
        }
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

    /// Tracked last striker (exact; survives the bounce, unlike
    /// `strike_lock`). Sensor bridge for `Ball Incoming`.
    pub fn hit_by_side(&self) -> Option<Side> {
        self.hit_by
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
                    // Stock has no AutoAim path; its move_or_aim already
                    // carries the deliberate aim (deep/serve-target), so the
                    // legacy latch path applies.
                    aim: None,
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
        let distance = if self.interpolate_ball {
            point_segment_distance(racket, self.ball_prev, self.ball.pos)
        } else {
            self.ball.pos.distance(racket)
        };
        distance <= STRIKE_RADIUS
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
        // Start-of-tick ball position — the swept-contact segment start (v0.15).
        self.ball_prev = self.ball.pos;

        // Serve clock (`serveDeadlineTick`). Runs across ServeSetup + Toss and
        // is NOT reset by a recatch re-toss, so a server that never gets the
        // serve away forfeits it to the opponent when it expires.
        if matches!(self.phase, Phase::ServeSetup | Phase::Toss) {
            self.serve_clock_t += FIXED_DT;
            if self.serve_clock_t >= SERVE_CLOCK {
                self.on_serve_deadline();
            }
        }

        let cmds = [
            commands[0].unwrap_or_else(|| self.stock_command(Side::Home)),
            commands[1].unwrap_or_else(|| self.stock_command(Side::Away)),
        ];

        // Latch the serve aim here, BEFORE step_players: on the first Toss
        // tick the server's swing release strikes the just-tossed ball
        // inside step_players, flipping phase to Rally before the phase
        // dispatch below ever sees Toss (step_toss only handles a hanging
        // toss). This tick's cmds were computed with phase == Toss visible,
        // so `Is Self Actively Serving` is true and the aim wire carries
        // the serve branch (titanium54: (7.0, 0.0); game lands (7.0077,
        // 0.0)). A candidate inside the legal diagonal box is honored;
        // anything else falls back to the box center. `is_none` pins the
        // first Toss tick — the strike-tick wire has already flipped back
        // to the rally default and must not overwrite.
        if self.phase == Phase::Toss && self.serve_aim_latch.is_none() {
            let server = self.score.server();
            let receiver = self.score.receiver();
            let ad = self.score.ad_court();
            let candidate = self.latch_candidate(&cmds[Self::idx(server)]);
            let latched = if candidate != Vec2::ZERO
                && court::is_serve_in(receiver, ad, candidate.x, candidate.y)
            {
                candidate
            } else {
                court::legal_serve_target(receiver, ad)
            };
            self.serve_aim_latch = Some(latched);
        }

        // Latch rally aims BEFORE step_players so a strike this tick uses
        // this tick's opponent-court output.
        let side_gate = std::env::var("AIA_RALLY_LATCH").as_deref() == Ok("side");
        for i in 0..2 {
            let side = if i == 0 { Side::Home } else { Side::Away };
            let aim = self.latch_candidate(&cmds[i]);
            if aim.x * side.other().sign() > 0.0 {
                if !side_gate || self.ball.pos.x * side.sign() > 0.0 {
                    self.rally_aim_latch[i] = Some(aim);
                }
            }
        }
        self.step_players(&cmds);
        self.last_cmd_aim = [cmds[0].move_or_aim, cmds[1].move_or_aim];
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
                // Sprint movement drains stamina (measured -0.010/tick);
                // walking is neutral (long flat plateaus in the timeplot).
                if cmds[i].sprint {
                    p.stamina = (p.stamina - STAMINA_SPRINT_DRAIN).max(0.0);
                }
            } else {
                // Standing still regenerates (measured +0.001/tick).
                p.stamina = (p.stamina + STAMINA_REGEN).min(1.0);
            }
            // Swing charge model: hold builds charge, release swings.
            // `TennisAutoSwing` gate (`AIA_SWING_MODEL=hold`): the game
            // holds the swing through the approach (54% of ticks, 0.6-1.2 s
            // holds measured in native timeplots) and releases at contact —
            // the graph never chooses to hold (AutoSwing has no Bool input,
            // only the swing/shake decision outputs). The gate holds when
            // the phase allows swinging: server setup/toss (pre-charge) and
            // rallies; receivers join once the serve is live-and-bounced.
            // Without the gate, the live `in_range && !must_wait` flicker
            // wipes charge every tick and no strike ever charges (sim
            // ChargePct was pegged 0 vs 0.28 game mean).
            let gate_hold = self.swing_hold_gate
                && p.recover <= 0.0
                && (self.phase == Phase::Rally
                    || (self.score.server() == side
                        && (self.phase == Phase::ServeSetup || self.phase == Phase::Toss)));
            let swinging = cmds[i].swing || gate_hold;
            if swinging && p.recover <= 0.0 {
                if !p.holding {
                    p.holding = true;
                    p.charge = 0.0;
                }
                p.charge = (p.charge + FIXED_DT / CHARGE_WINDUP).min(1.0);
                // Game auto-contact (`TennisAutoSwing`): a held swing strikes
                // as soon as the ball is in the racket zone — hold duration
                // sets the charge. (Brains that hold while in range and
                // release when it exits could never connect otherwise.)
                // Toss strike: server only, on descent (game: strike at the
                // toss apex). Skipped when striking would be a foul
                // (receiver pre-serve-bounce).
                let tossing = self.phase == Phase::Toss && self.score.server() == side;
                let foul_risk = self.serve_in_flight
                    && !self.serve_bounced
                    && self.score.receiver() == side;
                let in_toss_window = !tossing || self.ball.vel.y < 0.0;
                let striking_phase = self.phase == Phase::Rally || tossing;
                if striking_phase
                    && !foul_risk
                    && in_toss_window
                    && self.ball_in_strike_range(side)
                {
                    let q = p.charge;
                    let cmd = cmds[i];
                    p.holding = false;
                    self.on_swing_release(i, cmd, q);
                    p.recover = SWING_SECONDS + RECOVER_SECONDS;
                    p.charge = 0.0;
                }
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
        // Turn rule (user-confirmed): once a side strikes, only the OPPONENT
        // may strike next. A side may never hit the ball it just hit again -
        // not even after a bounce. `hit_by` is the last striker and survives
        // the bounce, so it *is* the turn marker; `strike_lock` is the
        // within-flight latch (redundant, kept so a swing-pulse bot can never
        // reset the ball it just served/hit). Before this, clearing the lock on
        // bounce let the striker legally hit its own shot a second time.
        if self.strike_lock == Some(side) || self.hit_by == Some(side) {
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
        self.do_strike(i, cmd, q, serving_this_contact);
    }

    /// Strike core: aim, fatigue scatter, launch. `serving` marks the toss
    /// strike (serve-in-flight flags + phase → Rally).
    fn do_strike(&mut self, i: usize, cmd: TennisCommand, q: f32, serving: bool) {
        let side = if i == 0 { Side::Home } else { Side::Away };
        let shot = resolve_shot_type(cmd.shot_type, &mut self.vm_rng[Self::idx(side)]);
        // --- Anti-stalemate fatigue (recovered v0.12 formulas) ---
        self.rally_hits += 1;
        let active = super::params::fatigue_points(self.rally_hits, self.extra_deuce());
        let q_floored = q.max(charge_floor());
        let q_fatigued = super::params::fatigued_charge(q_floored, active);
        let mut target = if serving {
            // Serve strike uses the ServeAimHint latched at ServeSetup
            // entry — the live Vector31 here is the stance/move output
            // (using it fires the serve backward into the server's own
            // court: the all-double-fault blocker).
            self.serve_aim_latch.unwrap_or_else(|| {
                court::legal_serve_target(self.score.receiver(), self.score.ad_court())
            })
        } else {
            // Rally strike: latched opponent-court aim (positioning targets
            // never overwrite it); deep default when never latched.
            // Sweep gate (no rebuild per variant): AIA_RALLY_DEFAULT =
            //   middle (default_aim_target ±13.44,0) | corner (±13.75,∓4.5)
            //   | back (opponent center_of_back ±10.5,0) | opp (opponent pos)
            //   | open (mirror-z of opponent pos).
            // AIA_MIN_CHARGE = float floor for never-holding bots (q=0 → 85%).
            let latched = self.rally_aim_latch[Self::idx(side)];
            let fallback = rally_fallback_target(self, side);
            // AIA_RALLY_LATCH=off: no latch — strike-tick live aim when it is
            // opponent-court, else fallback. Tests whether latching itself
            // (vs strike-tick aim) is the parity lever.
            // AIA_RALLY_LATCH=side: latch only while the ball is on the
            // striker's own half (receive-transition gate) — positioning
            // blips emitted while the ball is away never overwrite.
            if std::env::var("AIA_RALLY_LATCH").as_deref() == Ok("off") {
                let live = self.latch_candidate(&cmd);
                if live.x * side.other().sign() > 0.0 {
                    live
                } else {
                    fallback
                }
            } else {
                latched.unwrap_or(fallback)
            }
        };
        // Aim scatter: two Unity RNG draws (lateral, then depth) at active
        // fatigue, scaled by the recovered rates and court size.
        if active > 0 {
            let lateral = self.unity.range(-1.0, 1.0)
                * (FATIGUE_LATERAL * COURT_SINGLES_WIDTH * active as f32);
            let longitudinal = self.unity.range(-1.0, 1.0)
                * (FATIGUE_DEPTH * COURT_LENGTH * active as f32);
            let attack = Vec2::new(-side.sign(), 0.0); // toward the opponent
            let right = Vec2::new(0.0, -attack.x); // cross((0,1,0), attack).xz
            target.x += right.x * lateral + attack.x * longitudinal;
            target.y += right.y * lateral + attack.y * longitudinal;
        }
        let from = self.ball.pos;
        let target3 = Vec3::new(target.x, BOUNCE_FLOOR_Y, target.y);
        let vel = solve_shot(from, target3, shot.game_arg(), q_fatigued, &self.flight);
        self.ball = BallState::new(from, vel, shot.game_arg(), q);
        self.players[i].last_aim = target;
        self.last_shot[Self::idx(side)] = Some(shot);
        self.strike_lock = Some(side);
        self.hit_by = Some(side);
        if q >= 0.75 && matches!(shot, ShotType::Topspin | ShotType::Flat) {
            self.score.record_charged(side);
        }
        if serving {
            self.serve_in_flight = true;
            self.serve_taped = false;
            self.serve_bounced = false;
            self.phase = Phase::Rally;
        }
    }

    fn setup_serve(&mut self) {
        let server = self.score.server();
        let ad = self.score.ad_court();
        // Serve setup forces the SERVER to its stance. The receiver is
        // deliberately untouched: game truth (titanium54 native timeplots
        // 2026-09-11) shows the receiver free-walking the whole setup —
        // holding (8.15, 2.52), then wandering out to (10.48, 10.92),
        // mid-court (2.7, -0.5), wherever its brain (or a human hand)
        // takes it. Never place it at receive_stance here.
        self.players[Self::idx(server)].pos = court::serve_stance(server, ad);
        self.random_aim = self.reroll_random_aim();
        self.ball_held = true;
        self.serve_in_flight = false;
        self.serve_taped = false;
        self.serve_bounced = false;
        self.last_was_ace = false;
        self.rally_hits = 0;
        // The strike aim is latched fresh each point on Toss entry (see
        // step_toss) — clear any previous point's latch here.
        self.serve_aim_latch = None;
        self.strike_lock = None;
        self.hit_by = None;
        // Holds never carry across points (game charge reads 0 at point
        // starts; point-boundary holds always release first).
        for p in self.players.iter_mut() {
            p.holding = false;
            p.charge = 0.0;
        }
        self.ball = BallState::new(
            Vec3::new(
                self.players[Self::idx(server)].pos.x,
                PLAYER_GROUND_Y + 1.0,
                self.players[Self::idx(server)].pos.y,
            ),
            Vec3::ZERO,
            ShotType::Flat.game_arg(),
            0.0,
        );
        self.phase = Phase::ServeSetup;
        self.phase_t = 0.0;
        self.serve_clock_t = 0.0;
    }

    /// Serve-clock expiry: the game awards the serve to the opponent
    /// (`serveDeadlineTick` / `<ServeCountdownDisplay>` reaching 0). No point
    /// is scored — only the serve changes hands, for the rest of the game
    /// (`Score::award_serve` clears the override when the game ends).
    fn on_serve_deadline(&mut self) {
        let forfeiter = self.score.server();
        self.score.award_serve(forfeiter.other());
        self.setup_serve();
    }

    fn step_serve_setup(&mut self, cmds: &[TennisCommand; 2]) {
        // The strike aim is NOT latched here: at setup entry the server
        // brain has not seen Toss yet (`Is Self Actively Serving` is false),
        // so its aim wire still carries the rally default / stance output.
        // Game truth (titanium54 native timeplots 2026-09-11): final_aim
        // reads best-point junk at setup entry, switches to the serve branch
        // (7.0, 0.0) once serving goes live mid-setup, and all 8 serves land
        // (7.0077, 0.0000) — the latch window is the first Toss tick, see
        // step().
        let _ = cmds;
        // Setup settle timer before the toss (server walk stand-in; the
        // sim teleports the server, the game walks it ~1.2 s). This gates
        // nothing about the receiver — it walks free the whole time.
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
        self.ball = BallState::new(hand, Vec3::new(0.0, speed, 0.0), ShotType::Flat.game_arg(), 0.0);
        self.ball_held = false;
        self.phase = Phase::Toss;
        self.phase_t = 0.0;
    }

    fn step_toss(&mut self, cmds: &[TennisCommand; 2]) {
        let _ = self.flight.step(&mut self.ball, FIXED_DT, false);
        let server = self.score.server();
        // NOTE: the serve aim is latched in step(), before step_players —
        // on the first Toss tick the release-strike there flips phase to
        // Rally before this dispatch runs, so a latch here would never fire.
        // This step only handles a hanging toss (recatch / serve clock).
        // Auto-strike the serve (game `TennisAutoSwing`): as the tossed ball
        // descends into the racket zone the serve fires with the latched
        // ServeAimHint, whatever the server brain's swing output — bots that
        // hold (or never press) swing still serve, like in the game.
        if self.ball.vel.y < 0.0 {
            let i = Self::idx(server);
            let racket = self.players[i].racket_center();
            let distance = if self.interpolate_ball {
                point_segment_distance(racket, self.ball_prev, self.ball.pos)
            } else {
                self.ball.pos.distance(racket)
            };
            if distance <= STRIKE_RADIUS {
                let q = self.players[i].charge;
                let cmd = cmds[i];
                self.do_strike(i, cmd, q, true);
                return;
            }
        }
        // Recatch when the tossed ball falls back below hand height (v0.14).
        if self.ball.vel.y < 0.0 && self.ball.pos.y < PLAYER_GROUND_Y + 1.05 {
            // Never tossed in time: re-arm (serve clock policy).
            self.toss();
            return;
        }
        // Ball hanging in the strike window: server may swing (handled by
        // on_swing_release via the Toss phase). The serve clock is handled
        // centrally in `step` (it must survive recatches).
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
        self.strike_lock = None;
        // Keep `ball.bounces` (now 1): the game's `Ball Has Bounced` sensor
        // must read true after the serve bounce or receiver brains never
        // return serve, and the next bounce resolves the point (bounces 2).
    }

    fn on_rally_bounce(&mut self) {
        // Second bounce on the same side kills the point: the side that did
        // NOT last strike wins. First bounce out of court: last striker loses.
        self.strike_lock = None;
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
        // Exact track first; the position heuristic is fallback only (it
        // misattributes wrong-way duds hit from the striker's own half).
        if let Some(s) = self.hit_by {
            return Some(s);
        }
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

    /// Extra deuce points beyond 3/3 (fatigue deuce component input).
    pub fn extra_deuce(&self) -> i32 {
        if self.score.points[0] >= 3 && self.score.points[1] >= 3 {
            (self.score.points[0] + self.score.points[1] - 6) as i32
        } else {
            0
        }
    }

    /// Active fatigue points for the next strike (sensors + tests).
    pub fn active_fatigue(&self) -> i32 {
        super::params::fatigue_points(self.rally_hits, self.extra_deuce())
    }

    /// Rally-side fatigue component (for the `Rally Fatigue` sensor).
    pub fn rally_fatigue(&self) -> i32 {
        super::params::rally_fatigue_points(self.rally_hits)
    }

    /// Deuce-side fatigue component (for the `Deuce Fatigue` sensor).
    pub fn deuce_fatigue(&self) -> i32 {
        super::params::deuce_fatigue_points(self.extra_deuce())
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

/// Shortest distance from `p` to the segment `a`–`b`. The v0.15 swept-contact
/// test: a ball that moves further than `STRIKE_RADIUS` in one tick still
/// counts as struck when its swept path crosses the racket sphere.
fn point_segment_distance(p: Vec3, a: Vec3, b: Vec3) -> f32 {
    let ab = b - a;
    let len2 = ab.length_squared();
    if len2 <= 1e-12 {
        return p.distance(a);
    }
    let t = ((p - a).dot(ab) / len2).clamp(0.0, 1.0);
    p.distance(a + ab * t)
}

/// Rally fallback aim when no opponent-court output was ever latched.
/// Env-gated sweep (default "middle"); see do_strike call site.
fn rally_fallback_target(world: &TennisWorld, side: Side) -> Vec2 {
    let mode = std::env::var("AIA_RALLY_DEFAULT").unwrap_or_default();
    let opp = world.player(side.other()).pos;
    match mode.as_str() {
        "corner" => {
            let z = if world.score.ad_court() { -4.5 } else { 4.5 };
            Vec2::new(side.other().sign() * 13.75, z)
        }
        "back" => court::center_of_back(side.other()),
        "opp" => opp,
        "open" => Vec2::new(opp.x, -opp.y),
        _ => court::default_aim_target(side),
    }
}

/// Min-charge floor for auto-contact strikes (env AIA_MIN_CHARGE, default 0).
/// Never-holding bots strike at q=0 → 85% speed; the game's truth is unknown.
fn charge_floor() -> f32 {
    std::env::var("AIA_MIN_CHARGE")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0.0)
}

/// Resolve the controller's `Float1` shot dropdown value (data.py order:
/// 0 Topspin, 1 Slice, 2 Flat, 3 Drop, 4 Lob, 5 Curve Left, 6 Curve Right,
/// 7/other = Random via the VM stream — `rng.next() % 7`).
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
#[cfg(test)]
mod serve_clock_tests {
    use super::*;

    /// The serve clock is a refusal forfeit: when it expires the opponent is
    /// awarded the serve (no point scored), and the new server's clock restarts.
    #[test]
    fn serve_clock_expiry_awards_the_serve_to_the_opponent() {
        let mut w = TennisWorld::new(0);
        let before = w.score.server();
        w.phase = Phase::ServeSetup;
        w.serve_clock_t = SERVE_CLOCK; // deadline already reached
        w.step([None, None]);
        assert_eq!(w.score.server(), before.other());
        assert_eq!(w.score.serve_forfeits[before as usize], 1);
        assert_eq!(w.serve_clock_t, 0.0, "the new server's clock restarts");
    }

    /// The clock spans the whole serve: a recatch re-toss must not clear it.
    #[test]
    fn serve_clock_survives_a_recatch() {
        let mut w = TennisWorld::new(0);
        w.phase = Phase::Toss;
        w.serve_clock_t = 1.0;
        w.toss();
        assert!(
            w.serve_clock_t >= 1.0,
            "re-toss must not clear the serve clock"
        );
    }

    /// v0.15 turns on the interpolated (swept) ball contact; v0.14 keeps the
    /// end-of-tick point test; `new()` stays v0.14.
    #[test]
    fn version_selects_swept_ball_contact() {
        use crate::mode::GameSpec;
        assert!(!TennisWorld::new(0).interpolate_ball);
        assert!(!TennisWorld::for_spec(0, GameSpec::tennis_v014()).interpolate_ball);
        assert!(TennisWorld::for_spec(0, GameSpec::tennis_v015()).interpolate_ball);
    }
}

#[cfg(test)]
mod turn_rule_tests {
    use super::*;

    /// Park the ball exactly on `side`'s racket so it is in strike range.
    fn rally_world_with_ball_on(side: Side) -> TennisWorld {
        let mut w = TennisWorld::new(0);
        w.phase = Phase::Rally;
        let racket = w.player(side).racket_center();
        w.ball = BallState::new(racket, Vec3::ZERO, ShotType::Flat.game_arg(), 0.0);
        w
    }

    /// It is turn-based: after a side strikes, only the opponent may strike
    /// next. The striker must NOT be able to hit the ball it just hit again,
    /// even once the ball has bounced (the lock clears on bounce).
    #[test]
    fn a_side_cannot_strike_twice_in_a_row() {
        let home = Side::Home;
        let mut w = rally_world_with_ball_on(home);
        let cmd = TennisCommand {
            swing: true,
            ..Default::default()
        };
        // Opponent struck last -> it is Home's turn.
        w.hit_by = Some(home.other());
        w.strike_lock = None;
        w.on_swing_release(0, cmd, 0.9);
        assert_eq!(w.hit_by, Some(home), "Home struck on its turn");
        let after_first = w.ball.vel;
        // Simulate the ball bouncing: the in-flight lock clears, but the turn
        // is still the opponent's. A second Home swing must be a no-op.
        w.strike_lock = None;
        w.on_swing_release(0, cmd, 0.9);
        assert_eq!(
            w.ball.vel, after_first,
            "Home must not hit its own shot again (double hit)"
        );
        assert_eq!(w.hit_by, Some(home));
    }

    /// The opponent's turn arrives when the opponent strikes; then the original
    /// striker may hit again (the turn flips back).
    #[test]
    fn the_turn_flips_when_the_opponent_strikes() {
        let home = Side::Home;
        let away = home.other();
        let mut w = rally_world_with_ball_on(home);
        w.hit_by = Some(home); // Home already struck; turn is Away's
        let cmd = TennisCommand {
            swing: true,
            ..Default::default()
        };
        // Away may not strike: the ball is on Home's racket, not in range.
        w.on_swing_release(1, cmd, 0.9);
        assert_eq!(w.hit_by, Some(home), "out-of-range Away swing is a no-op");
        // Now give Away the ball and the turn: Away strikes.
        let racket = w.player(away).racket_center();
        w.ball = BallState::new(racket, Vec3::ZERO, ShotType::Flat.game_arg(), 0.0);
        w.on_swing_release(1, cmd, 0.9);
        assert_eq!(w.hit_by, Some(away), "Away struck on its turn");
        // Turn is Home's again: move the ball to Home and strike.
        let racket = w.player(home).racket_center();
        w.ball = BallState::new(racket, Vec3::ZERO, ShotType::Flat.game_arg(), 0.0);
        w.on_swing_release(0, cmd, 0.9);
        assert_eq!(w.hit_by, Some(home), "turn flipped back to Home");
    }
}

#[cfg(test)]
mod stamina_tests {
    use super::*;

    /// Rally-phase world with the ball parked mid-court (no point resolves in
    /// one step).
    fn rally_world() -> TennisWorld {
        let mut w = TennisWorld::new(0);
        w.phase = Phase::Rally;
        w.ball = BallState::new(
            Vec3::new(-5.0, 1.0, 0.0),
            Vec3::ZERO,
            ShotType::Flat.game_arg(),
            0.0,
        );
        w
    }

    /// Measured game rule: sprinting costs 0.010 stamina per tick.
    #[test]
    fn sprinting_drains_one_percent_per_tick() {
        let mut w = rally_world();
        let cmd = TennisCommand {
            move_or_aim: Vec2::new(-8.0, 5.0), // far target: keeps moving
            sprint: true,
            ..Default::default()
        };
        w.step([Some(cmd), None]);
        let got = w.player(Side::Home).stamina;
        assert!(
            (got - (1.0 - STAMINA_SPRINT_DRAIN)).abs() < 1e-6,
            "sprint tick must drain {STAMINA_SPRINT_DRAIN}, got {got}"
        );
    }

    /// Measured game rule: standing still regenerates 0.001 per tick. (Drain
    /// first — regen clamps at the 1.0 ceiling.)
    #[test]
    fn standing_still_regenrates_a_tenth_percent_per_tick() {
        let mut w = rally_world();
        let sprint = TennisCommand {
            move_or_aim: Vec2::new(-8.0, 5.0),
            sprint: true,
            ..Default::default()
        };
        w.step([Some(sprint), None]);
        w.step([Some(sprint), None]);
        let drained = w.player(Side::Home).stamina;
        // Stand still: target the player's CURRENT position (they moved during
        // the sprint ticks, so the old stance is a walk target, not "stand").
        let here = w.player(Side::Home).pos;
        let stand = TennisCommand {
            move_or_aim: here,
            ..Default::default()
        };
        w.step([Some(stand), None]);
        let got = w.player(Side::Home).stamina;
        assert!(
            (got - (drained + STAMINA_REGEN)).abs() < 1e-6,
            "standing tick must regen {STAMINA_REGEN} from {drained}, got {got}"
        );
    }

    /// Measured game rule: walking (moving without sprint) is neutral — the
    /// native timeplot shows flat plateaus during long rallies.
    #[test]
    fn walking_is_neutral() {
        let mut w = rally_world();
        let cmd = TennisCommand {
            move_or_aim: Vec2::new(-8.0, 5.0),
            sprint: false,
            ..Default::default()
        };
        w.step([Some(cmd), None]);
        assert!((w.player(Side::Home).stamina - 1.0).abs() < 1e-6);
    }
}



