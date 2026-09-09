//! Ball flight â€” port of the v0.14 recovered physics
//! (`recovered_flight_v014.hpp` forces + stepping, `recovered_ball_v014.hpp`
//! net cross/tape/rebound + floor bounce, `recovered_prediction_v014.hpp`
//! NthLanding predictor).
//!
//! Force model (per substep dt): with `pace = |v| / MAX_SPEED`,
//! - lateral curve accel `right Â· paceÂ˛ Â· curve`, curve decaying at
//!   `(curve shots ? 0.75 : 2.4) Â· pace` per second,
//! - gravity `paceÂ˛ Â· g` downward,
//! - speed capped at `MAX_SPEED Â· max(1, pace)`.
//!
//! Scaling accelerations by paceÂ˛ preserves the full-gravity parabola shape
//! at any speed â€” that is what makes the ballistic solver's seed exact and
//! the steering loop converge on the landing point.

use bevy::prelude::Vec3;

use super::params::*;
use super::shot_type::ShotArg;

/// Live ball state.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BallState {
    pub pos: Vec3,
    pub vel: Vec3,
    /// Lateral curve accel magnitude at pace 1 (signed, + = ball's right).
    pub curve: f32,
    pub shot: ShotArg,
    pub charge: f32,
    /// Per-hit power scalar (charge/fatigue multiplier, normally 1). Flight
    /// forces scale by `pace²`, preserving the full-gravity arc shape — the
    /// property the ballistic solver's seed relies on.
    pub pace: f32,
    /// Bounces since the last strike (serve fault / rally tracking).
    pub bounces: usize,
}

impl BallState {
    pub fn new(pos: Vec3, vel: Vec3, shot: ShotArg, charge: f32) -> Self {
        let sign = shot.curve_sign();
        let curve = if sign == 0.0 {
            0.0
        } else {
            super::shot_type::curve_accel(shot, charge, vel.length()) * sign
        };
        Self {
            pos,
            vel,
            curve,
            shot,
            charge,
            pace: 1.0,
            bounces: 0,
        }
    }
}

/// Where and when the ball next touches the floor.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Landing {
    pub x: f32,
    pub z: f32,
    /// Seconds from the prediction start.
    pub w: f32,
}

/// Flight + collision model. Cheap to construct; holds only constants.
#[derive(Debug, Clone, Copy, Default)]
pub struct FlightModel;

impl FlightModel {
    /// Advance one physics tick of `dt` with adaptive substeps
    /// (`step â‰¤ 0.2/|v|`, v0.14 `reference_flight_v014_step_dt`).
    /// Returns `FlightEvent` for what happened this tick (last event wins).
    pub fn step(&self, ball: &mut BallState, dt: f32, serve: bool) -> FlightEvent {
        let mut event = FlightEvent::None;
        let speed = ball.vel.length().max(1e-4);
        let max_step: f32 = (0.2 / speed).min(dt);
        let steps = ((dt / max_step).ceil() as usize).max(1);
        let h = dt / steps as f32;
        for _ in 0..steps {
            let prev = ball.pos;
            self.integrate(ball, h);
            if let Some(e) = self.resolve_collisions(ball, prev, serve) {
                event = e;
                if matches!(event, FlightEvent::NetTape | FlightEvent::NetRebound) {
                    break;
                }
            }
        }
        event
    }

    /// One force-and-move substep. Pub(crate) for solver/parity tests.
    pub fn integrate(&self, ball: &mut BallState, h: f32) {
        let speed = ball.vel.length();
        let _ = speed;
        let pace = ball.pace.max(1e-4);
        let pace2 = pace * pace;
        // Curve accel along the team right axis (unit ±Z, the game's
        // NetRight(team) — verified against the NthLanding fixture), with
        // the slow 0.75 decay whenever a curve field is present.
        if ball.curve != 0.0 {
            let right = Vec3::new(0.0, 0.0, 1.0);
            ball.vel += right * (pace2 * ball.curve) * h;
            // Empirical (NthLanding fixture slice-team-one): any ball
            // carrying a curve field decays at the slow 0.75 rate — the
            // 2.4 rate only applies to curve-free flight.
            let decay = if ball.curve != 0.0 {
                CURVE_DECAY_CURVE_SHOTS
            } else {
                CURVE_DECAY_OTHER
            };
            ball.curve *= (1.0 - decay * pace * h).max(0.0);
        }
        ball.vel.y -= GRAVITY * pace2 * h;
        // Speed cap: MAX_SPEED * max(1, pace) â€” release caps a launch at 46,
        // accelerations may exceed only in proportion to the current pace.
        let cap = MAX_SPEED * pace.max(1.0);
        let s = ball.vel.length();
        if s > cap {
            ball.vel *= cap / s;
        }
        ball.pos += ball.vel * h;
    }

