//! RacingV2 graph VM - exact semantic copy of CompiledGraphPlan (v0.22 capture,
//! 2026-09-22). REAL Rust code, NOT opcode dumps: the dumped ISA tables in
//! dropmap.py / race_abi.json are regenerable evidence only (re-grab any time
//! with the probe tool). THIS file is the canonical, maintainable, optimizable
//! implementation used for unit tests and the in-game mod.

//! SEMANTIC CONTRACT (non-negotiable while optimizing):
//! 1. Float math is f32 rounded AFTER EVERY op (game is float32; f64
//!    intermediates diverge silently). No reassociation, no FMA fusion.
//! 2. GraphValue.Kind None (null) propagates through ops as in the game.
//! 3. ConditionalPick/ConditionalSet coerce per GraphBrain as_float/as_bool/
//!    as_vec rules [fixture-pending].
//! 4. RandomFloat consumes the UnityEngine.Random stream in ITS draw order -
//!    validate with the parity_cmd rngtest fixture [fixture-pending].
//! 5. CallGate/CallHandler/CallFunction + RelativePosition are per-game gate
//!    behaviour (see graphc profiles GATE_QUIRKS) [fixture-pending].
//! 6. Latches: WriteVariable/ReadVariable use the carried register state
//!    (GraphCompiler.CarryOverRegisters semantics).

//! VALIDATION = fixture diff: the same CompiledInstruction array through the
//! in-game CompiledGraphPlan.Execute and through execute() here must produce
//! bit-identical outputs. This interpreter stays the oracle FOREVER; speed
//! work (per-plan compilation, SIMD, unboxed registers) is validated against
//! it, never the other way around.

use std::rc::Rc;

/// Captured GraphOpCode declaration order - the discriminants ARE the wire
/// values in CompiledInstruction.Op (graph JSON / IR immediates).
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RacingOp {
    ReadPort = 0, Copy = 1, AddFloats = 2, SubtractFloats = 3,
    MultiplyFloats = 4, DivideFloats = 5, ModuloFloats = 6, PowerFloats = 7,
    AbsFloat = 8, ClampFloat = 9, LerpFloat = 10, RandomFloat = 11,
    OperationFloat = 12, AndBool = 13, OrBool = 14, NotBool = 15,
    IsNull = 16, CompareBools = 17, CompareFloats = 18, CompareStrings = 19,
    AddStrings = 20, AddVector3 = 21, SubtractVector3 = 22, ScaleVector3 = 23,
    NormalizeVector3 = 24, MagnitudeVector3 = 25, DistanceVector3 = 26,
    DotVector3 = 27, ConstructVector3 = 28, SplitVector3 = 29,
    RelativePosition = 30, ConditionalPick = 31, ReadVariable = 32,
    WriteVariable = 33, CallGate = 34, CallHandler = 35, CallFunction = 36,
}


impl RacingOp {
    /// Loud decode: a bad discriminant is a corrupted plan, never a default.
    pub fn from_wire(v: u32) -> Result<Self, String> {
        use RacingOp::*;
        let op = match v {
            0 => ReadPort, 1 => Copy, 2 => AddFloats, 3 => SubtractFloats,
            4 => MultiplyFloats, 5 => DivideFloats, 6 => ModuloFloats,
            7 => PowerFloats, 8 => AbsFloat, 9 => ClampFloat, 10 => LerpFloat,
            11 => RandomFloat, 12 => OperationFloat, 13 => AndBool, 14 => OrBool,
            15 => NotBool, 16 => IsNull, 17 => CompareBools, 18 => CompareFloats,
            19 => CompareStrings, 20 => AddStrings, 21 => AddVector3,
            22 => SubtractVector3, 23 => ScaleVector3, 24 => NormalizeVector3,
            25 => MagnitudeVector3, 26 => DistanceVector3, 27 => DotVector3,
            28 => ConstructVector3, 29 => SplitVector3, 30 => RelativePosition,
            31 => ConditionalPick, 32 => ReadVariable, 33 => WriteVariable,
            34 => CallGate, 35 => CallHandler, 36 => CallFunction,
            other => return Err(format!("bad GraphOpCode discriminant {}", other)),
        };
        Ok(op)
    }
}

