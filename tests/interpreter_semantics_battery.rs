//! Interpreter semantics battery — behaviour proof via TimePlot channels.
//!
//! Companion to `tennis_interpreter_certification.rs`: that one replays real
//! bots (what they happen to touch), this one probes the VM's semantics
//! directly. `scripts/gen_interpreter_battery.py` emits one graph where every
//! ambiguous behaviour is wired to a named `B.*` TimePlot channel, so the
//! answer is readable channel-by-channel instead of inferred.
//!
//! Three interpreters, two observation paths:
//!   reference  `GraphBrain` (src/graph/eval.rs)     channels + trace
//!   VM O0      `Lowerer` + no passes                channels + trace
//!   VM O1      `Lowerer` + `PassManager::o1()`      trace only
//!
//! O1 strips debug sinks by design (`compiler_mode_parity`), so channel parity
//! is reference-vs-O0; O1 is still covered by the Pass 1..8 var-commit trace.
//!
//! Everything is REPORTED; nothing fails unless `BATTERY_STRICT=1`. Goldens
//! only pin unambiguous math (compare dropdown, clamp, power, vector
//! round-trip, double negation) — deliberately NOT the policy questions
//! (unwired `AddFloats`, the ConditionalSet hold rule, `Operation` empty
//! modifier), which are exactly what this battery is meant to answer.
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use aia_comp_sim::api::TeamApi;
use aia_comp_sim::brain::{TeamBrain, TeamId};
use aia_comp_sim::graph::{load_graph, GraphBrain};
use aia_comp_sim::graph_vm::lower::Lowerer;
use aia_comp_sim::graph_vm::{compare_traces, ProgramBuilder, RuntimeBrain};
use aia_comp_sim::mode::GameSpec;

static LOCK: Mutex<()> = Mutex::new(());

const TICKS: u64 = 12;
const MAX_DIVERGENCE_LINES: usize = 40;
const EPS: f32 = 1e-4;

fn fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("data/interpreter_probes/semantics_battery.txt")
}

/// Channels whose value is unambiguous math (not a policy question).
/// (channel, expected) — checked every tick.
const GOLDEN: &[(&str, f32)] = &[
    ("B.cmp_0", 1.0),
    ("B.cmp_1", 0.0),
    ("B.cmp_2", 0.0),
    ("B.cmp_3", 1.0),
    ("B.cmp_4", 1.0),
    ("B.cmp_lt", 1.0),
    ("B.vec_x", 1.5),
    ("B.vec_y", 2.5),
    ("B.vec_z", 3.5),
    ("B.clamp_hi", 1.0),
    ("B.clamp_lo", -1.0),
    ("B.clamp_in", 0.5),
    ("B.pow_neg", 0.5),
    ("B.not_chain", 1.0),
];

fn channels() -> BTreeMap<String, f32> {
    aia_comp_sim::debug_draw::snapshot()
        .plots
        .into_iter()
        .collect()
}

fn near(a: f32, b: f32) -> bool {
    (a - b).abs() <= EPS || (a.is_nan() && b.is_nan())
}