    /// Net plane + floor collisions for one substep.
    fn resolve_collisions(&self, ball: &mut BallState, prev: Vec3, serve: bool) -> Option<FlightEvent> {
        // Net plane crossing (strict sign change around X = 0).
        if prev.x * ball.pos.x < 0.0 {
            let fraction: f32 = ((0.0 - prev.x) / (ball.pos.x - prev.x)).clamp(0.0, 1.0);
            let at_net = prev.lerp(ball.pos, fraction);
            if at_net.y < NET_TAPE_HEIGHT && at_net.z.abs() <= NET_HALF_WIDTH {
                if serve {
                    // Serve tape: commit position, world scores the fault.
                    ball.pos = at_net;
                    return Some(FlightEvent::NetTape);
                }
                self.net_rebound(ball, at_net);
                return Some(FlightEvent::NetRebound);
            }
            // Above the tape or outside the posts: clean cross, no interaction.
        }

        // Floor contact.
        if ball.pos.y <= BOUNCE_FLOOR_Y && ball.vel.y < 0.0 {
            self.floor_bounce(ball);
            return Some(FlightEvent::Bounce);
        }
        None
    }

    /// v0.14 `ApplyNetRebound` (RVA 0xc18400): dead-cat bounce back off the net.
    fn net_rebound(&self, ball: &mut BallState, at_net: Vec3) {
        // Side the ball came FROM (vx sign inverted): the rebound is placed
        // and pushed back on that side.
        let from_side = -ball.vel.x.signum();
        ball.pos.x = from_side * (BALL_RADIUS + 0.08);
        ball.pos.y = at_net
            .y
            .clamp(COURT_Y + BALL_RADIUS + 0.05, NET_HEIGHT + COURT_Y + BALL_RADIUS);
        ball.pos.z = at_net.z;
        // vx = ∓max(2.4, |vx|*0.38): pushed back toward the side it came from.
        ball.vel.x = from_side * 2.4f32.max(ball.vel.x.abs() * 0.38);
        ball.vel.y = 2.2f32.max(ball.vel.y.abs() * 0.45 + 1.4);
        ball.vel.z *= 0.72;
        let s = ball.vel.length();
        if s > MAX_SPEED {
            ball.vel *= MAX_SPEED / s;
        }
    }

    /// v0.14 `ApplyFloorBounce` (RVA 0xc18270) with the pinned shot-specific
    /// pending flags consumed on the first bounce.
    fn floor_bounce(&self, ball: &mut BallState) {
        let first = ball.bounces == 0;
        ball.pos.y = BOUNCE_FLOOR_Y;
        let mut restitution = BOUNCE_RESTITUTION;
        let mut forward = BOUNCE_FORWARD_KEEP;
        if first {
            match ball.shot {
                ShotArg::Topspin => {
                    ball.vel.x *= TOPSPIN_FORWARD_KICK;
                    ball.vel.z *= TOPSPIN_FORWARD_KICK;
                    restitution *= TOPSPIN_BOUNCE_SCALE;
                }
                ShotArg::Slice => {
                    let q = ball.charge.clamp(0.0, 1.0);
                    let f0 = (SLICE_FORWARD_KEEP + 0.08).min(1.0);
                    forward = f0 + q * (SLICE_FORWARD_KEEP * SLICE_FORWARD_KEEP - f0);
                    let r0 = (SLICE_BOUNCE_SCALE + 0.10).min(1.0);
                    restitution = r0 + q * (SLICE_BOUNCE_SCALE * 1.4 - r0);
                }
                ShotArg::Drop => {
                    forward = DROP_FORWARD_KEEP;
                    restitution *= DROP_BOUNCE_SCALE;
                }
                _ => {}
            }
        }
        ball.curve = 0.0;
        let mut vy = ball.vel.y.abs() * restitution;
        if first {
            match ball.shot {
                ShotArg::Drop => vy = vy.clamp(DROP_HOP_MIN, DROP_HOP_MAX),
                ShotArg::Slice => vy = vy.max(BOUNCE_MIN_SPEED * 0.85),
                _ => vy = vy.max(BOUNCE_MIN_SPEED),
            }
        } else {
            vy = vy.max(BOUNCE_MIN_SPEED);
        }
        ball.vel.y = vy;
        ball.vel.x *= forward;
        ball.vel.z *= forward;
        if ball.vel.y < BOUNCE_SPIN_FLOOR {
            ball.vel.y = 0.0;
        }
        ball.bounces += 1;
    }

    /// Predict the `n`-th landing (0 = first) by forward simulation.
    /// Horizons: 6 s (first bounce) / 10 s (second), matching the recovered
    /// `TryPredictNthLandingFrom`. `None` = no landing within horizon.
    pub fn predict_landing(&self, ball: &mut BallState, n: usize) -> Option<Landing> {
        self.predict_landing_tape(ball, n, false)
    }

