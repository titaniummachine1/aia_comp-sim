//! Ballistic shot solver â€” port of the v0.14 recovered
//! `TennisBall.ComputeShotVelocity` / `SteerVelocityToAim`
//! (`tennis-sim/native/recovered_shot_v014.hpp`) with the v0.12 steering
//! loop shape (`recovered_shot.hpp`).
//!
//! The game's contract: **aim at a destination and the ball lands there**.
//! The solve seeds a full-gravity parabola that touches the target, stretches
//! time until the arc clears the net plane, caps launch speed at 46 m/s, then
//! a steering loop re-predicts the landing with the real flight model and
//! corrects horizontal velocity until the predicted first bounce is on the
//! target.

use bevy::prelude::Vec3;

use super::ball::{BallState, FlightModel};
use super::params::*;
use super::shot_type::ShotType;

/// Solve the launch velocity for a strike from `from` (ball center at contact)
/// toward `target` (desired first-bounce XZ; target Y is the court plane).
///
/// `q` is the swing charge in [0, 1]. Returns velocity in m/s.
pub fn compute_shot_velocity(from: Vec3, target: Vec3, shot: ShotType, q: f32) -> Vec3 {
    let q = q.clamp(0.0, 1.0);
    let idx = shot as usize;
    let target_xz = Vec3::new(target.x, 0.0, target.z);
    let from_xz = Vec3::new(from.x, 0.0, from.z);
    let mut flat = target_xz - from_xz;
    let mut distance = flat.length();
    if distance < 1e-4 {
        // Straight up-ish degenerate aim: drop at the feet.
        flat = Vec3::ZERO;
        distance = 0.0;
    } else {
        flat /= distance;
    }

    // Charge â†’ speed multiplier (v0.14 min/max charge power).
    let high = if shot_has_full_charge(shot) {
        1.3
    } else {
        CHARGE_POWER_MIN + CHARGE_OTHER_FRACTION * (CHARGE_POWER_MAX - CHARGE_POWER_MIN)
    };
    let power = CHARGE_POWER_MIN + (high - CHARGE_POWER_MIN) * q;
    let speed = power * SHOT_SPEEDS[idx];

    // Target height: net-top plane plus per-shot margin (v0.12 recovered
    // additions; the v0.14 solver keeps the same structure).
    let lift = SHOT_LIFTS[idx];
    let target_y = NET_HEIGHT + COURT_Y + BALL_RADIUS
        + match shot {
            ShotType::Flat => 0.06,
            ShotType::Lob => 0.28 + lift * (1.0 + 0.18 * q),
            ShotType::Drop => 0.16 + lift * (1.0 + 0.12 * q),
            _ => 0.12,
        };

    let mut t = SHOT_MIN_TIMES[idx].max(distance / speed.max(1.0));

    // Net-clearance constraint: stretch flight time so the parabola clears
    // the tape plane where it crosses X = 0.
    if from.x * target.x < 0.0 && flat.x.abs() > 1e-6 {
        let fraction = (from.x.abs() / (flat.x.abs() * distance)).clamp(0.0, 1.0);
        if (0.02..0.98).contains(&fraction) {
            let clearance = match shot {
                ShotType::Flat => (NET_HEIGHT - 0.03) + BALL_RADIUS + 0.06,
                ShotType::Lob => {
                    (NET_HEIGHT - 0.03) + BALL_RADIUS + 0.28 + lift * (1.0 + 0.18 * q)
                }
                ShotType::Drop => {
                    (NET_HEIGHT - 0.03) + BALL_RADIUS + 0.16 + lift * (1.0 + 0.12 * q)
                }
                _ => (NET_HEIGHT - 0.03) + BALL_RADIUS + 0.12,
            };
            // Linear height the seed parabola has at the net plane.
            let at_net = from.y + (target_y - from.y) * fraction;
            let rise = (clearance + 0.08) - at_net;
            // coefficient = 0.5*g*fraction*(1-fraction) > 0 inside the net
            // span; rise > 0 means the seed arc must be stretched to clear.
            let coefficient = GRAVITY * 0.5 * fraction * (1.0 - fraction);
            if rise > 0.0 && coefficient > 1e-5 {
                t = t.max((rise / coefficient).sqrt());
            }
        }
    }

    // Per-shot hang-time floors (v0.14).
    match shot {
        ShotType::Lob => {
            t = t.max((0.82 + 0.14 * q + lift * 0.14).max(0.8 + 0.08 * q + 0.08));
        }
        ShotType::Drop => {
            t = t.max(0.63 + 0.13 * q + lift * 0.13);
        }
        ShotType::Slice => {
            t = t.max(0.5 + 0.12 * q + lift * 0.08);
        }
        _ => {}
    }

    // Vertical solve on the full-gravity parabola: land on the court plane.
    let floor_y = BOUNCE_FLOOR_Y;
    // Solver convention: gravity = -28 (C++ z-up). vy comes out UPWARD.
    let mut vy = (floor_y - from.y) / t + 0.5 * GRAVITY * t;

    // Launch speed cap loop (â‰¤12 raises of 0.05 s, bounded by max(2.4, t+0.35)).
    let t_cap = (2.4f32).max(t + 0.35);
    let mut iterations = 0;
    let mut vxz = flat * distance / t.max(1e-4);
    loop {
        let total = (vxz.length().hypot(vy)).max(0.0);
        if total <= MAX_SPEED || iterations >= 12 || t >= t_cap {
            break;
        }
        t += 0.05;
        vy = (floor_y - from.y) / t + 0.5 * GRAVITY * t;
        vxz = flat * distance / t.max(1e-4);
        iterations += 1;
    }

    let mut vel = Vec3::new(vxz.x, vy, vxz.z);

    // Curve shots carry their lateral force inside the flight model; the
    // steering loop below polishes the landing against it, so no pre-bias is
    // applied here (keeps the seed a pure ballistic solve).
    vel
}

