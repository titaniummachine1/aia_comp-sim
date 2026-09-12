//! Observable traces for acceptance: GraphBrain ≡ Runtime (phased).

use bevy::prelude::Vec2;

use crate::brain::BrainOutput;
use crate::graph_vm::value::VmValue;

/// One SetVariable commit observed during a settle pass.
#[derive(Debug, Clone, PartialEq)]
pub struct VarCommit {
    pub name: String,
    pub value: VmValue,
}

/// Full think observability — acceptance criterion (not score/win-rate).
#[derive(Debug, Clone, PartialEq)]
pub struct ObservableTrace {
    /// Exactly 8 passes matching GraphBrain settle loops.
    pub passes: [Vec<VarCommit>; 8],
    pub controllers: BrainOutput,
}

impl ObservableTrace {
    pub fn empty() -> Self {
        Self {
            passes: Default::default(),
            controllers: BrainOutput::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TraceMismatch {
    pub tick: u64,
    pub phase: String,
    pub detail: String,
    pub source_sid: String,
    pub source_port: String,
    pub expected: String,
    pub actual: String,
}

impl std::fmt::Display for TraceMismatch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Tick {}\n  → {}\n  → {}\n  Instruction SID: {} Port: {}\n  Expected: {}\n  Actual: {}",
            self.tick,
            self.phase,
            self.detail,
            self.source_sid,
            self.source_port,
            self.expected,
            self.actual
        )
    }
}

pub fn compare_traces(
    tick: u64,
    reference: &ObservableTrace,
    runtime: &ObservableTrace,
) -> Option<TraceMismatch> {
    for (pi, (rp, tp)) in reference
        .passes
        .iter()
        .zip(runtime.passes.iter())
        .enumerate()
    {
        if rp.len() != tp.len() {
            return Some(TraceMismatch {
                tick,
                phase: format!("Pass {}", pi + 1),
                detail: "variable commit count".into(),
                source_sid: String::new(),
                source_port: String::new(),
                expected: format!("{} commits", rp.len()),
                actual: format!("{} commits", tp.len()),
            });
        }
        for (a, b) in rp.iter().zip(tp.iter()) {
            if a.name != b.name || !a.value.same_value(b.value) {
                return Some(TraceMismatch {
                    tick,
                    phase: format!("Pass {}", pi + 1),
                    detail: format!("Variable: {}", a.name),
                    source_sid: String::new(),
                    source_port: String::new(),
                    expected: format!("{:?}", a.value),
                    actual: format!("{:?}", b.value),
                });
            }
        }
    }
    for i in 0..4 {
        let a = &reference.controllers.commands[i];
        let b = &runtime.controllers.commands[i];
        if a.sprint != b.sprint || a.interact != b.interact || !vec_eq(a.move_to, b.move_to) {
            return Some(TraceMismatch {
                tick,
                phase: "Controllers".into(),
                detail: format!("Controller {}", i + 1),
                source_sid: String::new(),
                source_port: String::new(),
                expected: format!("{:?}", a),
                actual: format!("{:?}", b),
            });
        }
    }
    // Tennis-mode controller. A tennis graph leaves the soccer `commands`
    // array at its default, so without this the trace identity would pass
    // vacuously for every tennis save. Compare the real controller output.
    let (ta, tb) = (
        reference.controllers.tennis_command,
        runtime.controllers.tennis_command,
    );
    if ta.is_some() != tb.is_some() {
        return Some(TraceMismatch {
            tick,
            phase: "Controllers".into(),
            detail: "TennisController presence".into(),
            source_sid: String::new(),
            source_port: String::new(),
            expected: format!("{ta:?}"),
            actual: format!("{tb:?}"),
        });
    }
    if let (Some(a), Some(b)) = (ta, tb) {
        if a.swing != b.swing
            || a.sprint != b.sprint
            || !near_f(a.shot_type, b.shot_type)
            || !near_v(a.move_or_aim, b.move_or_aim)
            || !near_opt_v(a.aim, b.aim)
        {
            return Some(TraceMismatch {
                tick,
                phase: "Controllers".into(),
                detail: "TennisController (move_or_aim/swing/shot_type/sprint/aim)".into(),
                source_sid: String::new(),
                source_port: String::new(),
                expected: format!("{a:?}"),
                actual: format!("{b:?}"),
            });
        }
    }
    None
}

/// Float tolerance for tennis controller comparisons. Positions agree to
/// sub-micro precision when the reference and the VM are truly identical;
/// this bounds genuine rounding without hiding a real divergence.
const TENNIS_EPS: f32 = 1e-4;

fn near_f(a: f32, b: f32) -> bool {
    crate::graph_vm::value::nan_eq(a, b) || (a - b).abs() <= TENNIS_EPS
}

fn near_v(a: Vec2, b: Vec2) -> bool {
    near_f(a.x, b.x) && near_f(a.y, b.y)
}

fn near_opt_v(a: Option<Vec2>, b: Option<Vec2>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => near_v(a, b),
        _ => false,
    }
}

fn vec_eq(a: Vec2, b: Vec2) -> bool {
    crate::graph_vm::value::nan_eq(a.x, b.x) && crate::graph_vm::value::nan_eq(a.y, b.y)
}
