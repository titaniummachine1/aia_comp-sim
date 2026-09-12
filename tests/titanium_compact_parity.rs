//! Deep parity on a real, hand-built bot: compact any save and prove the
//! sim plays it identically to the original.
//!
//! Opt-in, because the fixtures are multi-MB and dev-machine specific:
//!
//! ```text
//!   $env:TITANIUM_ORIG    = "...\Saves\Soccer\Titanium.txt"
//!   $env:TITANIUM_COMPACT = "...\graphc-rs-compact-o2.txt"
//!   cargo test --release --test titanium_compact_parity -- --nocapture
//! ```
//!
//! The graphc side is validated structurally by `graphc::compact` unit tests
//! (a producer that also reaches a controller is always kept; only
//! debug-exclusive chains drop). This test closes the loop in the sim: both
//! saves run head-to-head on the same world snapshots and must emit the same
//! controller commands every tick.
use std::path::PathBuf;

use aia_comp_sim::brain::{BrainOutput, TeamBrain, TeamId};
use aia_comp_sim::graph::load_graph;
use aia_comp_sim::graph_vm::RuntimeBrain;
use aia_comp_sim::mode::GameSpec;
use aia_comp_sim::params::SimParams;
use aia_comp_sim::world::{MatchWorld, FIXED_DT};

const TICKS: u64 = 60;

fn brain(path: &str) -> RuntimeBrain {
    let p = PathBuf::from(path);
    let graph = load_graph(&p, Some(GameSpec::soccer()))
        .unwrap_or_else(|e| panic!("load {}: {e}", p.display()));
    RuntimeBrain::compile_for(graph, GameSpec::soccer())
}

#[test]
fn compacted_save_matches_original_bot_tick_for_tick() {
    let (Ok(orig), Ok(compact)) =
        (std::env::var("TITANIUM_ORIG"), std::env::var("TITANIUM_COMPACT"))
    else {
        eprintln!("skip: set TITANIUM_ORIG and TITANIUM_COMPACT to run this test");
        return;
    };

    let mut orig_brain = brain(&orig);
    let mut compact_brain = brain(&compact);
    let mut away = aia_comp_sim::brain::IdleBrain;
    let mut world = MatchWorld::new_kickoff_opening(SimParams::default(), TeamId::Home);

    for tick in 0..TICKS {
        let (home_api, away_api) = world.build_apis();
        let a = orig_brain.think(&home_api);
        let b = compact_brain.think(&home_api);
        assert_outputs_match(tick, &a, &b);

        let away_out = away.think(&away_api);
        // Advance with the ORIGINAL commands; parity means this is also what
        // the compacted brain wanted.
        world.step_with_commands(&a, &away_out, FIXED_DT);
    }
    eprintln!("compacted bot matched the original for {TICKS} ticks");
}

fn assert_outputs_match(tick: u64, a: &BrainOutput, b: &BrainOutput) {
    for i in 0..4 {
        let x = a.commands[i];
        let y = b.commands[i];
        assert!(
            (x.move_to.x - y.move_to.x).abs() <= 1e-4
                && (x.move_to.y - y.move_to.y).abs() <= 1e-4
                && x.sprint == y.sprint
                && x.interact == y.interact
                && x.shoot == y.shoot,
            "controller {} diverged at tick {tick}\n  original: {x:?}\n  compacted: {y:?}",
            i + 1
        );
    }
}