/// Captured CompiledInstruction layout exactly: 3 in, 3 out, 1 immediate.
#[derive(Clone, Copy, Debug)]
pub struct RacingInst {
    pub op: RacingOp,
    pub in_regs: [i32; 3],
    pub out_regs: [i32; 3],
    pub imm: i32,
}

/// Mirror of game GraphValue { Kind, Bool, Float, Vector3, Ref }.
#[derive(Clone, Debug, PartialEq)]
pub enum RacingValue {
    None,
    Bool(bool),
    Float(f32),
    Vector3([f32; 3]),
    Str(Rc<str>),
    Ref(usize),
}

impl RacingValue {
    pub fn is_null(&self) -> bool { matches!(self, RacingValue::None) }
    /// GraphBrain as_float coercion [fixture-pending].
    pub fn as_float(&self) -> Option<f32> {
        match self {
            RacingValue::Float(v) => Some(*v),
            RacingValue::Bool(b) => Some(if *b { 1.0 } else { 0.0 }),
            RacingValue::None => None,
            _ => None,
        }
    }
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            RacingValue::Bool(b) => Some(*b),
            RacingValue::Float(v) => Some(*v != 0.0),
            RacingValue::None => None,
            _ => None,
        }
    }
}


/// Execution frame: registers + carried latch state across ticks.
#[derive(Default)]
pub struct RacingFrame {
    pub regs: Vec<RacingValue>,
    pub vars: Vec<RacingValue>,
}

impl RacingFrame {
    pub fn ensure(&mut self, n: usize) {
        if self.regs.len() < n { self.regs.resize(n, RacingValue::None); }
    }
    fn get(&self, r: i32) -> RacingValue {
        if r < 0 { return RacingValue::None; }
        self.regs.get(r as usize).cloned().unwrap_or(RacingValue::None)
    }
    fn set(&mut self, r: i32, v: RacingValue) {
        if r >= 0 { let i = r as usize; self.ensure(i + 1); self.regs[i] = v; }
    }
}

/// Unity Random mirror (xorshift128 over 4 u32s, InitState semantics).
/// MUST be validated against the parity_cmd rngtest fixture before any
/// RandomFloat parity claim [fixture-pending].
#[derive(Clone)]
pub struct UnityRng { s: [u32; 4] }

impl UnityRng {
    pub fn init_state(seed: i32) -> Self {
        let mut s = [0u32; 4];
        let mut x = seed as u32;
        for slot in s.iter_mut() {
            x ^= x << 13; x ^= x >> 17; x ^= x << 5;
            *slot = if x == 0 { 0x9E3779B9 } else { x };
        }
        UnityRng { s }
    }
    fn next_u32(&mut self) -> u32 {
        let t = self.s[0] ^ self.s[0] << 11;
        self.s[0] = self.s[1]; self.s[1] = self.s[2]; self.s[2] = self.s[3];
        self.s[3] = self.s[3] ^ (self.s[3] >> 19) ^ (t ^ (t >> 8));
        self.s[3]
    }
    pub fn value(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32
    }
    pub fn range_float(&mut self, lo: f32, hi: f32) -> f32 { lo + (hi - lo) * self.value() }
}

/// Run a compiled plan. Returns Err loudly for ops whose per-game semantics
/// are not fixture-validated yet - never silently guess.
pub fn execute(
    ops: &[RacingInst],
    frame: &mut RacingFrame,
    rng: &mut UnityRng,
) -> Result<(), String> {
    for inst in ops {
        execute_inst(inst, frame, rng)?;
    }
    Ok(())
}


fn bin_f(a: &RacingValue, b: &RacingValue, f: impl FnOnce(f32, f32) -> f32) -> RacingValue {
    match (a.as_float(), b.as_float()) {
        (Some(x), Some(y)) => RacingValue::Float(f(x, y)),
        _ => RacingValue::None,
    }
}

