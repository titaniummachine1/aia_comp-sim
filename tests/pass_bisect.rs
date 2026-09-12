//! Opt-in pass bisector — which O1 pass flips a save's behaviour?
//!
//! Report-only and opt-in (`PASS_BISECT=<save name>`). Drives the same graph
//! through progressive prefixes of the O1 pipeline
//! (ConstFold -> RelayRemoval -> CSE -> Fusion -> RegAlloc) for N ticks and
//! prints the TennisController aim/shot per prefix, so the first prefix that
//! differs from O0 pinpoints the offending pass. Directly targets the
//! `bat` O0 != O1 finding (HANDOFF section 14, finding 2).
//!
//!   PASS_BISECT=bat cargo test --test pass_bisect -- --nocapture
//!   PASS_BISECT=bat PASS_BISECT_TICKS=8 cargo test --test pass_bisect -- --nocapture
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use aia_comp_sim::brain::TeamBrain;
use aia_comp_sim::graph::load_graph;
use aia_comp_sim::graph_vm::lower::Lowerer;
use aia_comp_sim::graph_vm::passes::PassManager;
use aia_comp_sim::graph_vm::{ProgramBuilder, RuntimeBrain};
use aia_comp_sim::mode::GameSpec;
use aia_comp_sim::tennis::api::build_team_api;
use aia_comp_sim::tennis::court::Side;
use aia_comp_sim::tennis::world::TennisWorld;

static LOCK: Mutex<()> = Mutex::new(());

fn saves_dir() -> PathBuf {
    PathBuf::from(std::env::var("USERPROFILE").unwrap_or_default())
        .join(r"AppData\LocalLow\Unicorn One\AIComp\Saves\Tennis")
}

/// Run `name` for `ticks`, applying `pm` (None = pass-free O0), and return one
/// summary line per tick.
fn run(name: &str, ticks: u64, pm: Option<&PassManager>) -> Vec<String> {
    let graph =
        load_graph(&saves_dir().join(format!("{name}.txt")), Some(GameSpec::tennis_builder()))
            .expect("load save");
    let mut compiled = Lowerer::compile_for(graph, GameSpec::tennis_builder());
    if let Some(pm) = pm {
        pm.run_all(&mut compiled.settle);
        pm.run_all(&mut compiled.controllers);
    }
    let program = Arc::new(ProgramBuilder.pack(&compiled));
    let mut brain = RuntimeBrain::from_program(program, compiled.vars.clone(), compiled.apis.clone());
    aia_comp_sim::debug_draw::begin_frame();

    let mut world = TennisWorld::new(20_260_907);
    let mut out = Vec::new();
    for _ in 0..ticks {
        let api = build_team_api(&world, Side::Home);
        aia_comp_sim::debug_draw::begin_frame();
        let o = brain.think(&api);
        if let Some(t) = o.tennis_command {
            out.push(format!(
                "aim={:?} shot={:.3} swing={} move=({:.3},{:.3})",
                t.aim, t.shot_type, t.swing, t.move_or_aim.x, t.move_or_aim.y
            ));
        } else {
            out.push("<no tennis controller>".into());
        }
        world.step([o.tennis_command, None]);
    }
    out
}

#[test]
fn pass_bisect() {
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let Ok(name) = std::env::var("PASS_BISECT") else {
        eprintln!("skip: set PASS_BISECT=<save name> (e.g. PASS_BISECT=bat)");
        return;
    };
    let path = saves_dir().join(format!("{name}.txt"));
    if !path.exists() {
        eprintln!("skip: no save at {}", path.display());
        return;
    }
    let ticks: u64 = std::env::var("PASS_BISECT_TICKS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(8);

    let o0 = run(&name, ticks, None);
    println!("\n{name}: {ticks} ticks");
    println!("  O0          : {}", o0.join("  |  "));
    let mut first_bad: Option<usize> = None;
    for k in 1..=5 {
        let pm = PassManager::o1_prefix(k);
        let got = run(&name, ticks, Some(&pm));
        let same = got == o0;
        if !same && first_bad.is_none() {
            first_bad = Some(k);
        }
        println!(
            "  +{:<11} ({}): {}  {}",
            pm.names().last().copied().unwrap_or("-"),
            k,
            if same { "== O0" } else { "!= O0" },
            got.join("  |  ")
        );
    }
    match first_bad {
        Some(k) => println!(
            "\n  first divergence at pass #{k}: {} (see graph_vm/passes)",
            PassManager::o1_prefix(k).names().last().copied().unwrap_or("?")
        ),
        None => println!("\n  all prefixes == O0"),
    }
}