#[test]
fn interpreter_semantics_battery() {
    let _guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let path = fixture();
    if !path.exists() {
        eprintln!("skip: run `python scripts/gen_interpreter_battery.py` first");
        return;
    }
    let graph = load_graph(&path, Some(GameSpec::tennis_builder())).expect("load battery");
    if !graph.nodes.values().any(|n| n.id == "TimePlot") {
        panic!("battery has no TimePlot channels — regenerate the fixture");
    }

    let mut reference = GraphBrain::new(graph.clone()).with_trace();
    let mut vm_o1 =
        RuntimeBrain::compile_for(graph.clone(), GameSpec::tennis_builder()).with_trace();
    let compiled0 = Lowerer::compile_for(graph, GameSpec::tennis_builder());
    let program0 = Arc::new(ProgramBuilder.pack(&compiled0));
    let mut vm_o0 = RuntimeBrain::from_program(
        program0,
        compiled0.vars.clone(),
        compiled0.apis.clone(),
    )
    .with_trace();

    let api = TeamApi::empty(TeamId::Home);
    let strict = std::env::var("BATTERY_STRICT").as_deref() == Ok("1");

    let mut chan_div: Vec<String> = Vec::new();
    let mut trace_div: Vec<String> = Vec::new();
    let mut golden_div: Vec<String> = Vec::new();
    let mut diverged: BTreeSet<String> = BTreeSet::new();
    let mut series: BTreeMap<String, Vec<f32>> = BTreeMap::new();
    let mut last_ref: BTreeMap<String, f32> = BTreeMap::new();

    for tick in 0..TICKS {
        aia_comp_sim::debug_draw::begin_frame();
        reference.think(&api);
        let ref_ch = channels();
        let ref_tr = reference.take_trace().expect("reference with_trace");
        reference = reference.with_trace();

        // O1 is compiled with the optimizer, which strips debug sinks: no
        // channels, trace only.
        aia_comp_sim::debug_draw::begin_frame();
        vm_o1.think(&api);
        let o1_tr = vm_o1.take_trace().expect("o1 with_trace");
        vm_o1 = vm_o1.with_trace();

        aia_comp_sim::debug_draw::begin_frame();
        vm_o0.think(&api);
        let o0_ch = channels();
        let o0_tr = vm_o0.take_trace().expect("o0 with_trace");
        vm_o0 = vm_o0.with_trace();

        // Channel parity: reference vs O0.
        for (ch, rv) in &ref_ch {
            let ov = o0_ch.get(ch).copied().unwrap_or(f32::NAN);
            if !near(*rv, ov) {
                diverged.insert(ch.clone());
                if chan_div.len() < MAX_DIVERGENCE_LINES {
                    chan_div.push(format!("tick {tick} {ch}: ref={rv} o0={ov}"));
                }
            }
            series.entry(ch.clone()).or_default().push(*rv);
        }
        // Golden math (unambiguous cases) on the reference.
        for (ch, want) in GOLDEN {
            if let Some(rv) = ref_ch.get(*ch) {
                if !near(*rv, *want) && golden_div.len() < MAX_DIVERGENCE_LINES {
                    golden_div.push(format!("tick {tick} {ch}: ref={rv} want={want}"));
                }
            }
        }
        // Trace parity: reference vs O0 and vs O1 (Pass 1..8 + controllers).
        for (tag, t) in [("o0", &o0_tr), ("o1", &o1_tr)] {
            if let Some(m) = compare_traces(tick, &ref_tr, t) {
                if trace_div.len() < MAX_DIVERGENCE_LINES {
                    trace_div.push(format!("[{tag}] tick {tick}: {m}"));
                }
            }
        }
        last_ref = ref_ch;
    }

    let total_ch = last_ref.len();
    let varying = series
        .values()
        .filter(|v| {
            v.first()
                .map(|f| v.iter().any(|x| (x - f).abs() > EPS))
                .unwrap_or(false)
        })
        .count();

    eprintln!("battery: {total_ch} channels x {TICKS} ticks ({varying} tick-varying)");
    eprintln!("  reference values @ final tick (!! = reference != O0):");
    for (ch, v) in &last_ref {
        let mark = if diverged.contains(ch) { " !!" } else { "" };
        eprintln!("    {ch:14} {v:>14.6}{mark}");
    }

    if !chan_div.is_empty() {
        eprintln!(
            "\n{} channel divergence line(s) — reference vs O0 \
             (report-only unless BATTERY_STRICT=1):",
            chan_div.len()
        );
        for d in &chan_div {
            eprintln!("  {d}");
        }
    }
    if !trace_div.is_empty() {
        eprintln!(
            "\n{} trace divergence line(s) — reference vs O0/O1:",
            trace_div.len()
        );
        for d in &trace_div {
            eprintln!("  {d}");
        }
    }
    if !golden_div.is_empty() {
        eprintln!(
            "\n{} GOLDEN mismatch line(s) — these cases are supposed to be \
             unambiguous:",
            golden_div.len()
        );
        for d in &golden_div {
            eprintln!("  {d}");
        }
    }
    if chan_div.is_empty() && trace_div.is_empty() && golden_div.is_empty() {
        eprintln!("\nPASS: reference == O0 == O1 on every channel and trace.");
    }

    assert!(
        total_ch >= 30,
        "battery too small ({total_ch} channels) — regenerate the fixture"
    );
    assert!(
        varying >= 3,
        "battery is static ({varying} tick-varying channels) — the parity check would be vacuous"
    );
    if strict {
        assert!(
            chan_div.is_empty() && trace_div.is_empty() && golden_div.is_empty(),
            "strict: {} channel / {} trace / {} golden divergence(s); see report above",
            chan_div.len(),
            trace_div.len(),
            golden_div.len()
        );
    }
}
