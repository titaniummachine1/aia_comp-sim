//! Compiler optimization-mode parity, observed in the sim itself.
//!
//! The SAME bot source is compiled at every optimize mode (`o0`/`o1`/`o2`);
//! the three saves are replayed head-to-head tick by tick and their
//! controller output must be byte-identical. The modes may only drop
//! unnecessary material:
//!   o1 drops debug sinks (api.plot) and re-runs DCE,
//!   o2 additionally drops visual chrome (nodes at 0,0, no colors / port
//!      rects / connection metadata) and remaps ids to dense base62.
//! Neither may change behavior. `o0` is the default and must keep debug
//! sinks working.
//!
//! Fixtures (regenerate with `python scripts/run_compiler_mode_parity.py`):
//!   data/compiler_probes/mode_parity/{o0,o1,o2}.txt
use std::path::PathBuf;
use std::sync::Mutex;

use aia_comp_sim::brain::{BrainOutput, TeamBrain, TeamId};
use aia_comp_sim::graph::{load_graph, TeamGraph};
use aia_comp_sim::graph_vm::RuntimeBrain;
use aia_comp_sim::mode::GameSpec;

/// The sim's debug channel store is process-global; serialize the tests that
/// run a brain (which writes TimePlots) so they cannot cross-contaminate.
static DEBUG_LOCK: Mutex<()> = Mutex::new(());

const TICKS: usize = 40;

fn fixture(mode: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("data/compiler_probes/mode_parity")
        .join(format!("{mode}.txt"))
}

fn load(mode: &str) -> TeamGraph {
    load_graph(&fixture(mode), Some(GameSpec::soccer()))
        .unwrap_or_else(|e| panic!("load mode_parity/{mode}.txt: {e}"))
}

fn run_mode(mode: &str) -> Vec<BrainOutput> {
    let graph = load(mode);
    // Each mode gets a fresh cached compile; same GameSpec, same lowering.
    let mut brain = RuntimeBrain::compile_for(graph, GameSpec::soccer());
    let api = aia_comp_sim::api::TeamApi::empty(TeamId::Home);
    (0..TICKS).map(|_| brain.think(&api)).collect()
}

fn timeplot_count(graph: &TeamGraph) -> usize {
    graph.nodes.values().filter(|n| n.id == "TimePlot").count()
}

#[test]
fn compiler_modes_are_behaviorally_identical() {
    if !fixture("o0").exists() {
        eprintln!("skip: mode_parity fixtures not generated (run run_compiler_mode_parity.py)");
        return;
    }
    let _guard = DEBUG_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    let o0 = run_mode("o0");
    let o1 = run_mode("o1");
    let o2 = run_mode("o2");

    for t in 0..TICKS {
        assert_eq!(
            o0[t], o1[t],
            "o1 diverged from the default o0 at tick {t}\n  o0: {:?}\n  o1: {:?}",
            o0[t], o1[t]
        );
        assert_eq!(
            o0[t], o2[t],
            "o2 diverged from the default o0 at tick {t}\n  o0: {:?}\n  o2: {:?}",
            o0[t], o2[t]
        );
    }

    // Guard against a vacuous pass: the bot must actually be moving and the
    // state must evolve (latch + loop + if), or "parity" would mean nothing.
    let distinct: std::collections::HashSet<String> = o0
        .iter()
        .map(|o| format!("{:?}", o.commands[0].move_to))
        .collect();
    assert!(
        distinct.len() > 5,
        "bot output is too static to prove parity ({} distinct positions)",
        distinct.len()
    );
}

#[test]
fn default_mode_keeps_debug_plots() {
    if !fixture("o0").exists() {
        eprintln!("skip: mode_parity fixtures not generated (run run_compiler_mode_parity.py)");
        return;
    }
    let _guard = DEBUG_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    // Structural: o0 (default) keeps TimePlot nodes; o1/o2 strip them.
    assert!(timeplot_count(&load("o0")) > 0, "o0 must keep debug TimePlots");
    assert_eq!(timeplot_count(&load("o1")), 0, "o1 must strip debug TimePlots");
    assert_eq!(timeplot_count(&load("o2")), 0, "o2 must strip debug TimePlots");

    // Functional: the default build's debug channels still read live values.
    let graph = load("o0");
    let mut brain = RuntimeBrain::compile_for(graph, GameSpec::soccer());
    let api = aia_comp_sim::api::TeamApi::empty(TeamId::Home);
    let mut seen_n = Vec::new();
    for _ in 0..TICKS {
        aia_comp_sim::debug_draw::begin_frame();
        brain.think(&api);
        let plots = aia_comp_sim::debug_draw::snapshot().plots;
        let get = |ch: &str| {
            plots
                .iter()
                .find(|(name, _)| name == ch)
                .map(|(_, v)| *v)
                .unwrap_or_else(|| panic!("missing debug channel {ch} in {plots:?}"))
        };
        seen_n.push(get("MP.n"));
    }
    // CC-style latch sanity: n = 1,2,3,... (same-tick store visibility).
    for (i, n) in seen_n.iter().enumerate() {
        assert!(
            (n - (i as f32 + 1.0)).abs() < 1e-4,
            "MP.n tick {i}: got {n} want {}",
            i + 1
        );
    }
}