    /// Prediction with the tape policy: stop_on_tape (serves) fails at a
    /// below-tape net crossing; rally predictions pass through — the game's
    /// TryPredictNthLandingFrom with stopOnTape=false keeps integrating
    /// (fixture negative-curve-team-one crosses below tape and lands).
    pub fn predict_landing_tape(
        &self,
        ball: &mut BallState,
        n: usize,
        stop_on_tape: bool,
    ) -> Option<Landing> {
        let horizon = if n == 0 { 6.0 } else { 10.0 };
        let mut t = 0.0;
        let start_bounces = ball.bounces;
        while t < horizon {
            let speed = ball.vel.length().max(1e-4);
            let max_step: f32 = (0.2 / speed).min(FIXED_DT).min(0.05);
            let h = max_step;
            let prev = ball.pos;
            self.integrate(ball, h);
            t += h;
            if prev.x * ball.pos.x < 0.0 {
                let fraction: f32 = ((0.0 - prev.x) / (ball.pos.x - prev.x)).clamp(0.0, 1.0);
                let at_net = prev.lerp(ball.pos, fraction);
                if at_net.y < NET_TAPE_HEIGHT && at_net.z.abs() <= NET_HALF_WIDTH && stop_on_tape {
                    // Serve/tape prediction ends at the net: no landing.
                    return None;
                }
            }
            if ball.pos.y <= BOUNCE_FLOOR_Y && ball.vel.y < 0.0 {
                if ball.bounces - start_bounces == n {
                    let fraction = ((BOUNCE_FLOOR_Y - prev.y) / (ball.pos.y - prev.y)).clamp(0.0, 1.0);
                    let hit = prev.lerp(ball.pos, fraction);
                    return Some(Landing {
                        x: hit.x,
                        z: hit.z,
                        w: t,
                    });
                }
                self.floor_bounce(ball);
            }
        }
        None
    }
}

/// What the flight did this tick (last event within the tick wins).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlightEvent {
    None,
    /// Touched the net tape on a serve â†’ fault.
    NetTape,
    /// Hit the net body mid-rally â†’ rebound, ball live on the striker's side.
    NetRebound,
    /// First (or subsequent) floor contact.
    Bounce,
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::prelude::Vec2;

    #[test]
    fn pace_squared_scaling_preserves_parabola_shape() {
        // Full-speed arc and its half-speed twin must trace the same shape
        // (scaled in time) â€” that is the property the solver relies on.
        let flight = FlightModel;
        let mut full = BallState::new(
            Vec3::new(-13.0, 1.5, 0.0),
            Vec3::new(20.0, 6.0, 0.0),
            ShotArg::Flat,
            0.0,
        );
        let mut half = BallState::new(
            Vec3::new(-13.0, 1.5, 0.0),
            Vec3::new(10.0, 3.0, 0.0),
            ShotArg::Flat,
            0.0,
        );
        // Advance the full-speed ball 1 s of flight, the half-speed ball the
        // shape-equivalent 2 s in coarse steps.
        for _ in 0..52 {
            flight.integrate(&mut full, FIXED_DT);
        }
        for _ in 0..104 {
            flight.integrate(&mut half, FIXED_DT);
        }
        let a = Vec2::new(full.pos.x, full.pos.z);
        let b = Vec2::new(half.pos.x, half.pos.z);
        assert!(a.distance(b) < 0.6, "shape drifted: {a:?} vs {b:?}");
    }

    #[test]
    fn net_rebound_pushes_ball_back() {
        let flight = FlightModel;
        // Start just before the net plane so the crossing lands inside this
        // tick's substeps (20 m/s moves ~0.38 m per tick).
        let mut ball = BallState::new(
            Vec3::new(-0.05, 0.8, 0.0),
            Vec3::new(20.0, 0.0, 3.0),
            ShotArg::Flat,
            0.0,
        );
        let e = flight.step(&mut ball, FIXED_DT, false);
        assert_eq!(e, FlightEvent::NetRebound);
        assert!(ball.pos.x < 0.0, "ball must come back home-side");
        assert!(ball.vel.x < 0.0);
    }

    #[test]
    fn bounce_restores_upward_speed_per_shot_family() {
        let flight = FlightModel;
        // Drop shot lands softly but hops in its clamped band.
        let mut drop = BallState::new(
            Vec3::new(5.0, 1.0, 0.0),
            Vec3::new(3.0, -8.0, 0.0),
            ShotArg::Drop,
            0.0,
        );
        flight.floor_bounce(&mut drop);
        assert!((DROP_HOP_MIN..=DROP_HOP_MAX).contains(&drop.vel.y), "{}", drop.vel.y);
        assert!((drop.vel.x - 3.0 * DROP_FORWARD_KEEP).abs() < 1e-5);

        let mut flat = BallState::new(
            Vec3::new(5.0, 1.0, 0.0),
            Vec3::new(20.0, -8.0, 0.0),
            ShotArg::Flat,
            0.0,
        );
        flight.floor_bounce(&mut flat);
        assert!((flat.vel.y - (8.0 * BOUNCE_RESTITUTION).max(BOUNCE_MIN_SPEED)).abs() < 1e-5);
    }
}
