//! Ballistic shot solver — reverse-engineered from the game's own
//! `TennisBall.ComputeShotVelocity` and verified against the live-captured
//! 168-case matrix (`tests/fixtures/tennis-v014/shot_solver.jsonl`, mined by
//! `modhost/capture_fixtures.py`).
//!
//! Discovered v0.14 structure (empirical, per game ARG order):
//!
//! | arg | family    | speed base | lift  | flight time                      |
//! |-----|-----------|-----------|-------|----------------------------------|
//! | 0   | Topspin   | 24        | 0.08  | max(minT, d/speed)               |
//! | 1   | Slice     | 16        | 0.04  | fixed tail 0.5+0.12q+lift*0.08   |
//! | 2   | Flat      | 28        | 0.02  | max(minT, d/speed)               |
//! | 3   | Lob       | 12        | 6.0   | floor 0.82+0.14q+lift*0.14       |
//! | 4   | Drop      | 7.6       | 0.27  | floor 0.63+0.13q+lift*0.13       |
//! | 5/6 | CurveL/R  | 24        | 0.08  | max(minT, d/speed)               |
//!
//! Charge: speed factor `0.85 + (high-0.85)*q` with high=1.3 for the
//! straight family (0,2) and `0.85+0.42*0.45` otherwise.
//!
//! **Deliberately no net-clearance stretch for the straight family (0,2,5,6)**
//! — confirmed against the game and by play: those shots fire on their
//! ballistic arc and CAN hit the net (a "straight" shot has no line to the
//! destination that avoids it). Lob handles net clearance via
//! EnforceLobLoft; the slice's crossed-net delta (+~0.05 s) is an open
//! fit tracked by the replay test.
//!
//! Vertical solve: `vy = (target_y - from.y)/t + 0.5*g*t` (g = 28 downward,
//! z-up solver convention) with the target on the bounce floor plane — this
//! is the ballistic "lands exactly on the aim" contract. Launch speed is
//! capped at 46 m/s by raising t.

use bevy::prelude::Vec2;
use bevy::prelude::Vec3;

use super::ball::{BallState, FlightModel};
use super::params::*;
use super::shot_type::ShotArg;

/// Solve the launch velocity for a strike from `from` (ball center at
/// contact) toward `target` (desired first-bounce XZ on the floor plane).
/// `arg` is the game's physical shot argument; `q` the swing charge [0,1].
/// `incoming_dir` is the pre-hit velocity direction — the game falls back
/// to it (at full charged speed, straight family) when the aim is
/// degenerate (~zero distance); the fixture "fallback" rows pin this.
pub fn compute_shot_velocity_arg(
    from: Vec3,
    target: Vec3,
    arg: ShotArg,
    q: f32,
    incoming_dir: Option<Vec2>,
) -> Vec3 {
    let q = q.clamp(0.0, 1.0);
    let (base, _lift) = arg.table();
    let from_xz = Vec2::new(from.x, from.z);
    let target_xz = Vec2::new(target.x, target.z);
    let flat = target_xz - from_xz;
    let distance = flat.length();

    // Charge -> speed factor (full range only for the straight family).
    let high = if arg.full_charge() { 1.3 } else { 0.85 + CHARGE_OTHER_FRACTION * 0.45 };
    let factor = CHARGE_POWER_MIN + (high - CHARGE_POWER_MIN) * q;
    let speed = factor * base;

    // Degenerate aim: fire along the incoming direction at charged speed
    // (game fallback rows; no ballistic solve happens).
    if distance < 1e-3 {
        if let Some(dir3) = incoming_dir {
            let dir = dir3.normalize_or_zero();
            return Vec3::new(dir.x * speed, 0.0, dir.y * speed);
        }
    }
    let dir = if distance > 1e-4 { flat / distance } else { Vec2::X };

    // Flight time per family (empirical v0.14 formulas).
    let mut t = match arg {
        ShotArg::Slice => 0.5 + 0.12 * q + lift * 0.08,
        ShotArg::Lob => (0.82 + 0.14 * q + lift * 0.14).max(0.8 + 0.08 * q + 0.08),
        ShotArg::Drop => 0.63 + 0.13 * q + lift * 0.13,
        _ => SHOT_MIN_TIMES[arg as usize].max(distance / speed.max(1e-4)),
    };

    // Launch-speed cap loop: raise t (≤12 × 0.05 s) until ≤ 46 m/s.
    let vy_for = |t: f32| (BOUNCE_FLOOR_Y - from.y) / t + 0.5 * GRAVITY * t;
    let vxz_for = |t: f32| dir * (distance / t.max(1e-4));
    let t_cap = 2.4f32.max(t + 0.35);
    let mut iterations = 0;
    loop {
        let vxz = vxz_for(t);
        let vy = vy_for(t);
        let total = (vxz.x * vxz.x + vxz.y * vxz.y + vy * vy).sqrt();
        if total <= MAX_SPEED || iterations >= 12 || t >= t_cap {
            break;
        }
        t += 0.05;
        iterations += 1;
    }

    let vxz = vxz_for(t);
    Vec3::new(vxz.x, vy_for(t), vxz.y)
}

/// Steer the launch velocity until the simulated first bounce lands on
/// `target` (v0.12 ≤20 iterations, exit when squared error ≤ 0.0144).
pub fn steer_velocity_to_aim(
    from: Vec3,
    target: Vec3,
    arg: ShotArg,
    q: f32,
    vel: Vec3,
    flight: &FlightModel,
) -> Vec3 {
    let mut vel = vel;
    let target_xz = Vec2::new(target.x, target.z);
    let mut best = vel;
    let mut best_err = f32::INFINITY;
    for _ in 0..20 {
        let mut probe = BallState::new(from, vel, arg, q);
        let Some(landing) = flight.predict_landing(&mut probe, 0) else {
            // Failed prediction: add loft and retry (game: vel.y += 1.15).
            vel.y += 1.15;
            best = vel;
            continue;
        };
        let err = Vec2::new(landing.x - target_xz.x, landing.z - target_xz.y);
        let err2 = err.length_squared();
        if err2 < best_err {
            best_err = err2;
            best = vel;
        }
        if err2 <= 0.0144 {
            break;
        }
        let t_corr = landing.w.max(0.12);
        vel.x -= err.x / t_corr;
        vel.z -= err.y / t_corr;
    }
    best
}

/// Convenience: solve + steer in one call.
pub fn solve_shot(from: Vec3, target: Vec3, arg: ShotArg, q: f32, flight: &FlightModel) -> Vec3 {
    let seed = compute_shot_velocity_arg(from, target, arg, q, None);
    steer_velocity_to_aim(from, target, arg, q, seed, flight)
}
