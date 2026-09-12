//! Compiler-probe check: the graphc `api.plot` save runs in the sim VM and
//! every TimePlot channel reads its golden value, tick by tick.
//!
//! The fixture `data/compiler_probes/compiler_probe.txt` is produced by the
//! real compiler pipeline (plain Python -> desc -> graphc-rs); regenerate
//! with `python scripts/run_compiler_probe.py`. This test pins the sim side
//! (game side: load the .txt as a team, export a TimePlot, read CC.*):
//! N ticks, per-tick TimePlot drain, golden sequences, plus O0 vs O1 op
//! counts (size = file nodes, inference = ops executed per tick).
use aia_comp_sim::brain::{TeamBrain, TeamId};
use aia_comp_sim::graph::load_graph;
use aia_comp_sim::graph_vm::lower::Lowerer;
use aia_comp_sim::graph_vm::passes::PassManager;
use aia_comp_sim::graph_vm::RuntimeBrain;
use aia_comp_sim::mode::GameSpec;

fn plots_one_tick(brain: &mut RuntimeBrain, api: &aia_comp_sim::api::TeamApi) -> Vec<(String, f32)> {
    aia_comp_sim::debug_draw::begin_frame();
    brain.think(api);
    aia_comp_sim::debug_draw::snapshot().plots
}

#[test]
fn compiler_probe_channels_read_golden_values() {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("data/compiler_probes/compiler_probe.txt");
    let graph = load_graph(&path, Some(GameSpec::soccer())).expect("load compiler probe");
    let file_nodes = graph.nodes.len();

    // O0 vs O1 cost on the same lowering. Lexicographic objective:
    // transitions first (tick-invariant: every op fires once per think),
    // file size only to break ties.
    let o0 = Lowerer::compile_for(graph.clone(), GameSpec::soccer());
    let o0_ops = o0.settle.instructions.len() + o0.controllers.instructions.len();
    let mut o1 = o0.clone();
    let pm = PassManager::o1();
    pm.run_all(&mut o1.settle);
    pm.run_all(&mut o1.controllers);
    let o1_ops = o1.settle.instructions.len() + o1.controllers.instructions.len();
    eprintln!("compiler_probe cost: file_nodes={file_nodes} o0_ops={o0_ops} o1_ops={o1_ops}");
    assert!(o1_ops < o0_ops, "O1 must fold the const chains (CC.const/CC.gcd)");
    // Pinned cost (transitions primary, size secondary — update deliberately
    // when the bot changes; any silent drift means the optimizer regressed).
    assert_eq!(file_nodes, 93, "probe size drift");
    assert_eq!(o0_ops, 90, "O0 transition drift");
    assert_eq!(o1_ops, 80, "O1 transition drift");

    // Goldens, ticks 1..=10 (hand-verified; see scripts/run_compiler_probe.py).
    // fib_lat converges to fib(tick+1): 1,1,2,3,5,8,13,21,34,55.
    let tick = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
    let pow2 = [1.0, 2.0, 4.0, 8.0, 16.0, 32.0, 64.0, 128.0, 256.0, 512.0];
    let fact = [1.0, 2.0, 6.0, 24.0, 120.0, 720.0, 5040.0, 40320.0, 362880.0, 3628800.0];
    let horner = [11.0, 39.0, 107.0, 233.0, 435.0, 731.0, 1139.0, 1677.0, 2363.0, 3215.0];
    let fib_lat = [1.0, 1.0, 2.0, 3.0, 5.0, 8.0, 13.0, 21.0, 34.0, 55.0];

    let api = aia_comp_sim::api::TeamApi::empty(TeamId::Home);
    let mut brain = RuntimeBrain::compile_for(graph, GameSpec::soccer());
    for (step, n) in tick.iter().enumerate() {
        let plots = plots_one_tick(&mut brain, &api);
        let get = |ch: &str| {
            plots
                .iter()
                .find(|(name, _)| name == ch)
                .map(|(_, v)| *v)
                .unwrap_or_else(|| panic!("tick {n}: missing channel {ch} in {plots:?}"))
        };
        let eq = |ch: &str, want: f32| {
            let got = get(ch);
            assert!(
                (got - want).abs() < 1e-4,
                "tick {n}: {ch} got {got} want {want}"
            );
        };
        eq("CC.const", 14.0);
        eq("CC.cse_a", 12.0);
        eq("CC.cse_b", 12.0);
        eq("CC.tick", *n);
        eq("CC.pow2", pow2[step]);
        eq("CC.fact", fact[step]);
        eq("CC.horner", horner[step]);
        eq("CC.gcd", 6.0);
        eq("CC.fib20", 6765.0);
        eq("CC.fib_lat", fib_lat[step]);
        eq("CC.pw", 128.0);
        eq("CC.found", 1.0);
        let _ = step;
    }
}

