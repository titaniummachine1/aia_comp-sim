//! Opt-in CSE IR diff — dump a save's LoweredIR before/after CSE so a bad
//! merge can be read directly (targets the `bat` O0!=O1 finding, HANDOFF 17b).
//!
//!   CSE_DIFF=bat cargo test --test cse_ir_diff -- --nocapture
//!
//! Writes `cse_{settle,ctrl}_{before,after}.txt` into `%TEMP%` and prints the
//! instruction counts. Diff the two files (or read them) to see which producer
//! CSE aliased.
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use aia_comp_sim::graph::load_graph;
use aia_comp_sim::graph_vm::ir::LoweredIR;
use aia_comp_sim::graph_vm::lower::Lowerer;
use aia_comp_sim::graph_vm::passes::{ConstFold, Cse, Pass, RelayRemoval};
use aia_comp_sim::mode::GameSpec;

static LOCK: Mutex<()> = Mutex::new(());

fn saves_dir() -> PathBuf {
    PathBuf::from(std::env::var("USERPROFILE").unwrap_or_default())
        .join(r"AppData\LocalLow\Unicorn One\AIComp\Saves\Tennis")
}

fn dump(ir: &LoweredIR, name: &str) {
    let mut s = String::new();
    for (i, inst) in ir.instructions.iter().enumerate() {
        s.push_str(&format!(
            "{i:5} {:?} dst={:?} args={:?} imm={:?} sid={} port={}\n",
            inst.op, inst.dest, inst.args, inst.immediates, inst.source_sid, inst.source_port
        ));
    }
    let p = std::env::temp_dir().join(name);
    std::fs::write(&p, s).unwrap();
    println!("wrote {}", p.display());
}

#[test]
fn cse_ir_diff() {
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let Ok(name) = std::env::var("CSE_DIFF") else {
        eprintln!("skip: set CSE_DIFF=<save name> (e.g. CSE_DIFF=bat)");
        return;
    };
    let path = saves_dir().join(format!("{name}.txt"));
    if !path.exists() {
        eprintln!("skip: no save at {}", path.display());
        return;
    }
    let graph = load_graph(&path, Some(GameSpec::tennis_builder())).expect("load save");
    let mut compiled = Lowerer::compile_for(graph, GameSpec::tennis_builder());
    for ir in [&mut compiled.settle, &mut compiled.controllers] {
        ConstFold.run(ir);
        RelayRemoval.run(ir);
    }
    let before = compiled.clone();
    for ir in [&mut compiled.settle, &mut compiled.controllers] {
        Cse.run(ir);
    }
    dump(&before.settle, "cse_settle_before.txt");
    dump(&compiled.settle, "cse_settle_after.txt");
    dump(&before.controllers, "cse_ctrl_before.txt");
    dump(&compiled.controllers, "cse_ctrl_after.txt");
    println!(
        "settle {} -> {} ; controllers {} -> {}",
        before.settle.instructions.len(),
        compiled.settle.instructions.len(),
        before.controllers.instructions.len(),
        compiled.controllers.instructions.len()
    );
    let _ = Arc::new(0); // keep imports stable
}
