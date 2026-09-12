//! Tennis measured constants — mirrored from the C++ sim's v0.14
//! word-exact recovered helpers (`tennis-sim/native/pinned_scene_v014.hpp`,
//! `recovered_shot_v014.hpp`, `recovered_ball_v014.hpp`,
//! `recovered_court_v014.hpp`) and the v0.12 model config (`core.hpp`).
//!
//! Pinned values are locked: `params::measured_constants_tests` asserts them.
//! If a test fails, re-measure against the game — do not change the expected
//! value (same rule as soccer's `params::measured_constants_tests`).

/// Physics tick. Captured game word 1016833504 = 0.0189999938 s (~52.63 Hz).
pub const FIXED_DT: f32 = 0.018_999_994;

/// Downward gravity, m/s² (arcade value used by the shot solver).
pub const GRAVITY: f32 = 28.0;

/// Ball launch/flight speed cap, m/s (`reference_flight_v014_cap_launch_speed`).
pub const MAX_SPEED: f32 = 46.0;

/// Court: length along X (net plane at X = 0, home at negative X), singles
/// width along Z, net at the middle. Y is height. Units are meters.
pub const COURT_LENGTH: f32 = 28.0;
pub const COURT_SINGLES_WIDTH: f32 = 12.0;
pub const COURT_DOUBLES_WIDTH: f32 = 18.0;
pub const NET_HEIGHT: f32 = 0.95;
pub const COURT_Y: f32 = 0.0;
pub const LINE_WIDTH: f32 = 0.15;
/// Ball scale 0.62 → radius 0.31 (pinned word 1050589265/1058977874 pair).
pub const BALL_RADIUS: f32 = 0.31;
/// Extra out-of-bounds margin around the court for "in" tests.
pub const OUT_MARGIN: f32 = 0.45;

/// Ball out radius used by IsInCourtXZ (ball_scale*0.55*0.5*0.98828125).
pub const BALL_OUT_RADIUS: f32 = 0.62 * 0.55 * 0.5 * 0.988_281_25;

/// Half the net is playable width plus the post margin
/// (`doubles_width*0.5 + 0.25` = 9.25, v0.14 TryCrossNetPlane).
pub const NET_HALF_WIDTH: f32 = COURT_DOUBLES_WIDTH * 0.5 + 0.25;
/// Tape plane height: net top plus 35% of the ball radius
/// (`net_height + radius * 0.35` = 1.0585).
pub const NET_TAPE_HEIGHT: f32 = NET_HEIGHT + BALL_RADIUS * 0.35;

/// Floor contact plane: `courtY + radius` = 0.31 (v0.14 bounce floor).
pub const BOUNCE_FLOOR_Y: f32 = COURT_Y + BALL_RADIUS;

// --- Players (v0.12 model config, core.hpp Config) ---
pub const WALK_SPEED: f32 = 8.5;
pub const SPRINT_SPEED: f32 = 13.0;
/// Ball reachable within this radius of the racket center.
pub const STRIKE_RADIUS: f32 = 2.6;
pub const STRIKE_HEIGHT: f32 = 1.25;
pub const RACKET_FORWARD: f32 = 0.55;
pub const RACKET_SIDE: f32 = 0.45;
/// Charge quality tiers around the strike point (aia3 uses exactly 1.0 for the
/// perfect-contact radius).
pub const PERFECT_RADIUS: f32 = 1.0;
pub const GOOD_RADIUS: f32 = 1.85;
/// Swing charge: seconds to full windup, then the hit window.
pub const CHARGE_WINDUP: f32 = 0.5;
pub const CHARGE_WINDOW: f32 = 0.4;
pub const SWING_SECONDS: f32 = 0.2;
pub const RECOVER_SECONDS: f32 = 0.16;

// --- Stamina (game `Stamina.OnSimulationTick`, measured from the native
// timeplot `v44_Stamina`, seed-10 match) ---
/// Sprinting drains this much stamina per tick (measured: -0.010/tick runs of
/// 3-21 ticks = 118 drain ticks over the match, 1.00 -> 0.10).
pub const STAMINA_SPRINT_DRAIN: f32 = 0.01;
/// Standing still regenerates this much per tick (measured: +0.0009..+0.0010
/// runs, 345 regen ticks; long rallies show flat plateaus = walking neutral).
pub const STAMINA_REGEN: f32 = 0.001;