/// Live-data probe: fib(position.x) -> debug. Same save runs against the
/// live world (players walk to center, x varies per tick). Nothing is
/// golden-deterministic across worlds — the checkable property is
/// self-consistency per tick: LIVE.fib == fib(int(LIVE.mi)) with
/// LIVE.mi == floor(mod(LIVE.x, 20)) normalized to [0, 20).
/// The game export must satisfy the same relations (compare script).
#[test]
fn live_position_fib_is_self_consistent() {
    use aia_comp_sim::brain::IdleBrain;
    use aia_comp_sim::params::SimParams;
    use aia_comp_sim::world::{MatchWorld, FIXED_DT};

    const FIB: [f32; 20] = [
        0.0, 1.0, 1.0, 2.0, 3.0, 5.0, 8.0, 13.0, 21.0, 34.0, 55.0, 89.0,
        144.0, 233.0, 377.0, 610.0, 987.0, 1597.0, 2584.0, 4181.0,
    ];
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("data/compiler_probes/live_position_fib.txt");
    let graph = load_graph(&path, Some(GameSpec::soccer())).expect("load live probe");
    let file_nodes = graph.nodes.len();
    let o0 = Lowerer::compile_for(graph.clone(), GameSpec::soccer());
    let o0_ops = o0.settle.instructions.len() + o0.controllers.instructions.len();
    let mut o1 = o0.clone();
    PassManager::o1().run_all(&mut o1.settle);
    PassManager::o1().run_all(&mut o1.controllers);
    let o1_ops = o1.settle.instructions.len() + o1.controllers.instructions.len();
    eprintln!("live_probe cost: file_nodes={file_nodes} o0_ops={o0_ops} o1_ops={o1_ops}");
    assert_eq!(file_nodes, 94, "live probe size drift");
    assert_eq!(o0_ops, 94, "live O0 transition drift");
    assert_eq!(o1_ops, 85, "live O1 transition drift");

    let mut brain = RuntimeBrain::compile_for(graph, GameSpec::soccer());
    let mut away = IdleBrain;
    let mut world = MatchWorld::new_kickoff_opening(SimParams::default(), TeamId::Home);
    let mut saw_varying_x = false;
    let mut first_x: Option<f32> = None;
    for tick in 0..40 {
        let (home_api, away_api) = world.build_apis();
        aia_comp_sim::debug_draw::begin_frame();
        let home_out = brain.think(&home_api);
        let plots = aia_comp_sim::debug_draw::snapshot().plots;
        let away_out = away.think(&away_api);
        world.step_with_commands(&home_out, &away_out, FIXED_DT);
        let get = |ch: &str| {
            plots
                .iter()
                .find(|(name, _)| name == ch)
                .map(|(_, v)| *v)
                .unwrap_or_else(|| panic!("tick {tick}: missing {ch} in {plots:?}"))
        };
        let x = get("LIVE.x");
        let mi = get("LIVE.mi");
        let fib = get("LIVE.fib");
        let m = x % 20.0;
        let m = if m < 0.0 { m + 20.0 } else { m };
        let want_mi = m - m % 1.0;
        assert!((mi - want_mi).abs() < 1e-3, "tick {tick}: mi {mi} vs {want_mi} (x={x})");
        assert!((0.0..20.0).contains(&mi), "tick {tick}: mi {mi} out of range");
        assert!(
            (fib - FIB[mi as usize]).abs() < 1e-3,
            "tick {tick}: fib {fib} vs table {} (mi={mi}, x={x})",
            FIB[mi as usize]
        );
        if let Some(fx) = first_x {
            if (x - fx).abs() > 1e-3 {
                saw_varying_x = true;
            }
        } else {
            first_x = Some(x);
        }
    }
    assert!(saw_varying_x, "LIVE.x never varied — not a live check");
}
