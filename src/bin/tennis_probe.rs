//! Interpreter-semantics probe: run the sim_probe graph headlessly, drain
//! the TimePlot stream per tick, dump channel -> [(tick, value)] JSON.
//!
//! Usage: tennis_probe [--seed 7] [--ticks 600] [--out probe_sim.json]

use aia_comp_sim::tennis::vm::{BrainSide, TennisBrain};
use aia_comp_sim::tennis::world::TennisWorld;
use aia_comp_sim::brain::TeamId;

fn main() {
    let mut seed: u64 = 7;
    let mut ticks: usize = 600;
    let mut out = String::from("probe_sim.json");
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--seed" => seed = args.next().and_then(|v| v.parse().ok()).unwrap_or(seed),
            "--ticks" => ticks = args.next().and_then(|v| v.parse().ok()).unwrap_or(ticks),
            "--out" => out = args.next().unwrap_or(out),
            _ => {}
        }
    }

    let path = std::path::PathBuf::from(std::env::var("USERPROFILE").expect("USERPROFILE"))
        .join(r"AppData\LocalLow\Unicorn One\AIComp\Saves\Tennis\sim_probe.txt");
    let graph = aia_comp_sim::graph::load::load_graph(
        &path,
        Some(aia_comp_sim::mode::GameSpec::tennis_builder()),
    )
    .expect("probe graph loads");
    let brain = TennisBrain::compile(graph);
    let mut home = Some(BrainSide { brain, team: TeamId::Home });
    let mut away: Option<BrainSide> = None;
    let mut world = TennisWorld::new(seed);

    let mut series: std::collections::BTreeMap<String, Vec<(usize, f64)>> = Default::default();
    for tick in 0..ticks {
        let home_cmd = home.as_mut().and_then(|b| b.command_for(&world));
        let away_cmd = away.as_mut().and_then(|b| b.command_for(&world));
        world.step([home_cmd, away_cmd]);
        let frame = aia_comp_sim::debug_draw::snapshot();
        for (name, v) in frame.plots {
            let e = series.entry(name).or_default();
            // one sample per tick per channel
            if e.last().map(|(t, _)| *t) != Some(tick) {
                e.push((tick, v as f64));
            } else {
                e.last_mut().unwrap().1 = v as f64;
            }
        }
    }

    let text = json_string(&series);
    std::fs::write(&out, text).expect("write probe out");
    println!("wrote {out} ({} channels, {} ticks)", series.len(), ticks);
}

// Minimal JSON serializer (avoids adding serde_json to the bin's Cargo scope —
// it is already a workspace dep, but keep this self-contained anyway).
fn json_string(series: &std::collections::BTreeMap<String, Vec<(usize, f64)>>) -> String {
    use std::fmt::Write;
    let mut s = String::from("{");
    for (i, (name, samples)) in series.iter().enumerate() {
        if i > 0 {
            s.push(',');
        }
        let _ = write!(s, "{}:[", serde_escape(name));
        for (j, (t, v)) in samples.iter().enumerate() {
            if j > 0 {
                s.push(',');
            }
            let vs = if v.is_nan() {
                "NaN".to_string()
            } else if v.is_infinite() {
                if *v > 0.0 {
                    "Infinity".to_string()
                } else {
                    "-Infinity".to_string()
                }
            } else {
                format!("{v:.6}")
            };
            let _ = write!(s, "[{t},{vs}]");
        }
        s.push(']');
    }
    s.push('}');
    s
}

fn serde_escape(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}