/// Player root ground Y (pinned word 1034595072 = 0.08333397).
pub const PLAYER_GROUND_Y: f32 = 0.083_333_97;
/// Serve toss release height above `PLAYER_GROUND_Y` (game-measured: the held
/// ball bobs 1.02-1.44 m absolute and the toss launches at the bob top,
/// settled v0.14 timeplot 2026-09-12).
pub const TOSS_RELEASE_HEIGHT: f32 = 1.36;
/// Absolute height the tossed ball reaches (game-measured flight/toss apex
/// 3.72 m; the launch speed is solved to reach it).
pub const TOSS_APEX_Y: f32 = 3.72;
/// The game strikes the RISING toss at ~3.5 m (not on descent): tick ~86,
/// ball 3.5-3.6 m, serve flight then peaks 3.72 into the box.
pub const TOSS_STRIKE_Y: f32 = 3.45;
/// Toss launch speed floor (game rise 1.44 -> 3.5 m in ~26 ticks => ~11.2;
/// the ballistic solve for the measured apex gives ~11.8, so this floor is
/// not binding any more).
pub const TOSS_MIN_SPEED: f32 = 11.5;
/// Serve contact height above the player root.
pub const SERVE_CONTACT_Y: f32 = 3.2;

// --- Bounce (v0.14 BallV014BounceConfig, pinned words) ---
pub const BOUNCE_RESTITUTION: f32 = 0.78;
pub const BOUNCE_FORWARD_KEEP: f32 = 0.94;
pub const BOUNCE_MIN_SPEED: f32 = 2.4;
pub const TOPSPIN_FORWARD_KICK: f32 = 1.2;
pub const TOPSPIN_BOUNCE_SCALE: f32 = 1.42;
pub const SLICE_FORWARD_KEEP: f32 = 0.82;
pub const SLICE_BOUNCE_SCALE: f32 = 0.72;
pub const DROP_FORWARD_KEEP: f32 = 0.40;
pub const DROP_BOUNCE_SCALE: f32 = 1.08;
pub const DROP_HOP_MIN: f32 = 9.6;
pub const DROP_HOP_MAX: f32 = 11.8;
/// Spin floor: below this upward speed the ball rolls dead.
pub const BOUNCE_SPIN_FLOOR: f32 = 0.45;

// --- Shot solver charge words (v0.14 pinned) ---
pub const CHARGE_POWER_MIN: f32 = 0.85;
pub const CHARGE_POWER_MAX: f32 = 1.0;
/// Non-straight shots get a reduced high-end charge multiplier
/// (`0.85 + 0.42*0.45` = 1.039, verified at q=0/0.5/1 in the fixture).
pub const CHARGE_OTHER_FRACTION: f32 = 0.42;

/// Minimum flight times per game arg (straight family only).
pub const SHOT_MIN_TIMES: [f32; 7] = [0.14, 0.18, 0.12, 0.40, 0.38, 0.14, 0.14];

/// Lateral curve acceleration magnitude at pace 1, per shot
/// (`reference_curve`: curveL/R = ±(1.3+1.4q)*12.4; drop/lob (0.45+0.4q)*5.2;
/// slice (0.85+0.55q)*|v|·k — see shot.rs for per-type handling).
pub const CURVE_BASE_CURVE_SHOTS: f32 = 12.4;
pub const CURVE_BASE_SOFT_SHOTS: f32 = 5.2;
/// Curve decay rate per second: curve shots decay slower (0.75 vs 2.4).
pub const CURVE_DECAY_CURVE_SHOTS: f32 = 0.75;
pub const CURVE_DECAY_OTHER: f32 = 2.4;

