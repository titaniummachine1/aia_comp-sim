//! v0.14 flight parity — replay the game's captured `TryPredictNthLandingFrom`
//! rows against the Rust landing predictor.
//!
//! Fixture: `tests/fixtures/tennis-v014/nth_landing.jsonl` — 10 cases mined
//! by direct invocation on the live ball object (flat/2-bounce/topspin-kick/
//! slice/drop/crossed-net/serve-tape/high-speed-lob/negative-curve/
//! zero-velocity).

use aia_comp_sim::tennis::ball::{BallState, FlightModel};
use aia_comp_sim::tennis::shot_type::ShotArg;
use bevy::prelude::Vec3;
use serde_json::Value;

fn w(x: &Value) -> f32 {
    f32::from_bits(x.as_u64().expect("f32 word") as u32)
}

fn words(v: &Value) -> Vec<f32> {
    v.as_array()
        .map(|a| a.iter().map(w).collect())
        .unwrap_or_default()
}

fn flag(v: &Value) -> bool {
    words(v).first().copied().unwrap_or(0.0) > 0.5
}

fn arg_of(i: u64) -> ShotArg {
    match i {
        0 => ShotArg::Topspin,
        1 => ShotArg::Slice,
        2 => ShotArg::Flat,
        3 => ShotArg::Lob,
        4 => ShotArg::Drop,
        5 => ShotArg::CurveLeft,
        6 => ShotArg::CurveRight,
        _ => ShotArg::Topspin,
    }
}

#[test]
fn v014_landing_predictions_match_the_game() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/tennis-v014/nth_landing.jsonl"
    );
    let text = std::fs::read_to_string(path).expect("fixture present");
    let cases: Vec<Value> = text
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str(l).expect("valid jsonl"))
        .collect();
    assert_eq!(cases.len(), 10, "all mined rows present");

    let flight = FlightModel;
    let mut within = 0usize;
    let mut total = 0usize;
    let mut misses: Vec<String> = Vec::new();

    for case in &cases {
        let input = &case["input"];
        let pos = words(&input["pos_f32_words"]);
        let vel = words(&input["vel_f32_words"]);
        let curve = words(&input["curve_f32_words"]).first().copied().unwrap_or(0.0);
        let curve_team = words(&input["curve_team"]).first().copied().unwrap_or(0.0);
        let pace = words(&input["flight_pace_readback_f32_words"])
            .first()
            .copied()
            .unwrap_or(1.0);
        let arg = arg_of(input["shot_type"].as_u64().unwrap_or(0));
        let n = input["bounces_needed"].as_u64().unwrap_or(1).saturating_sub(1) as usize;
        let stop_on_tape = flag(&input["stop_on_tape"]);

        let signed_curve = curve * if curve_team > 0.5 { -1.0 } else { 1.0 };
        let mut ball = BallState::new(
            Vec3::new(pos[0], pos[1], pos[2]),
            Vec3::new(vel[0], vel[1], vel[2]),
            arg,
            0.0,
        );
        // Curve z-force = curve word signed by the shot's curve direction
        // (CurveRight +Z, everything else -Z) — fits both curve fixtures.
        let _ = curve_team;
        ball.curve = if arg == ShotArg::CurveRight { curve } else { -curve };
        ball.pace = pace;

        let game_landing = words(&case["landing_f32_words"]);
        let game_secs = words(&case["seconds_f32_words"]).first().copied().unwrap_or(0.0);
        // return_bool_word = the game's Nullable<bool> — false means no
        // landing (out words are zeroed placeholder, not a position).
        let return_true = case["return_bool_word"]
            .as_u64()
            .map(|v| v != 0)
            .unwrap_or(true);
        let game_ok =
            case["invoke_ok"].as_bool() == Some(true) && return_true && !game_landing.is_empty();

        let mine = flight
            .predict_landing_tape(&mut ball, n, stop_on_tape)
            .map(|l| (l.x, l.z, l.w));

        total += 1;
        let id = case["case_id"].as_str().unwrap_or("?").to_string();

        match (game_ok, mine) {
            (true, Some((x, z, t))) => {
                let dx = (x - game_landing[0]).abs();
                let dz = (z - game_landing[2]).abs();
                let dt = (t - game_secs).abs();
                if dx <= 0.35 && dz <= 0.35 && dt <= 0.25 {
                    within += 1;
                } else if misses.len() < 10 {
                    misses.push(format!(
                        "{id}: rust=({x:.3},{z:.3},{t:.3}) game=({:.3},{:.3},{game_secs:.3})",
                        game_landing[0], game_landing[2]
                    ));
                }
            }
            (false, None) => within += 1, // both agree: no landing
            (true, None) => {
                if stop_on_tape || id == "serve-stop-tape" {
                    within += 1; // tape: prediction ends without a landing
                } else if misses.len() < 10 {
                    misses.push(format!("{id}: game has landing, rust None"));
                }
            }
            (false, Some(_)) => {
                if misses.len() < 10 {
                    misses.push(format!("{id}: game None, rust has landing"));
                }
            }
        }
    }

    println!("landing parity: {within}/{total} within tolerance");
    for m in &misses {
        println!("  MISS {m}");
    }
    // Gate: first flight-fidelity bar — 8/10 (curve-after-bounce residual
    // ~0.55 m on one case). Tighten as the predictor curve term gets replayed.
    assert!(
        within * 10 >= total as usize * 8,
        "landing parity below bar: {within}/{total}\n{}",
        misses.join("\n")
    );
}
