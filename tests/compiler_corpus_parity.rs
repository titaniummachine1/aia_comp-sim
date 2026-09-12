//! Compiler -> VM corpus guard: every committed compiler save must lower to a
//! **sound** VM program (no unimplemented node type), and replay without
//! panicking.
//!
//! This is the compiler<->VM boundary check the plan's D5 calls for: the
//! compiler may only emit nodes the VM implements, or a bot would load and make
//! decisions on Null. Any `Unimplemented` node type fails the test loudly.
//!
//! The example saves live in the sibling `aia_graphc` checkout; override the
//! directory with `GRAPHC_EXAMPLES` (default `../aia_graphc/examples`).
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use aia_comp_sim::brain::TeamBrain;
use aia_comp_sim::graph::load_graph;
use aia_comp_sim::graph_vm::lower::Lowerer;
use aia_comp_sim::graph_vm::{ProgramBuilder, RuntimeBrain};
use aia_comp_sim::mode::GameSpec;
use aia_comp_sim::tennis::api::build_team_api;
use aia_comp_sim::tennis::court::Side;
use aia_comp_sim::tennis::world::TennisWorld;

static LOCK: Mutex<()> = Mutex::new(());

fn examples_dir() -> PathBuf {
    if let Ok(p) = std::env::var("GRAPHC_EXAMPLES") {
        return PathBuf::from(p);
    }
    // tests run with cwd = crate root; the sibling checkout is ../aia_graphc
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("aia_graphc")
        .join("examples")
}

fn is_tennis(graph: &aia_comp_sim::graph::TeamGraph) -> bool {
    graph
        .nodes
        .values()
        .any(|n| n.id == "TennisController" || n.id.starts_with("Tennis"))
}

#[test]
fn compiler_example_saves_lower_soundly() {
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let dir = examples_dir();
    if !dir.is_dir() {
        eprintln!("skip: no examples dir at {}", dir.display());
        return;
    }
    let mut checked = 0usize;
    let mut problems: Vec<String> = Vec::new();
    for entry in std::fs::read_dir(&dir).expect("read examples dir") {
        let path = entry.expect("dir entry").path();
        if path.extension().and_then(|e| e.to_str()) != Some("txt") {
            continue;
        }
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        // Only compiler-produced saves have serializableNodes JSON; skip others.
        let graph = match load_graph(&path, Some(GameSpec::tennis_builder())) {
            Ok(g) if !g.nodes.is_empty() => g,
            _ => continue,
        };
        checked += 1;
        let tennis = is_tennis(&graph);
        let spec = GameSpec::tennis_builder();
        aia_comp_sim::graph_vm::diagnostics::clear();
        let mut compiled = Lowerer::compile_for(graph, spec);
        let program = Arc::new(ProgramBuilder.pack(&compiled));
        let mut brain =
            RuntimeBrain::from_program(program, compiled.vars.clone(), compiled.apis.clone());

        // Replay a few ticks so lowering/execution errors surface too.
        if tennis {
            let mut world = TennisWorld::new(7);
            let _ = compiled.settle.instructions.len();
            for _ in 0..4 {
                let api = build_team_api(&world, Side::Home);
                aia_comp_sim::debug_draw::begin_frame();
                let o = brain.think(&api);
                world.step([o.tennis_command, None]);
            }
        }

        let unsound = aia_comp_sim::graph_vm::diagnostics::unsound();
        if !unsound.is_empty() {
            problems.push(format!("{name}: UNSOUND (unimplemented): {unsound:?}"));
        }
    }
    assert!(checked > 0, "no compiler saves found under {}", dir.display());
    assert!(
        problems.is_empty(),
        "compiler emitted nodes the VM cannot implement:\n  {}",
        problems.join("\n  ")
    );
    eprintln!("compiler corpus sound: {checked} save(s) checked");
}