// --- Fatigue (v0.12 model constants; the anti-stalemate mechanism) ---
pub const FATIGUE_LATERAL: f32 = 0.03;
pub const FATIGUE_DEPTH: f32 = 0.012;
pub const FATIGUE_POWER_LOSS: f32 = 0.025;
pub const FATIGUE_RALLY_GRACE: i32 = 12;
pub const FATIGUE_DEUCE_GRACE: i32 = 2;
/// Minimum charge multiplier under fatigue (power floors at 55%).
pub const FATIGUE_POWER_FLOOR: f32 = 0.55;

/// `reference_fatigue_points`: active fatigue points for this hit.
pub fn fatigue_points(rally_shots: i32, extra_deuce: i32) -> i32 {
    max0(extra_deuce - FATIGUE_DEUCE_GRACE.max(0))
        + max0(rally_shots - FATIGUE_RALLY_GRACE.max(1) + 1)
}

/// Deuce-side component alone (for the `Deuce Fatigue` sensor).
pub fn deuce_fatigue_points(extra_deuce: i32) -> i32 {
    max0(extra_deuce - FATIGUE_DEUCE_GRACE.max(0))
}

/// Rally-side component alone (for the `Rally Fatigue` sensor).
pub fn rally_fatigue_points(rally_shots: i32) -> i32 {
    max0(rally_shots - FATIGUE_RALLY_GRACE.max(1) + 1)
}

fn max0(v: i32) -> i32 {
    if v < 0 {
        0
    } else {
        v
    }
}

/// `reference_fatigue` power term: charge multiplier under `active` fatigue.
pub fn fatigued_charge(q: f32, active: i32) -> f32 {
    if active <= 0 {
        q
    } else {
        (q * (1.0 - FATIGUE_POWER_LOSS * active as f32).max(FATIGUE_POWER_FLOOR)).clamp(0.0, 1.0)
    }
}

// --- Clocks ---
/// Between-point hold before the next serve setup (v0.12 point_pause).
pub const POINT_PAUSE: f32 = 1.15;
/// Serve-setup settle before the toss launches (game-measured: ball placed at
/// the server -> strike = 64 ticks; settle 51 + ~13 rise ticks = 64).
pub const SETUP_SETTLE: f32 = 1.02;
/// Serve clock: the server must get the serve under way within this window.
/// On expiry the game **awards the serve to the opponent** (a refusal /
/// delay-of-game forfeit of the serve for the rest of the game) — simulated by
/// `Score::award_serve` in `TennisWorld::on_serve_deadline`.
///
/// Value: the game shows a 5 s `<ServeCountdownDisplay>` before the switch, so
/// the visible countdown is 5 s. Whether the internal `serveDeadlineTick` starts
/// 5 s or (say) 15 s before expiry is still unverified — read the field from the
/// probe (`metadata_probe.h` already lists `serveDeadlineTick`) before pinning
/// more. Scope (this game vs the whole set) is likewise unconfirmed; the sim
/// models **this game** (rotation resumes when the game ends).
pub const SERVE_CLOCK: f32 = 5.0;

#[cfg(test)]
mod measured_constants_tests {
    use super::*;

    #[test]
    fn court_is_pinned() {
        assert_eq!(COURT_LENGTH, 28.0);
        assert_eq!(COURT_SINGLES_WIDTH, 12.0);
        assert_eq!(COURT_DOUBLES_WIDTH, 18.0);
        assert_eq!(NET_HEIGHT, 0.95);
        assert_eq!(LINE_WIDTH, 0.15);
        assert_eq!(BALL_RADIUS, 0.31);
        assert_eq!(OUT_MARGIN, 0.45);
    }

    #[test]
    fn f32_words_round_trip() {
        // Raw words from tests/fixtures/v014-pinned-scene-context.jsonl.
        assert_eq!(COURT_LENGTH.to_bits(), 1105199104);
        assert_eq!(COURT_SINGLES_WIDTH.to_bits(), 1094713344);
        assert_eq!(COURT_DOUBLES_WIDTH.to_bits(), 1099956224);
        assert_eq!(NET_HEIGHT.to_bits(), 1064514355);
        assert_eq!(LINE_WIDTH.to_bits(), 1041865114);
        assert_eq!(BALL_RADIUS.to_bits(), 1050589266);
        assert_eq!(PLAYER_GROUND_Y.to_bits(), 1034595072);
    }