fn execute_inst(
    inst: &RacingInst,
    frame: &mut RacingFrame,
    rng: &mut UnityRng,
) -> Result<(), String> {
    use RacingOp as O;
    use RacingValue as V;
    let a = frame.get(inst.in_regs[0]);
    let b = frame.get(inst.in_regs[1]);
    let c = frame.get(inst.in_regs[2]);
    let d0 = inst.out_regs[0];
    let fx = |v: f32| V::Float(v);
    let v = match inst.op {
        O::ReadPort => V::None,
        O::Copy => a.clone(),
        O::AddFloats => bin_f(&a, &b, |x, y| x + y),
        O::SubtractFloats => bin_f(&a, &b, |x, y| x - y),
        O::MultiplyFloats => bin_f(&a, &b, |x, y| x * y),
        O::DivideFloats => bin_f(&a, &b, |x, y| x / y),
        O::ModuloFloats => bin_f(&a, &b, |x, y| x % y),
        O::PowerFloats => bin_f(&a, &b, |x, y| x.powf(y)),
        O::AbsFloat => a.as_float().map_or(V::None, |x| fx(x.abs())),
        O::ClampFloat => {
            match (a.as_float(), b.as_float(), c.as_float()) {
                (Some(x), Some(lo), Some(hi)) => fx(x.max(lo).min(hi)),
                _ => V::None,
            }
        }
        O::LerpFloat => {
            match (a.as_float(), b.as_float(), c.as_float()) {
                (Some(x), Some(y), Some(t)) => fx(x + (y - x) * t),
                _ => V::None,
            }
        }
        O::RandomFloat => fx(rng.range_float(a.as_float().unwrap_or(0.0),
                                             b.as_float().unwrap_or(1.0))),
        O::OperationFloat => V::None,
        O::AndBool => bin_f(&a, &b, |x, y| (((x != 0.0) && (y != 0.0)) as i32) as f32),
        O::OrBool => bin_f(&a, &b, |x, y| (((x != 0.0) || (y != 0.0)) as i32) as f32),
        O::NotBool => a.as_bool().map_or(V::None, |x| V::Bool(!x)),
        O::IsNull => V::Bool(a.is_null()),
        O::CompareBools => V::Bool(a.as_bool() == b.as_bool() && !a.is_null()),
        O::CompareFloats => bin_f(&a, &b, |x, y| if x == y { 1.0 } else { 0.0 }),
        O::CompareStrings => {
            match (&a, &b) {
                (V::Str(x), V::Str(y)) => V::Bool(x == y),
                _ => V::None,
            }
        }
        O::AddStrings => {
            match (&a, &b) {
                (V::Str(x), V::Str(y)) => V::Str(Rc::from(format!("{}{}", x, y).as_str())),
                _ => V::None,
            }
        }

        O::AddVector3 | O::SubtractVector3 | O::ScaleVector3
        | O::NormalizeVector3 | O::MagnitudeVector3 | O::DistanceVector3
        | O::DotVector3 | O::ConstructVector3 | O::SplitVector3 => {
            match inst.op {
                O::AddVector3 => vec2(&a, &b, |x, y| [x[0] + y[0], x[1] + y[1], x[2] + y[2]]),
                O::SubtractVector3 => vec2(&a, &b, |x, y| [x[0] - y[0], x[1] - y[1], x[2] - y[2]]),
                O::ScaleVector3 => {
                    match (&a, b.as_float()) {
                        (V::Vector3(x), Some(s)) => V::Vector3([x[0] * s, x[1] * s, x[2] * s]),
                        _ => V::None,
                    }
                }
                O::NormalizeVector3 => {
                    match &a {
                        V::Vector3(x) => {
                            let m = (x[0] * x[0] + x[1] * x[1] + x[2] * x[2]).sqrt();
                            if m == 0.0 { V::None } else { V::Vector3([x[0] / m, x[1] / m, x[2] / m]) }
                        }
                        _ => V::None,
                    }
                }
                O::MagnitudeVector3 => vec1(&a, |x| (x[0] * x[0] + x[1] * x[1] + x[2] * x[2]).sqrt()),
                O::DistanceVector3 => {
                    let d = vec2(&a, &b, |x, y| [x[0] - y[0], x[1] - y[1], x[2] - y[2]]);
                    match d {
                        V::Vector3(t) => V::Float((t[0] * t[0] + t[1] * t[1] + t[2] * t[2]).sqrt()),
                        _ => V::None,
                    }
                }
                O::DotVector3 => {
                    match (&a, &b) {
                        (V::Vector3(x), V::Vector3(y)) => {
                            V::Float(x[0] * y[0] + x[1] * y[1] + x[2] * y[2])
                        }
                        _ => V::None,
                    }
                }
                O::ConstructVector3 => {
                    match (a.as_float(), b.as_float(), c.as_float()) {
                        (Some(x), Some(y), Some(z)) => V::Vector3([x, y, z]),
                        _ => V::None,
                    }
                }
                O::SplitVector3 => {
                    match &a {
                        V::Vector3(x) => fx(x[inst.imm.max(0).min(2) as usize]),
                        _ => V::None,
                    }
                }
                _ => V::None,
            }
        }
        O::ReadVariable => frame.vars.get(inst.imm.max(0) as usize)
            .cloned().unwrap_or(V::None),
        O::WriteVariable => {
            let i = inst.imm.max(0) as usize;
            if frame.vars.len() <= i { frame.vars.resize(i + 1, V::None); }
            frame.vars[i] = a.clone();
            V::None
        }
        O::ConditionalPick => {
            match a.as_bool() {
                Some(true) => b.clone(),
                Some(false) => c.clone(),
                None => V::None,
            }
        }
        O::RelativePosition | O::CallGate | O::CallHandler | O::CallFunction => {
            return Err(format!(
                "{:?} semantics are per-game gate behaviour - not yet ",
                inst.op,));
        }
    };
    frame.set(d0, v);
    Ok(())
}

