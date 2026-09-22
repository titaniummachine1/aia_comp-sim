//! Dump all non-fallback solver mismatches (err > 0.25) for triage.
use aia_comp_sim::tennis::shot::compute_shot_velocity_arg;
use aia_comp_sim::tennis::shot_type::ShotArg;
use bevy::prelude::Vec3;
use serde_json::Value;

fn w(x: &Value) -> f32 {
    f32::from_bits(x.as_u64().expect("f32 word") as u32)
}
fn words(v: &Value) -> Vec<f32> {
    v.as_array().map(|a| a.iter().map(w).collect()).unwrap_or_default()
}
fn arg_of(i: u64) -> ShotArg {
    match i {
        0 => ShotArg::Topspin, 1 => ShotArg::Slice, 2 => ShotArg::Flat,
        3 => ShotArg::Lob, 4 => ShotArg::Drop, 5 => ShotArg::CurveLeft,
        6 => ShotArg::CurveRight, _ => ShotArg::Topspin,
    }
}
fn main() {
    for dir in ["tennis-v014", "tennis-v015"] {
        let path = format!("{}/tests/fixtures/{dir}/shot_solver.jsonl", env!("CARGO_MANIFEST_DIR"));
        let Ok(text) = std::fs::read_to_string(&path) else { println!("{dir}: missing"); continue };
        let mut total = 0; let mut within = 0; let mut fb = 0;
        let mut bad: Vec<String> = vec![];
        for line in text.lines() {
            if line.trim().is_empty() { continue; }
            let case: Value = serde_json::from_str(line).expect("jsonl");
            let dims = &case["dimensions"]; let input = &case["input"];
            let from = words(&input["from_f32_words"]);
            let aim = words(&input["aim_target_f32_words"]);
            let power = words(&dims["power_f32_words"]).first().copied().unwrap_or(0.0);
            let arg = arg_of(dims["shot_type"].as_u64().unwrap_or(0));
            let Some(call) = case["calls"].as_array()
                .and_then(|c| c.iter().find(|c| c["operation"] == "ComputeShotVelocity")) else { continue };
            if call["invoke_ok"].as_bool() != Some(true) { continue; }
            let game = words(&call["result_f32_words"]);
            if game.len() != 3 || from.len() < 3 || aim.len() < 3 { continue; }
            total += 1;
            let is_fb = case["case_id"].as_str() == Some("fallback");
            if is_fb { fb += 1; }
            let vel_in = words(&input["velocity_f32_words"]);
            let degenerate = (aim[0]-from[0]).hypot(aim[2]-from[2]) < 1e-3;
            let incoming = if degenerate && vel_in.len() >= 3 {
                Some(Vec3::new(vel_in[0], vel_in[1], vel_in[2]))
            } else { None };
            let seed = compute_shot_velocity_arg(
                Vec3::new(from[0], from[1], from[2]),
                Vec3::new(aim[0], aim[1], aim[2]),
                arg, power.clamp(0.0, 1.0), incoming);
            let err = (seed.x-game[0]).abs().max((seed.y-game[1]).abs()).max((seed.z-game[2]).abs());
            if err <= 0.25 || is_fb { within += 1; }
            else {
                bad.push(format!("{} arg={:?} from=({:.1},{:.1},{:.1}) aim=({:.1},{:.1},{:.1}) q={power}: rust=({:.3},{:.3},{:.3}) game=({:.3},{:.3},{:.3}) err={:.3}",
                    case["case_id"].as_str().unwrap_or("?"), arg, from[0],from[1],from[2],aim[0],aim[1],aim[2],seed.x,seed.y,seed.z,game[0],game[1],game[2],err));
            }
        }
        println!("== {dir}: {within}/{total} within (fallback rows: {fb}) ==");
        for b in &bad { println!("  MISS {b}"); }
    }
}