    #[test]
    fn derived_planes_are_pinned() {
        assert!((NET_TAPE_HEIGHT - 1.0585).abs() < 1e-6);
        assert!((NET_HALF_WIDTH - 9.25).abs() < 1e-6);
        assert!((BOUNCE_FLOOR_Y - 0.31).abs() < 1e-6);
    }

    #[test]
    fn bounce_config_is_pinned() {
        assert_eq!(BOUNCE_RESTITUTION, 0.78);
        assert_eq!(BOUNCE_FORWARD_KEEP, 0.94);
        assert_eq!(BOUNCE_MIN_SPEED, 2.4);
        assert_eq!(TOPSPIN_FORWARD_KICK, 1.2);
        assert_eq!(TOPSPIN_BOUNCE_SCALE, 1.42);
        assert_eq!(SLICE_FORWARD_KEEP, 0.82);
        assert_eq!(SLICE_BOUNCE_SCALE, 0.72);
        assert_eq!(DROP_FORWARD_KEEP, 0.40);
        assert_eq!(DROP_BOUNCE_SCALE, 1.08);
        assert_eq!(DROP_HOP_MIN, 9.6);
        assert_eq!(DROP_HOP_MAX, 11.8);
    }

    #[test]
    fn shot_table_is_pinned() {
        // (speed base, lift) per game ARG, verified against the live capture.
        use super::super::shot_type::ShotArg;
        let expect = [
            (ShotArg::Topspin, 24.0, 0.08),
            (ShotArg::Slice, 16.0, 0.04),
            (ShotArg::Flat, 28.0, 0.02),
            (ShotArg::Lob, 12.0, 6.0),
            (ShotArg::Drop, 7.6, 0.27),
            (ShotArg::CurveLeft, 24.0, 0.08),
            (ShotArg::CurveRight, 24.0, 0.08),
        ];
        for (arg, s, l) in expect {
            assert_eq!(arg.table(), (s, l), "{arg:?}");
        }
    }

    #[test]
    fn clocks_are_pinned() {
        assert!((FIXED_DT - 0.019).abs() < 1e-5);
        assert_eq!(GRAVITY, 28.0);
        assert_eq!(MAX_SPEED, 46.0);
        assert!((POINT_PAUSE - 1.15).abs() < 1e-6);
        assert_eq!(SERVE_CLOCK, 5.0);
    }
}

#[cfg(test)]
mod fatigue_tests {
    use super::*;

    #[test]
    fn rally_component_grace_then_linear() {
        assert_eq!(rally_fatigue_points(0), 0);
        assert_eq!(rally_fatigue_points(11), 0);
        assert_eq!(rally_fatigue_points(12), 1);
        assert_eq!(rally_fatigue_points(24), 13);
    }

    #[test]
    fn deuce_component_grace_then_linear() {
        assert_eq!(deuce_fatigue_points(0), 0);
        assert_eq!(deuce_fatigue_points(2), 0);
        assert_eq!(deuce_fatigue_points(3), 1);
        assert_eq!(deuce_fatigue_points(6), 4);
    }

    #[test]
    fn active_points_sum_components() {
        assert_eq!(fatigue_points(0, 0), 0);
        assert_eq!(fatigue_points(12, 3), 2);
        assert_eq!(fatigue_points(24, 6), 17);
    }

    #[test]
    fn charge_multiplier_floors_at_55_pct() {
        assert!((fatigued_charge(0.8, 0) - 0.8).abs() < 1e-6);
        assert!((fatigued_charge(1.0, 1) as f64 - 0.975).abs() < 1e-6);
        assert_eq!(fatigued_charge(1.0, 18), FATIGUE_POWER_FLOOR);
        assert!((fatigued_charge(0.5, 18) as f64 - 0.5 * 0.55).abs() < 1e-6);
        assert!((fatigued_charge(0.3, 3) as f64 - 0.2775).abs() < 1e-6);
    }
}