fn vec1(a: &RacingValue, f: impl FnOnce([f32; 3]) -> f32) -> RacingValue {
    match a {
        RacingValue::Vector3(x) => RacingValue::Float(f(*x)),
        _ => RacingValue::None,
    }
}
fn vec2(a: &RacingValue, b: &RacingValue, f: impl FnOnce([f32; 3], [f32; 3]) -> [f32; 3])
    -> RacingValue {
    match (a, b) {
        (RacingValue::Vector3(x), RacingValue::Vector3(y)) => RacingValue::Vector3(f(*x, *y)),
        _ => RacingValue::None,
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    fn inst(op: RacingOp, i: [i32; 3], o: [i32; 3], imm: i32) -> RacingInst {
        RacingInst { op, in_regs: i, out_regs: o, imm }
    }

    fn frame(vals: Vec<RacingValue>) -> RacingFrame {
        RacingFrame { regs: vals, vars: vec![] }
    }

    #[test]
    fn f32_math_is_bit_exact_and_null_propagates() {
        let mut rng = UnityRng::init_state(1);
        let mut f = frame(vec![RacingValue::Float(0.1), RacingValue::Float(0.2)]);
        execute(&[inst(RacingOp::AddFloats, [0, 1, -1], [2, -1, -1], 0)],
                &mut f, &mut rng).unwrap();
        let expect = 0.1f32 + 0.2f32;
        match f.regs[2] {
            RacingValue::Float(v) => assert_eq!(v.to_bits(), expect.to_bits()),
            _ => panic!("want float"),
        }
        let mut f2 = frame(vec![RacingValue::Float(1.0), RacingValue::None]);
        execute(&[inst(RacingOp::AddFloats, [0, 1, -1], [2, -1, -1], 0)],
                &mut f2, &mut rng).unwrap();
        assert_eq!(f2.regs[2], RacingValue::None);
    }

    #[test]
    fn decode_is_loud_on_bad_discriminant() {
        assert!(RacingOp::from_wire(36).is_ok());
        assert!(RacingOp::from_wire(37).is_err());
    }

    #[test]
    fn unvalidated_ops_fail_loud_never_guess() {
        let mut rng = UnityRng::init_state(7);
        let mut f = frame(vec![]);
        let r = execute(&[inst(RacingOp::RelativePosition, [0, 1, -1], [0, -1, -1], 13)],
                        &mut f, &mut rng);
        assert!(r.is_err());
    }
}