/// Steer the launch velocity until the simulated first bounce lands on
/// `target` (v0.12 â‰¤20 iterations, exit when squared error â‰¤ 0.0144).
pub fn steer_velocity_to_aim(
    from: Vec3,
    target: Vec3,
    shot: ShotType,
    q: f32,
    vel: Vec3,
    flight: &FlightModel,
) -> Vec3 {
    let mut vel = vel;
    let target_xz = Vec2::new(target.x, target.z);
    let mut best = vel;
    let mut best_err = f32::INFINITY;
    for _ in 0..20 {
        let mut probe = BallState::new(from, vel, shot, q);
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
pub fn solve_shot(
    from: Vec3,
    target: Vec3,
    shot: ShotType,
    q: f32,
    flight: &FlightModel,
) -> Vec3 {
    let seed = compute_shot_velocity(from, target, shot, q);
    steer_velocity_to_aim(from, target, shot, q, seed, flight)
}

use bevy::prelude::Vec2;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tennis::params::*;

    /// The game's core promise: aim at a destination and the first bounce
    /// lands there. Grid over shot types, targets and charge.
    #[test]
    fn ballistic_solve_lands_on_target_across_shot_types() {
        let flight = FlightModel;
        let from = Vec3::new(-13.0, PLAYER_GROUND_Y + STRIKE_HEIGHT, 2.0);
        for shot in [
            ShotType::Flat,
            ShotType::Topspin,
            ShotType::Slice,
            ShotType::Lob,
            ShotType::Drop,
        ] {
            for target in [
                Vec3::new(11.0, BOUNCE_FLOOR_Y, -4.0),
                Vec3::new(6.0, BOUNCE_FLOOR_Y, 4.0),
                Vec3::new(2.5, BOUNCE_FLOOR_Y, 0.0),
            ] {
                for q in [0.0f32, 0.5, 1.0] {
                    let vel = solve_shot(from, target, shot, q, &flight);
                    let mut probe = BallState::new(from, vel, shot, q);
                    let landing = flight
                        .predict_landing(&mut probe, 0)
                        .unwrap_or_else(|| panic!("{shot:?} q{q}: no landing"));
                    let err =
                        ((landing.x - target.x).powi(2) + (landing.z - target.z).powi(2)).sqrt();
                    assert!(
                        err <= 0.12,
                        "{shot:?} q{q} target {:?}: landed ({:.2},{:.2}) err {:.3}m",
                        Vec2::new(target.x, target.z),
                        landing.x,
                        landing.z,
                        err
                    );
                }
            }
        }
    }

    /// Cross-court solves must clear the tape plane at the net crossing.
    #[test]
    fn solve_clears_the_net_for_cross_court_targets() {
        let flight = FlightModel;
        let from = Vec3::new(-13.0, PLAYER_GROUND_Y + STRIKE_HEIGHT, -3.0);
        for shot in [ShotType::Flat, ShotType::Topspin, ShotType::Slice] {
            let target = Vec3::new(12.0, BOUNCE_FLOOR_Y, 5.0);
            let vel = solve_shot(from, target, shot, 0.6, &flight);
            let mut probe = BallState::new(from, vel, shot, 0.6);
            // Height where the trajectory crosses X = 0.
            let t_cross = (0.0 - from.x) / vel.x;
            let y_cross = from.y + vel.y * t_cross - 0.5 * GRAVITY * t_cross * t_cross;
            // pace² gravity scaling stretches the arc; the true flight is the
            // reference — assert against the actual integrator.
            let _ = y_cross;
            let mut crossed_y = 0.0f32;
            for _ in 0..600 {
                let prev = probe.pos;
                flight.integrate(&mut probe, FIXED_DT);
                if prev.x * probe.pos.x < 0.0 {
                    let f = (0.0 - prev.x) / (probe.pos.x - prev.x);
                    crossed_y = prev.lerp(probe.pos, f).y;
                    break;
                }
            }
            assert!(
                crossed_y >= NET_TAPE_HEIGHT,
                "{shot:?}: net crossing at {crossed_y:.3} < tape {NET_TAPE_HEIGHT}"
            );
        }
    }

    /// Launch speed never exceeds the game cap (46 m/s).
    #[test]
    fn launch_respects_speed_cap() {
        let flight = FlightModel;
        let from = Vec3::new(-13.5, PLAYER_GROUND_Y + STRIKE_HEIGHT, 0.0);
        let target = Vec3::new(13.5, BOUNCE_FLOOR_Y, 0.0);
        let vel = solve_shot(from, target, ShotType::Flat, 1.0, &flight);
        assert!(vel.length() <= MAX_SPEED + 1e-3, "{}", vel.length());
    }
}
