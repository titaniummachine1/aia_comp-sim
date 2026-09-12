//! Demand-driven 1:1 lowerer — TeamGraph semantics match `graph::eval`.

use std::collections::{HashMap, HashSet};

use bevy::prelude::{Vec2, Vec3};

use crate::graph::pitch_vec::vec3_from_pitch;

use crate::graph::load::{GraphNode, TeamGraph};
use crate::graph_vm::context::VariableId;
use crate::graph_vm::ir::{IrInst, LoweredIR, Reg, LOWERED_IR_VERSION};
use crate::graph_vm::opcode::{ApiSlot, OpCode};
use crate::graph_vm::value::RegisterKind;

#[derive(Debug, Clone)]
pub struct VariableTable {
    pub names: Vec<String>,
    pub name_to_id: HashMap<String, VariableId>,
}

impl VariableTable {
    pub fn intern(&mut self, name: &str) -> VariableId {
        if let Some(&id) = self.name_to_id.get(name) {
            return id;
        }
        let id = VariableId(self.names.len() as u16);
        self.names.push(name.to_string());
        self.name_to_id.insert(name.to_string(), id);
        id
    }

    pub fn len(&self) -> usize {
        self.names.len()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ApiKind {
    Bool,
    Float,
    Transform,
    Vector3,
}

#[derive(Debug, Clone)]
pub struct ApiSlotTable {
    /// Which game this slot space interns into. `None` = pure VM: a game
    /// API exists to be interned against, so any `LoadApi` is an error.
    pub mode: Option<crate::mode::GameSpec>,
    pub labels: Vec<String>,
    pub kinds: Vec<ApiKind>,
    /// Game catalog index (`UNKNOWN_ID` if label not in the spec's catalog).
    pub dense_ids: Vec<u16>,
    label_to_slot: HashMap<(ApiKind, String), ApiSlot>,
}

impl ApiSlotTable {
    pub fn intern(&mut self, label: &str, kind: ApiKind) -> ApiSlot {
        let key = (kind, label.to_string());
        if let Some(&slot) = self.label_to_slot.get(&key) {
            return slot;
        }
        let Some(spec) = self.mode else {
            panic!(
                "pure VM has no game API: interning {kind:?} sensor {label:?} requires a \
                 GameSpec (simulation) — pure graphs must not touch mode-owned nodes"
            );
        };
        // v0.14/v0.15 runtime admission: labels the capture does not pin are
        // rejected loudly (no invented aliases, no legacy-index guessing).
        // v0.15's ABI is v0.14-identical (re-mine confirmed), so it shares the
        // table.
        if matches!(
            spec.version,
            crate::mode::GameVersion::TennisV014 | crate::mode::GameVersion::TennisV015
        ) {
            assert!(
                crate::graph::dropdowns::tennis_v014_admits(label),
                "label {label:?} is not admitted by the Tennis v0.14/v0.15 runtime capture \
                 (partial ABI — pin it in a capture or load with TennisV012)"
            );
        }
        let dense = match (spec.mode, kind) {
            (crate::mode::GameMode::Soccer, ApiKind::Bool) => {
                crate::api::bool_index(label).unwrap_or(crate::api::UNKNOWN_ID)
            }
            (crate::mode::GameMode::Soccer, ApiKind::Float) => {
                crate::api::float_index(label).unwrap_or(crate::api::UNKNOWN_ID)
            }
            (crate::mode::GameMode::Soccer, ApiKind::Transform) => {
                crate::api::transform_index(label).unwrap_or(crate::api::UNKNOWN_ID)
            }
            (crate::mode::GameMode::Soccer, ApiKind::Vector3) => {
                crate::api::vector_index(label).unwrap_or(crate::api::UNKNOWN_ID)
            }
            (crate::mode::GameMode::Tennis, ApiKind::Bool) => {
                crate::tennis::api::bool_index(label).unwrap_or(crate::api::UNKNOWN_ID)
            }
            (crate::mode::GameMode::Tennis, ApiKind::Float) => {
                crate::tennis::api::float_index(label).unwrap_or(crate::api::UNKNOWN_ID)
            }
            (crate::mode::GameMode::Tennis, ApiKind::Transform) => {
                crate::tennis::api::transform_index(label).unwrap_or(crate::api::UNKNOWN_ID)
            }
            (crate::mode::GameMode::Tennis, ApiKind::Vector3) => {
                crate::tennis::api::vector_index(label).unwrap_or(crate::api::UNKNOWN_ID)
            }
        };
        let idx = self.labels.len();
        self.labels.push(label.to_string());
        self.kinds.push(kind);
        self.dense_ids.push(dense);
        let slot = ApiSlot::new((idx + 1) as u16).expect("api slot");
        self.label_to_slot.insert(key, slot);
        slot
    }

    pub fn label(&self, slot: ApiSlot) -> &str {
        &self.labels[slot.get() as usize - 1]
    }

    pub fn kind(&self, slot: ApiSlot) -> ApiKind {
        self.kinds[slot.get() as usize - 1]
    }

    pub fn dense_id(&self, slot: ApiSlot) -> u16 {
        self.dense_ids
            .get(slot.get() as usize - 1)
            .copied()
            .unwrap_or(crate::api::UNKNOWN_ID)
    }
}

#[derive(Debug, Clone)]
pub struct CompileResult {
    pub settle: LoweredIR,
    pub controllers: LoweredIR,
    pub vars: VariableTable,
    pub apis: ApiSlotTable,
    pub set_variable_sids: Vec<String>,
    /// `ConstructSoccerProperties` faceoff spots, team space. Not code — the
    /// engine reads these when it places the teams, so they ride along beside
    /// the IR rather than through it.
    pub kickoff_positions: [Option<Vec2>; 4],
}

#[derive(Debug, Clone)]
struct CallFrame {
    create_sid: String,
    /// Function call node sid (for resolving Color args through params).
    call_sid: String,
    args: [Reg; 4],
    port_regs: HashMap<String, Reg>,
}

/// Recursion-depth cap for `lower_port`/`lower_node_output`.
///
/// Feedback cycles (ConditionalSet out → [Relay?] → false branch, old
/// ConditionalSetFloat self-loops, Set/Get memos) are **not** errors: Unity
/// treats the back-edge as previous-tick memory. Those are broken via
/// synthetic latch variables in [`Lowerer::lower_port`].
///
/// This cap remains only for pathological non-progress cases that somehow
/// avoid the in-stack check. 1500 is MEASURED — see historical notes in git;
/// on a 2 MB stack a naive recurse-without-break overflows near 3000.
/// graphc guards its own desc demand-chains at 800 (Windows 1 MB main thread
/// dies near 1540), so compiler-produced graphs never approach this cap —
/// the 800→1500 margin is headroom for hand-built Unity saves only.
/// Lowering itself always runs on a dedicated 8 MB stack (see
/// [`Lowerer::compile`]) so a pathological chain hits this loud guard
/// instead of killing the process with STATUS_STACK_OVERFLOW.
const MAX_LOWER_DEPTH: u32 = 1_500;

/// Stack reserved for the recursive lowering worker (see [`Lowerer::compile`]).
const LOWER_STACK_SIZE: usize = 8 << 20;

/// Set when a lowering hit [`MAX_LOWER_DEPTH`] without being broken as a latch.
///
/// Benign Unity memory cycles must not set this — they allocate a prev-tick
/// latch instead. If this flag is still raised, the graph has a runaway chain
/// we could not break; headless batch refuses those.
static RECURSION_LIMIT_HIT: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

/// Did any lowering hit the recursion cap? Clears the flag as it reads, so the
/// caller sees each occurrence once.
pub fn take_recursion_limit_hit() -> bool {
    RECURSION_LIMIT_HIT.swap(false, std::sync::atomic::Ordering::Relaxed)
}

#[derive(Debug)]
pub struct Lowerer {
    graph: TeamGraph,
    /// The game this graph runs in — `None` = pure VM (no game API attached;
    /// mode-owned nodes are rejected at compile). There is no implicit
    /// default: simulation requires an explicit [`GameSpec`].
    spec: Option<crate::mode::GameSpec>,
    next_reg: u32,
    port_regs: HashMap<String, Reg>,
    call_stack: Vec<CallFrame>,
    ir: Vec<IrInst>,
    vars: VariableTable,
    apis: ApiSlotTable,
    lower_depth: u32,
    /// Ports whose lowering is in progress (cycle detection).
    lowering_stack: HashSet<String>,
    /// Output ports that participate in a feedback latch → synthetic var id.
    latch_vars: HashMap<String, VariableId>,
}

impl Lowerer {
    /// Lower a graph in **pure VM** mode: no game attached, mode-owned nodes
    /// (`Soccer*` / `Tennis*`) are a hard error. Common nodes (math, vars,
    /// functions, vectors, conditionals) run — the VM-as-programming-language
    /// path for unit tests.
    pub fn compile_pure(graph: TeamGraph) -> CompileResult {
        Self::compile(graph, None)
    }

    /// Lower a graph for an explicit game spec. The spec is **mandatory for
    /// simulation** — the dense API catalog a label interns into is
    /// spec-owned, and a foreign-mode node must never silently resolve to
    /// another game's label.
    pub fn compile_for(graph: TeamGraph, spec: crate::mode::GameSpec) -> CompileResult {
        Self::compile(graph, Some(spec))
    }

    pub fn compile(graph: TeamGraph, spec: Option<crate::mode::GameSpec>) -> CompileResult {
        // The demand lowerer recurses on the call stack (`lower_port` →
        // `lower_node_output` → `lower_input` → `lower_port`). Run it on a
        // dedicated 8 MB stack so a pathological demand chain hits the loud
        // MAX_LOWER_DEPTH guard instead of overflowing a 1 MB viewer/headless
        // main thread or a 2 MB test thread (STATUS_STACK_OVERFLOW is
        // uncatchable — the process dies). Compile happens once per brain
        // load, never per tick, so one thread spawn per compile is noise.
        // Panics (e.g. spec-validation asserts) keep their loud semantics
        // via resume_unwind.
        let worker = std::thread::Builder::new()
            .name("graph-lower".into())
            .stack_size(LOWER_STACK_SIZE)
            .spawn(move || Self::compile_inner(graph, spec));
        match worker {
            Ok(handle) => match handle.join() {
                Ok(result) => result,
                Err(payload) => std::panic::resume_unwind(payload),
            },
            // Thread spawn failed (resource exhaustion) — fail loudly instead
            // of silently returning a wrong program.
            Err(_) => {
                panic!("graph lowerer: could not spawn 8 MB worker thread");
            }
        }
    }

    fn compile_inner(graph: TeamGraph, spec: Option<crate::mode::GameSpec>) -> CompileResult {
        if let Some(spec) = spec {
            let violations = crate::mode::validate_graph(spec, &graph);
            assert!(
                violations.is_empty(),
                "graph is not valid for {spec:?}:\n{}",
                violations
                    .iter()
                    .map(|(id, why)| format!("  {id}: {why}"))
                    .collect::<Vec<_>>()
                    .join("\n")
            );
        } else {
            let foreign: Vec<&str> = graph
                .nodes
                .values()
                .map(|n| n.id.as_str())
                .filter(|id| {
                    id.starts_with("Soccer") || id.starts_with("Tennis")
                })
                .collect();
            assert!(
                foreign.is_empty(),
                "pure VM has no game API: graph touches mode-owned nodes {:?}. \
                 Attach a GameSpec (simulation) or remove those nodes.",
                foreign
            );
        }
        let set_variable_sids = graph.set_variables.clone();
        let kickoff_positions = graph.kickoff_positions;
        let mut lowerer = Self {
            graph,
            spec,
            next_reg: 0,
            port_regs: HashMap::new(),
            call_stack: Vec::new(),
            ir: Vec::new(),
            vars: VariableTable {
                names: Vec::new(),
                name_to_id: HashMap::new(),
            },
            apis: ApiSlotTable {
                mode: None,
                labels: Vec::new(),
                kinds: Vec::new(),
                dense_ids: Vec::new(),
                label_to_slot: HashMap::new(),
            },
            lower_depth: 0,
            lowering_stack: HashSet::new(),
            latch_vars: HashMap::new(),
        };
        lowerer.apis.mode = lowerer.spec;

        for sid in &set_variable_sids {
            if let Some(node) = lowerer.graph.nodes.get(sid) {
                lowerer.vars.intern(&node.modifier);
            }
        }
        let mut variable_nodes: Vec<_> = lowerer
            .graph
            .nodes
            .values()
            .filter(|node| node.id == "SetVariable" || node.id == "GetVariable")
            .map(|node| (node.sid.clone(), node.modifier.clone()))
            .collect();
        variable_nodes.sort_by(|a, b| a.0.cmp(&b.0));
        for (_, name) in variable_nodes {
            lowerer.vars.intern(&name);
        }

        for sid in &set_variable_sids {
            lowerer.lower_set_variable(sid);
        }
        let settle = lowerer.take_ir();

        lowerer.port_regs.clear();
        // DebugDraw sinks once per think (controllers phase — settle runs 8×).
        let debug_draw_sids = lowerer.graph.debug_draws.clone();
        for sid in &debug_draw_sids {
            lowerer.lower_debug_draw(sid);
        }
        // Root Function side-effects (e.g. AIA Draw Player Debugs) — return unused.
        let root_fns = lowerer.graph.root_functions.clone();
        for sid in &root_fns {
            if let Some(node) = lowerer.graph.nodes.get(sid).cloned() {
                let _ = lowerer.lower_function_call(&node);
            }
        }
        let controllers_by_slot = lowerer.graph.controllers.clone();
        for (i, ctrl_sid) in controllers_by_slot.iter().enumerate() {
            if let Some(sid) = ctrl_sid {
                let is_tennis = lowerer
                    .graph
                    .nodes
                    .get(sid)
                    .map(|n| n.id == "TennisController")
                    .unwrap_or(false);
                if is_tennis {
                    lowerer.lower_tennis_controller(i, sid);
                } else {
                    lowerer.lower_controller(i, sid);
                }
            }
        }
        // Faceoff spots are graph OUTPUTS like a controller command is, not
        // constants: AIA derives them from TeamMultiplier and a "whose
        // kickoff" conditional, so they only exist once the graph has run.
        if let Some(props) = lowerer.properties_node_sid() {
            for slot in 0..4 {
                lowerer.lower_faceoff(slot, &props);
            }
        }
        let controllers = lowerer.take_ir();

        CompileResult {
            settle,
            controllers,
            vars: lowerer.vars,
            apis: lowerer.apis,
            set_variable_sids,
            kickoff_positions,
        }
    }

    fn take_ir(&mut self) -> LoweredIR {
        LoweredIR {
            ir_version: LOWERED_IR_VERSION,
            instructions: std::mem::take(&mut self.ir),
            block_succs: Vec::new(),
        }
    }

    fn fresh_reg(&mut self, kind: RegisterKind) -> Reg {
        let r = Reg(self.next_reg);
        self.next_reg += 1;
        let _ = kind;
        r
    }

    #[allow(dead_code)] // O0 helper; settle/controllers push IrInst directly.
    fn emit(&mut self, inst: IrInst) -> Reg {
        let dest = inst.dest;
        self.ir.push(inst);
        dest.expect("emit with dest")
    }

    fn lower_set_variable(&mut self, node_sid: &str) {
        let Some(node) = self.graph.nodes.get(node_sid).cloned() else {
            return;
        };
        if node.id != "SetVariable" {
            return;
        }
        let value = self
            .lower_input(node_sid, "Any1")
            .unwrap_or_else(|| self.emit_const_null(node_sid, "Any1"));
        let vid = self.vars.intern(&node.modifier);
        self.ir.push(IrInst {
            dest: None,
            kind: RegisterKind::Null,
            op: OpCode::StoreVar,
            args: vec![Reg(vid.0 as u32), value],
            immediates: vec![],
            source_sid: node_sid.to_string(),
            source_port: "Any1".to_string(),
        });
    }

    fn lower_controller(&mut self, slot: usize, node_sid: &str) {
        let move_to = self
            .lower_input(node_sid, "Vector31")
            .unwrap_or_else(|| self.emit_const_vec2(node_sid, "Vector31", Vec2::ZERO));
        let sprint = self
            .lower_input(node_sid, "Bool1")
            .unwrap_or_else(|| self.emit_const_bool(node_sid, "Bool1", false));
        let interact = self
            .lower_input(node_sid, "Bool2")
            .unwrap_or_else(|| self.emit_const_bool(node_sid, "Bool2", false));
        self.ir.push(IrInst {
            dest: None,
            kind: RegisterKind::Null,
            op: OpCode::EmitController,
            args: vec![move_to, sprint, interact],
            immediates: vec![slot as u32],
            source_sid: node_sid.to_string(),
            source_port: "controller".to_string(),
        });
    }

    /// `TennisController` ports: `Vector31` = move/aim target, `Bool1` =
    /// swing hold, `Float1` = shot type dropdown, `Bool2` = sprint
    /// (AIGamePyLibrary data.py port schema).
    ///
    /// The aim request is resolved separately through the
    /// `Controller <- TennisAutoMove(Vector32) <- TennisAutoAim` chain so
    /// the world can latch strikes from the aim path while moving from the
    /// movement path. Graphs without that chain (direct-wired controllers,
    /// unwired `Vector32`) yield no aim and the world falls back to the
    /// legacy move_or_aim behavior.
    fn lower_tennis_controller(&mut self, slot: usize, node_sid: &str) {
        let move_or_aim = self
            .lower_input(node_sid, "Vector31")
            .unwrap_or_else(|| self.emit_const_vec2(node_sid, "Vector31", Vec2::ZERO));
        let (aim, has_aim) = match self.aim_request_reg(node_sid) {
            Some(r) => (r, self.emit_const_bool(node_sid, "aim_present", true)),
            None => (
                move_or_aim,
                self.emit_const_bool(node_sid, "aim_present", false),
            ),
        };
        let swing = self
            .lower_input(node_sid, "Bool1")
            .unwrap_or_else(|| self.emit_const_bool(node_sid, "Bool1", false));
        let shot_type = self
            .lower_input(node_sid, "Float1")
            .unwrap_or_else(|| self.emit_const_float(node_sid, "Float1", 2.0));
        let sprint = self
            .lower_input(node_sid, "Bool2")
            .unwrap_or_else(|| self.emit_const_bool(node_sid, "Bool2", false));
        self.ir.push(IrInst {
            dest: None,
            kind: RegisterKind::Null,
            op: OpCode::EmitTennisController,
            args: vec![move_or_aim, swing, shot_type, sprint, aim, has_aim],
            immediates: vec![slot as u32],
            source_sid: node_sid.to_string(),
            source_port: "controller".to_string(),
        });
    }

    /// Demand-lower the aim request feeding a `TennisController`: walk
    /// `Controller(Vector31) <- TennisAutoMove(Vector32) <-
    /// TennisAutoAim(Vector31) <- request` and lower the request source.
    /// Returns `None` when the chain is absent (direct-wired controller or
    /// unwired `Vector32`) — the world then uses legacy move_or_aim aiming.
    fn aim_request_reg(&mut self, controller_sid: &str) -> Option<Reg> {
        let mut cur_sid = controller_sid.to_string();
        let mut cur_port = "Vector31";
        for _ in 0..8 {
            let in_sid = self.graph.input_port_sid(&cur_sid, cur_port)?;
            let src_out = self.graph.input_source.get(&in_sid)?.clone();
            let pref = self.graph.ports.get(&src_out)?.clone();
            let src_node = self.graph.nodes.get(&pref.node_sid)?.clone();
            match src_node.id.as_str() {
                "TennisAutoMove" => {
                    cur_sid = src_node.sid.clone();
                    cur_port = "Vector32";
                }
                "TennisAutoAim" => {
                    cur_sid = src_node.sid.clone();
                    cur_port = "Vector31";
                }
                _ => return Some(self.lower_port(&src_out)),
            }
        }
        None
    }

    fn properties_node_sid(&self) -> Option<String> {
        self.graph
            .nodes
            .values()
            .find(|n| n.id == "ConstructSoccerProperties" && n.owner_function_sid.is_empty())
            .map(|n| n.sid.clone())
    }

    /// Emit one faceoff spot. An unwired port emits nothing at all, which is
    /// what leaves that player on the engine default — distinct from a port
    /// wired to something that evaluates to zero, which really does mean the
    /// center spot.
    fn lower_faceoff(&mut self, slot: usize, props_sid: &str) {
        let port = ["Vector31", "Vector32", "Vector33", "Vector34"][slot];
        let Some(value) = self.lower_input(props_sid, port) else {
            return;
        };
        self.ir.push(IrInst {
            dest: None,
            kind: RegisterKind::Null,
            op: OpCode::EmitFaceoff,
            args: vec![value],
            immediates: vec![slot as u32],
            source_sid: props_sid.to_string(),
            source_port: port.to_string(),
        });
    }

    fn lower_debug_draw(&mut self, node_sid: &str) {
        let Some(node) = self.graph.nodes.get(node_sid).cloned() else {
            return;
        };
        let rgba = self.resolve_color_rgba(node_sid, "Color1");
        let imm = vec![
            rgba[0].to_bits(),
            rgba[1].to_bits(),
            rgba[2].to_bits(),
            rgba[3].to_bits(),
        ];
        match node.id.as_str() {
            "DebugDrawLine" => {
                let a = self
                    .lower_input(node_sid, "Vector31")
                    .unwrap_or_else(|| self.emit_const_vec2(node_sid, "Vector31", Vec2::ZERO));
                let b = self
                    .lower_input(node_sid, "Vector32")
                    .unwrap_or_else(|| self.emit_const_vec2(node_sid, "Vector32", Vec2::ZERO));
                let w = self
                    .lower_input(node_sid, "Float1")
                    .unwrap_or_else(|| self.emit_const_float(node_sid, "Float1", 1.0));
                self.ir.push(IrInst {
                    dest: None,
                    kind: RegisterKind::Null,
                    op: OpCode::DebugDrawLine,
                    args: vec![a, b, w],
                    immediates: imm,
                    source_sid: node_sid.to_string(),
                    source_port: "DebugDrawLine".to_string(),
                });
            }
            "TimePlot" => {
                // Channel name is a String constant; intern it so the id fits
                // an immediate and `RuntimeProgram::var_names` gives it back.
                let name = self.resolve_string_modifier(node_sid, "String1");
                let id = self.vars.intern(&name).0 as u32;
                let v = self
                    .lower_input(node_sid, "Float1")
                    .unwrap_or_else(|| self.emit_const_float(node_sid, "Float1", 0.0));
                self.ir.push(IrInst {
                    dest: None,
                    kind: RegisterKind::Null,
                    op: OpCode::TimePlot,
                    args: vec![v],
                    immediates: vec![id],
                    source_sid: node_sid.to_string(),
                    source_port: "TimePlot".to_string(),
                });
            }
            "DebugDrawDisc" => {
                let center = self
                    .lower_input(node_sid, "Vector31")
                    .unwrap_or_else(|| self.emit_const_vec2(node_sid, "Vector31", Vec2::ZERO));
                let radius = self
                    .lower_input(node_sid, "Float1")
                    .unwrap_or_else(|| self.emit_const_float(node_sid, "Float1", 1.0));
                let width = self
                    .lower_input(node_sid, "Float2")
                    .unwrap_or_else(|| self.emit_const_float(node_sid, "Float2", 1.0));
                self.ir.push(IrInst {
                    dest: None,
                    kind: RegisterKind::Null,
                    op: OpCode::DebugDrawDisc,
                    args: vec![center, radius, width],
                    immediates: imm,
                    source_sid: node_sid.to_string(),
                    source_port: "DebugDrawDisc".to_string(),
                });
            }
            _ => {}
        }
    }

    /// Walk Relay / CreateFunction params back to a String constant, the way
    /// `resolve_color_rgba` does for Color. Used for the TimePlot channel name.
    fn resolve_string_modifier(&self, node_sid: &str, port_name: &str) -> String {
        let Some(in_sid) = self.graph.input_port_sid(node_sid, port_name) else {
            return String::new();
        };
        self.resolve_string_from_port(&in_sid, 0)
    }

    fn resolve_string_from_port(&self, port_sid: &str, depth: u32) -> String {
        if depth > 12 {
            return String::new();
        }
        if let Some(src_out) = self.graph.input_source.get(port_sid) {
            return self.resolve_string_from_port(src_out, depth + 1);
        }
        let Some(pref) = self.graph.ports.get(port_sid) else {
            return String::new();
        };
        let Some(node) = self.graph.nodes.get(&pref.node_sid) else {
            return String::new();
        };
        match node.id.as_str() {
            "String" => node.modifier.clone(),
            "Relay" => match self.graph.input_port_sid(&node.sid, "Any1") {
                Some(in_sid) => self.resolve_string_from_port(&in_sid, depth + 1),
                None => String::new(),
            },
            _ => String::new(),
        }
    }

    fn resolve_color_rgba(&self, node_sid: &str, port_name: &str) -> [f32; 4] {
        let Some(in_sid) = self.graph.input_port_sid(node_sid, port_name) else {
            return crate::debug_draw::named_rgba("White");
        };
        self.resolve_color_from_port(&in_sid, 0)
    }

    /// Walk Relay / CreateFunction params back to a Color constant.
    fn resolve_color_from_port(&self, port_sid: &str, depth: u32) -> [f32; 4] {
        if depth > 12 {
            return crate::debug_draw::named_rgba("White");
        }
        // If this port is an input/relay, follow its source first.
        if let Some(src_out) = self.graph.input_source.get(port_sid) {
            return self.resolve_color_from_port(src_out, depth + 1);
        }
        let Some(pref) = self.graph.ports.get(port_sid) else {
            return crate::debug_draw::named_rgba("White");
        };
        let Some(node) = self.graph.nodes.get(&pref.node_sid) else {
            return crate::debug_draw::named_rgba("White");
        };
        match node.id.as_str() {
            "Color" => crate::debug_draw::named_rgba(&node.modifier),
            "Relay" => {
                if let Some(in_sid) = self.graph.input_port_sid(&node.sid, "Any1") {
                    self.resolve_color_from_port(&in_sid, depth + 1)
                } else {
                    crate::debug_draw::named_rgba("White")
                }
            }
            "CreateFunction" => {
                let idx = match pref.port_name.as_str() {
                    "Any1" => 0,
                    "Any2" => 1,
                    "Any3" => 2,
                    "Any4" => 3,
                    _ => return crate::debug_draw::named_rgba("White"),
                };
                let Some(frame) = self.call_stack.last() else {
                    return crate::debug_draw::named_rgba("White");
                };
                if frame.create_sid != node.sid {
                    return crate::debug_draw::named_rgba("White");
                }
                let arg_port = ["Any1", "Any2", "Any3", "Any4"][idx];
                if let Some(in_sid) = self.graph.input_port_sid(&frame.call_sid, arg_port) {
                    self.resolve_color_from_port(&in_sid, depth + 1)
                } else {
                    crate::debug_draw::named_rgba("White")
                }
            }
            _ => crate::debug_draw::named_rgba("White"),
        }
    }

    fn lower_input(&mut self, node_sid: &str, port_name: &str) -> Option<Reg> {
        let in_sid = self.graph.input_port_sid(node_sid, port_name)?;
        let src_out = self.graph.input_source.get(&in_sid)?.clone();
        Some(self.lower_port(&src_out))
    }

    fn cache_get(&self, port_sid: &str) -> Option<Reg> {
        if let Some(frame) = self.call_stack.last() {
            frame.port_regs.get(port_sid).copied()
        } else {
            self.port_regs.get(port_sid).copied()
        }
    }

    fn cache_set(&mut self, port_sid: String, reg: Reg) {
        if let Some(frame) = self.call_stack.last_mut() {
            frame.port_regs.insert(port_sid, reg);
        } else {
            self.port_regs.insert(port_sid, reg);
        }
    }

    fn lower_port(&mut self, port_sid: &str) -> Reg {
        if let Some(r) = self.cache_get(port_sid) {
            return r;
        }
        // Unity memory cell: if this output is already being computed higher
        // on the stack, the back-edge is previous-tick state — LoadVar latch,
        // do not recurse (Computer basics "Memory" / Zudan ConditionalSet latch).
        if self.lowering_stack.contains(port_sid) {
            return self.emit_load_latch(port_sid);
        }
        if self.lower_depth >= MAX_LOWER_DEPTH {
            // Safety net only — normal feedback cycles are broken above.
            RECURSION_LIMIT_HIT.store(true, std::sync::atomic::Ordering::Relaxed);
            return self.emit_const_null(port_sid, "depth-limit");
        }
        let Some(pref) = self.graph.ports.get(port_sid).cloned() else {
            return self.emit_const_null(port_sid, "missing");
        };
        self.lowering_stack.insert(port_sid.to_string());
        self.lower_depth += 1;
        let reg = self.lower_node_output(&pref.node_sid, &pref.port_name);
        self.lower_depth -= 1;
        self.lowering_stack.remove(port_sid);
        self.cache_set(port_sid.to_string(), reg);
        // Commit new value into the latch so the next tick / later back-edge
        // readers see what Unity holds as "previous output".
        if self.latch_vars.contains_key(port_sid) {
            self.emit_store_latch(port_sid, reg);
        }
        reg
    }

    fn latch_var_id(&mut self, port_sid: &str) -> VariableId {
        if let Some(&id) = self.latch_vars.get(port_sid) {
            return id;
        }
        let name = format!("__latch__{port_sid}");
        let id = self.vars.intern(&name);
        self.latch_vars.insert(port_sid.to_string(), id);
        id
    }

    fn emit_load_latch(&mut self, port_sid: &str) -> Reg {
        let vid = self.latch_var_id(port_sid);
        let dst = self.fresh_reg(RegisterKind::Null);
        self.ir.push(IrInst {
            dest: Some(dst),
            kind: RegisterKind::Null,
            op: OpCode::LoadVar,
            args: vec![Reg(vid.0 as u32)],
            immediates: vec![],
            source_sid: port_sid.to_string(),
            source_port: "latch".to_string(),
        });
        dst
    }

    fn emit_store_latch(&mut self, port_sid: &str, value: Reg) {
        let vid = self.latch_var_id(port_sid);
        self.ir.push(IrInst {
            dest: None,
            kind: RegisterKind::Null,
            op: OpCode::StoreVar,
            args: vec![Reg(vid.0 as u32), value],
            immediates: vec![],
            source_sid: port_sid.to_string(),
            source_port: "latch".to_string(),
        });
    }

    fn lower_node_output(&mut self, node_sid: &str, port_name: &str) -> Reg {
        let Some(node) = self.graph.nodes.get(node_sid).cloned() else {
            return self.emit_const_null(node_sid, port_name);
        };

        if node.id == "CreateFunction" {
            if let Some(frame) = self.call_stack.last() {
                if frame.create_sid == node.sid {
                    let idx = match port_name {
                        "Any1" => 0,
                        "Any2" => 1,
                        "Any3" => 2,
                        "Any4" => 3,
                        _ => return self.emit_const_null(node_sid, port_name),
                    };
                    let arg = frame.args[idx];
                    return self.emit_move(node_sid, port_name, arg, RegisterKind::Null);
                }
            }
            return self.emit_const_null(node_sid, port_name);
        }

        match node.id.as_str() {
            "Float" => self.emit_const_float(node_sid, port_name, parse_float(&node.modifier)),
            "Bool" => self.emit_const_bool(node_sid, port_name, node.modifier.trim() != "1"),
            "GetVariable" => {
                let vid = self.vars.intern(&node.modifier);
                let dst = self.fresh_reg(RegisterKind::Null);
                self.ir.push(IrInst {
                    dest: Some(dst),
                    kind: RegisterKind::Null,
                    op: OpCode::LoadVar,
                    args: vec![Reg(vid.0 as u32)],
                    immediates: vec![],
                    source_sid: node_sid.to_string(),
                    source_port: port_name.to_string(),
                });
                dst
            }
            "SetVariable" => {
                let value = self
                    .lower_input(node_sid, "Any1")
                    .unwrap_or_else(|| self.emit_const_null(node_sid, "Any1"));
                let vid = self.vars.intern(&node.modifier);
                self.ir.push(IrInst {
                    dest: None,
                    kind: RegisterKind::Null,
                    op: OpCode::StoreVar,
                    args: vec![Reg(vid.0 as u32), value],
                    immediates: vec![],
                    source_sid: node_sid.to_string(),
                    source_port: "Any1".to_string(),
                });
                self.emit_move(node_sid, port_name, value, RegisterKind::Null)
            }
            "Function" => {
                let value = self.lower_function_call(&node);
                self.emit_move(node_sid, port_name, value, RegisterKind::Null)
            }
            "SoccerGetBool" => {
                let slot = self.apis.intern(&node.modifier, ApiKind::Bool);
                self.emit_load_api(node_sid, port_name, slot, RegisterKind::Bool)
            }
            "SoccerGetFloat" => {
                let slot = self.apis.intern(&node.modifier, ApiKind::Float);
                self.emit_load_api(node_sid, port_name, slot, RegisterKind::Float)
            }
            "SoccerGetTransform" => {
                let slot = self.apis.intern(&node.modifier, ApiKind::Transform);
                self.emit_load_api(node_sid, port_name, slot, RegisterKind::Vector)
            }
            "SoccerGetVector3" => {
                let slot = self.apis.intern(&node.modifier, ApiKind::Vector3);
                self.emit_load_api(node_sid, port_name, slot, RegisterKind::Vector)
            }
            "TennisGetBool" => {
                let slot = self.apis.intern(&node.modifier, ApiKind::Bool);
                self.emit_load_api(node_sid, port_name, slot, RegisterKind::Bool)
            }
            "TennisGetFloat" => {
                let slot = self.apis.intern(&node.modifier, ApiKind::Float);
                self.emit_load_api(node_sid, port_name, slot, RegisterKind::Float)
            }
            "TennisGetTransform" => {
                let slot = self.apis.intern(&node.modifier, ApiKind::Transform);
                self.emit_load_api(node_sid, port_name, slot, RegisterKind::Vector)
            }
            "TennisGetVector3" => {
                let slot = self.apis.intern(&node.modifier, ApiKind::Vector3);
                self.emit_load_api(node_sid, port_name, slot, RegisterKind::Vector)
            }
            // TennisAuto* helper gates: documented approximations. AutoMove /
            // AutoAim pass their target through — the tennis world clamps
            // movement and aim to legal ground, which is where the game's own
            // clamp actually lives. AutoSwing decides the swing from the
            // swing-range / must-wait sensors and defaults to a flat shot.
            "TennisAutoMove" | "TennisAutoAim" => {
                crate::graph_vm::diagnostics::record_approximated(&node.id);
                let v = self
                    .lower_input(node_sid, "Vector31")
                    .unwrap_or_else(|| self.emit_const_vec2(node_sid, "Vector31", Vec2::ZERO));
                self.emit_move(node_sid, port_name, v, RegisterKind::Vector)
            }
            "TennisAutoSwing" => {
                crate::graph_vm::diagnostics::record_approximated("TennisAutoSwing");
                match port_name {
                    "Bool1" => {
                        let in_range = self
                            .apis
                            .intern("Ball In Swing Range", ApiKind::Bool);
                        let r = self.emit_load_api(node_sid, "Ball In Swing Range", in_range, RegisterKind::Bool);
                        let must_wait = self.apis.intern("Must Wait For Bounce", ApiKind::Bool);
                        let w = self.emit_load_api(node_sid, "Must Wait For Bounce", must_wait, RegisterKind::Bool);
                        let not_wait = self.fresh_reg(RegisterKind::Bool);
                        self.ir.push(IrInst {
                            dest: Some(not_wait),
                            kind: RegisterKind::Bool,
                            op: OpCode::Not,
                            args: vec![w],
                            immediates: vec![],
                            source_sid: node_sid.to_string(),
                            source_port: "swing-gate".to_string(),
                        });
                        let dst = self.fresh_reg(RegisterKind::Bool);
                        self.ir.push(IrInst {
                            dest: Some(dst),
                            kind: RegisterKind::Bool,
                            op: OpCode::And,
                            args: vec![r, not_wait],
                            immediates: vec![],
                            source_sid: node_sid.to_string(),
                            source_port: port_name.to_string(),
                        });
                        dst
                    }
                    "Float1" => self.emit_const_float(node_sid, port_name, 2.0),
                    _ => self.emit_const_null(node_sid, port_name),
                }
            }
            "RelativePosition" => {
                use crate::graph::dropdowns::{relative_position_mode, RelativePosMode};
                match relative_position_mode(&node.modifier) {
                    RelativePosMode::WorldPos => {
                        let pos = self.lower_input(node_sid, "Transform1").unwrap_or_else(|| {
                            self.emit_const_vec2(node_sid, "Transform1", Vec2::ZERO)
                        });
                        self.emit_move(node_sid, port_name, pos, RegisterKind::Vector)
                    }
                    RelativePosMode::DirOnly(d) => self.emit_const_vec2(node_sid, port_name, d),
                    RelativePosMode::PosPlus(d) => {
                        let pos = self.lower_input(node_sid, "Transform1").unwrap_or_else(|| {
                            self.emit_const_vec2(node_sid, "Transform1", Vec2::ZERO)
                        });
                        let offset = self.emit_const_vec2(node_sid, "RelOffset", d);
                        let dst = self.fresh_reg(RegisterKind::Vector);
                        self.ir.push(IrInst {
                            dest: Some(dst),
                            kind: RegisterKind::Vector,
                            op: OpCode::AddVec,
                            args: vec![pos, offset],
                            immediates: vec![],
                            source_sid: node_sid.to_string(),
                            source_port: port_name.to_string(),
                        });
                        dst
                    }
                }
            }
            "ConstructVector3" => {
                let x = self
                    .lower_input(node_sid, "Float1")
                    .unwrap_or_else(|| self.emit_const_float(node_sid, "Float1", 0.0));
                let y = self
                    .lower_input(node_sid, "Float2")
                    .unwrap_or_else(|| self.emit_const_float(node_sid, "Float2", 0.0));
                let z = self
                    .lower_input(node_sid, "Float3")
                    .unwrap_or_else(|| self.emit_const_float(node_sid, "Float3", 0.0));
                let dst = self.fresh_reg(RegisterKind::Vector);
                self.ir.push(IrInst {
                    dest: Some(dst),
                    kind: RegisterKind::Vector,
                    op: OpCode::ConstructVec,
                    args: vec![x, y, z],
                    immediates: vec![],
                    source_sid: node_sid.to_string(),
                    source_port: port_name.to_string(),
                });
                dst
            }
            "Vector3Split" => {
                let v = self
                    .lower_input(node_sid, "Vector31")
                    .unwrap_or_else(|| self.emit_const_vec2(node_sid, "Vector31", Vec2::ZERO));
                let axis = match port_name {
                    "Float1" => 0,
                    "Float2" => 1,
                    "Float3" => 2,
                    _ => return self.emit_const_null(node_sid, port_name),
                };
                let dst = self.fresh_reg(RegisterKind::Float);
                self.ir.push(IrInst {
                    dest: Some(dst),
                    kind: RegisterKind::Float,
                    op: OpCode::SplitVec,
                    args: vec![v],
                    immediates: vec![axis],
                    source_sid: node_sid.to_string(),
                    source_port: port_name.to_string(),
                });
                dst
            }
            "AddFloats" => self.bin_f(node_sid, port_name, OpCode::Add),
            "SubtractFloats" => self.bin_f(node_sid, port_name, OpCode::Sub),
            "MultiplyFloats" => self.bin_f(node_sid, port_name, OpCode::Mul),
            "DivideFloats" => self.bin_f(node_sid, port_name, OpCode::Div),
            // clamp(value, min, max). Was hitting the unknown-node fallback and
            // evaluating to Null, which silently invalidated every result for
            // any graph using it (Haialand-v2 does).
            "ClampFloat" => {
                let v = self
                    .lower_input(node_sid, "Float1")
                    .unwrap_or_else(|| self.emit_const_float(node_sid, "Float1", 0.0));
                let lo = self
                    .lower_input(node_sid, "Float2")
                    .unwrap_or_else(|| self.emit_const_float(node_sid, "Float2", 0.0));
                let hi = self
                    .lower_input(node_sid, "Float3")
                    .unwrap_or_else(|| self.emit_const_float(node_sid, "Float3", 0.0));
                let dst = self.fresh_reg(RegisterKind::Float);
                self.ir.push(IrInst {
                    dest: Some(dst),
                    kind: RegisterKind::Float,
                    op: OpCode::Clamp,
                    args: vec![v, lo, hi],
                    immediates: vec![],
                    source_sid: node_sid.to_string(),
                    source_port: port_name.to_string(),
                });
                dst
            }
            "Power" => self.bin_f(node_sid, port_name, OpCode::Pow),
            "Modulo" => self.bin_f(node_sid, port_name, OpCode::Mod),
            "Lerp" => {
                let a = self
                    .lower_input(node_sid, "Float1")
                    .unwrap_or_else(|| self.emit_const_float(node_sid, "Float1", 0.0));
                let b = self
                    .lower_input(node_sid, "Float2")
                    .unwrap_or_else(|| self.emit_const_float(node_sid, "Float2", 0.0));
                let t = self
                    .lower_input(node_sid, "Float3")
                    .unwrap_or_else(|| self.emit_const_float(node_sid, "Float3", 0.0));
                let dst = self.fresh_reg(RegisterKind::Float);
                self.ir.push(IrInst {
                    dest: Some(dst),
                    kind: RegisterKind::Float,
                    op: OpCode::Lerp,
                    args: vec![a, b, t],
                    immediates: vec![],
                    source_sid: node_sid.to_string(),
                    source_port: port_name.to_string(),
                });
                dst
            }
            "Relay" => {
                let value = self
                    .lower_input(node_sid, "Any1")
                    .unwrap_or_else(|| self.emit_const_null(node_sid, "Any1"));
                self.emit_move(node_sid, port_name, value, RegisterKind::Null)
            }
            "Operation" | "AbsFloat" | "Absolute"
                if !node.modifier.trim().is_empty() => {
                let a = self
                    .lower_input(node_sid, "Float1")
                    .unwrap_or_else(|| self.emit_const_float(node_sid, "Float1", 0.0));
                let (opcode, op) = if node.id == "Operation" {
                    let kind =
                        crate::graph::dropdowns::OperationKind::from_modifier(&node.modifier);
                    (OpCode::Operation, kind.as_u32())
                } else {
                    (OpCode::Abs, 0)
                };
                let dst = self.fresh_reg(RegisterKind::Float);
                self.ir.push(IrInst {
                    dest: Some(dst),
                    kind: RegisterKind::Float,
                    op: opcode,
                    args: vec![a],
                    immediates: if opcode == OpCode::Operation {
                        vec![op]
                    } else {
                        vec![]
                    },
                    source_sid: node_sid.to_string(),
                    source_port: port_name.to_string(),
                });
                dst
            }
            "AddVector3" => self.bin_v(node_sid, port_name, OpCode::AddVec),
            "SubtractVector3" => self.bin_v(node_sid, port_name, OpCode::SubVec),
            "ScaleVector3" => {
                let v = self
                    .lower_input(node_sid, "Vector31")
                    .unwrap_or_else(|| self.emit_const_vec2(node_sid, "Vector31", Vec2::ZERO));
                let s = self
                    .lower_input(node_sid, "Float1")
                    .unwrap_or_else(|| self.emit_const_float(node_sid, "Float1", 1.0));
                let dst = self.fresh_reg(RegisterKind::Vector);
                self.ir.push(IrInst {
                    dest: Some(dst),
                    kind: RegisterKind::Vector,
                    op: OpCode::ScaleVec,
                    args: vec![v, s],
                    immediates: vec![],
                    source_sid: node_sid.to_string(),
                    source_port: port_name.to_string(),
                });
                dst
            }
            "Normalize" => self.unary_v(node_sid, port_name, OpCode::Normalize),
            "Magnitude" => self.unary_v(node_sid, port_name, OpCode::Magnitude),
            "Distance" => self.bin_v(node_sid, port_name, OpCode::Distance),
            "DotProduct" => self.bin_v(node_sid, port_name, OpCode::Dot),
            "CrossProduct" => self.bin_v(node_sid, port_name, OpCode::Cross),
            "RandomFloat" => {
                let dst = self.fresh_reg(RegisterKind::Float);
                self.ir.push(IrInst {
                    dest: Some(dst),
                    kind: RegisterKind::Float,
                    op: OpCode::RandomF,
                    args: vec![],
                    immediates: vec![],
                    source_sid: node_sid.to_string(),
                    source_port: port_name.to_string(),
                });
                dst
            }
            "Not" => {
                let b = self
                    .lower_input(node_sid, "Bool1")
                    .unwrap_or_else(|| self.emit_const_bool(node_sid, "Bool1", false));
                let dst = self.fresh_reg(RegisterKind::Bool);
                self.ir.push(IrInst {
                    dest: Some(dst),
                    kind: RegisterKind::Bool,
                    op: OpCode::Not,
                    args: vec![b],
                    immediates: vec![],
                    source_sid: node_sid.to_string(),
                    source_port: port_name.to_string(),
                });
                dst
            }
            "CompareFloats" => {
                let a = self
                    .lower_input(node_sid, "Float1")
                    .unwrap_or_else(|| self.emit_const_float(node_sid, "Float1", 0.0));
                let b = self
                    .lower_input(node_sid, "Float2")
                    .unwrap_or_else(|| self.emit_const_float(node_sid, "Float2", 0.0));
                let op = node.modifier.parse::<i32>().unwrap_or(0);
                let (opcode, imm) = compare_float_op(op);
                let dst = self.fresh_reg(RegisterKind::Bool);
                self.ir.push(IrInst {
                    dest: Some(dst),
                    kind: RegisterKind::Bool,
                    op: opcode,
                    args: vec![a, b],
                    immediates: vec![imm],
                    source_sid: node_sid.to_string(),
                    source_port: port_name.to_string(),
                });
                dst
            }
            "CompareBool" => {
                let a = self
                    .lower_input(node_sid, "Bool1")
                    .unwrap_or_else(|| self.emit_const_bool(node_sid, "Bool1", false));
                let b = self
                    .lower_input(node_sid, "Bool2")
                    .unwrap_or_else(|| self.emit_const_bool(node_sid, "Bool2", false));
                let op = node.modifier.parse::<i32>().unwrap_or(0);
                let opcode = match op {
                    1 => OpCode::Or,
                    2 | 6 => OpCode::Eq,
                    3 => OpCode::Ne,
                    4 => OpCode::Nor,
                    5 => OpCode::Nand,
                    _ => OpCode::And,
                };
                let dst = self.fresh_reg(RegisterKind::Bool);
                self.ir.push(IrInst {
                    dest: Some(dst),
                    kind: RegisterKind::Bool,
                    op: opcode,
                    args: vec![a, b],
                    immediates: vec![],
                    source_sid: node_sid.to_string(),
                    source_port: port_name.to_string(),
                });
                dst
            }
            "ConditionalSetBool" => self.select(node_sid, port_name, RegisterKind::Bool),
            "ConditionalSetFloatV2" | "ConditionalSetFloat" => {
                self.select(node_sid, port_name, RegisterKind::Float)
            }
            "ConditionalSetVector3" => self.select(node_sid, port_name, RegisterKind::Vector),
            "IsNull" => {
                let v = self
                    .lower_input(node_sid, "Any1")
                    .or_else(|| self.lower_input(node_sid, "Vector31"))
                    .unwrap_or_else(|| self.emit_const_null(node_sid, "Any1"));
                let dst = self.fresh_reg(RegisterKind::Bool);
                self.ir.push(IrInst {
                    dest: Some(dst),
                    kind: RegisterKind::Bool,
                    op: OpCode::IsNull,
                    args: vec![v],
                    immediates: vec![],
                    source_sid: node_sid.to_string(),
                    source_port: port_name.to_string(),
                });
                dst
            }
            // Runtime Keypress — viewer fills [`crate::keypress`]; headless stays false.
            "Keypress" => {
                let kid = crate::keypress::key_id(&node.modifier);
                let dst = self.fresh_reg(RegisterKind::Bool);
                self.ir.push(IrInst {
                    dest: Some(dst),
                    kind: RegisterKind::Bool,
                    op: OpCode::Keypress,
                    args: vec![],
                    immediates: vec![kid],
                    source_sid: node_sid.to_string(),
                    source_port: port_name.to_string(),
                });
                dst
            }
            // Color is a named constant (White/Cyan/…); keep as non-null so
            // consumers (TimePlot/DebugDraw) can lower inputs. Stored as float 0
            // — color name only matters for Unity viz.
            "Color" => self.emit_const_float(node_sid, port_name, 0.0),
            // Side-effect / viz / faceoff stubs: evaluate as null outputs.
            "Debug"
            | "DebugDrawDisc"
            | "DebugDrawLine"
            | "TimePlot"
            | "Region"
            | "ConstructSoccerProperties"
            | "ConstructTennisProperties"
            | "Spherecast"
            | "Country"
            | "Stat"
            | "RandomColor"
            | "ConditionalSetString" => self.emit_const_null(node_sid, port_name),
            // ANY node type we do not implement lands here and silently
            // evaluates to Null. That is the most dangerous failure mode in
            // the whole VM: the graph loads, the match runs, results look
            // plausible, and a decision was quietly made on nothing. Record
            // it so callers can report it loudly instead of inferring later
            // from a scoreline that felt wrong.
            // Game-tolerated degenerate: Operation with an empty modifier
            // (7 mined graphs). The game runs it — treat as constant 0 with
            // an approximation record rather than a lowering panic.
            "Operation" | "AbsFloat" | "Absolute" => {
                crate::graph_vm::diagnostics::record_approximated("Operation(empty)");
                self.emit_const_float(node_sid, port_name, 0.0)
            }
            other => {
                crate::graph_vm::diagnostics::record_unimplemented(other);
                self.emit_const_null(node_sid, port_name)
            }
        }
    }

    fn select(&mut self, node_sid: &str, port_name: &str, kind: RegisterKind) -> Reg {
        let cond = self
            .lower_input(node_sid, "Bool1")
            .unwrap_or_else(|| self.emit_const_bool(node_sid, "Bool1", false));
        let t = match kind {
            RegisterKind::Bool => self
                .lower_input(node_sid, "Bool2")
                .unwrap_or_else(|| self.emit_const_bool(node_sid, "Bool2", false)),
            RegisterKind::Float => self
                .lower_input(node_sid, "Float1")
                .unwrap_or_else(|| self.emit_const_float(node_sid, "Float1", 0.0)),
            RegisterKind::Vector => self
                .lower_input(node_sid, "Vector31")
                .unwrap_or_else(|| self.emit_const_vec2(node_sid, "Vector31", Vec2::ZERO)),
            RegisterKind::Null => self.emit_const_null(node_sid, port_name),
        };
        // False branch: V2 wires it explicitly (often back to self via Relay).
        // Old ConditionalSetFloat has only Float1 — missing false means
        // implicit previous-tick hold (Computer basics "Memory").
        let f = match kind {
            RegisterKind::Bool => self.lower_input(node_sid, "Bool3").unwrap_or_else(|| {
                self.implicit_hold(node_sid, port_name, RegisterKind::Bool)
            }),
            RegisterKind::Float => self.lower_input(node_sid, "Float2").unwrap_or_else(|| {
                self.implicit_hold(node_sid, port_name, RegisterKind::Float)
            }),
            RegisterKind::Vector => self
                .lower_input(node_sid, "Vector32")
                .unwrap_or_else(|| self.implicit_hold(node_sid, port_name, RegisterKind::Vector)),
            RegisterKind::Null => self.emit_const_null(node_sid, port_name),
        };
        let dst = self.fresh_reg(kind);
        // GraphBrain ConditionalSet* always coerces via as_bool/as_float/as_vec.
        let kind_imm = match kind {
            RegisterKind::Float => 0,
            RegisterKind::Bool => 1,
            RegisterKind::Vector => 2,
            RegisterKind::Null => 3,
        };
        self.ir.push(IrInst {
            dest: Some(dst),
            kind,
            op: OpCode::Select,
            args: vec![cond, t, f],
            immediates: vec![kind_imm],
            source_sid: node_sid.to_string(),
            source_port: port_name.to_string(),
        });
        dst
    }

    /// Previous-tick value of this node's output (Unity hold when false unwired).
    fn implicit_hold(&mut self, node_sid: &str, port_name: &str, kind: RegisterKind) -> Reg {
        if let Some(out_sid) = self.graph.output_port_sid(node_sid, port_name) {
            let loaded = self.emit_load_latch(&out_sid);
            // LoadVar is untyped Null; Move into the Select's expected kind.
            if kind == RegisterKind::Null {
                loaded
            } else {
                self.emit_move(node_sid, "hold", loaded, kind)
            }
        } else {
            match kind {
                RegisterKind::Bool => self.emit_const_bool(node_sid, "hold", false),
                RegisterKind::Float => self.emit_const_float(node_sid, "hold", 0.0),
                RegisterKind::Vector => self.emit_const_vec2(node_sid, "hold", Vec2::ZERO),
                RegisterKind::Null => self.emit_const_null(node_sid, "hold"),
            }
        }
    }

    fn lower_function_call(&mut self, fn_node: &GraphNode) -> Reg {
        // Unity AIComp v0.63+ allows nested custom Function calls. Soft-cap
        // depth so runaway recursion cannot blow the stack / IR size.
        if self.call_stack.len() > 64 {
            return self.emit_const_null(&fn_node.sid, "Any1");
        }
        let Some(def) = self.graph.create_functions.get(&fn_node.modifier).cloned() else {
            return self.emit_const_null(&fn_node.sid, "Any1");
        };
        let args = [
            self.lower_input(&fn_node.sid, "Any1")
                .unwrap_or_else(|| self.emit_const_null(&fn_node.sid, "Any1")),
            self.lower_input(&fn_node.sid, "Any2")
                .unwrap_or_else(|| self.emit_const_null(&fn_node.sid, "Any2")),
            self.lower_input(&fn_node.sid, "Any3")
                .unwrap_or_else(|| self.emit_const_null(&fn_node.sid, "Any3")),
            self.lower_input(&fn_node.sid, "Any4")
                .unwrap_or_else(|| self.emit_const_null(&fn_node.sid, "Any4")),
        ];
        self.call_stack.push(CallFrame {
            create_sid: def.sid.clone(),
            call_sid: fn_node.sid.clone(),
            args,
            port_regs: HashMap::new(),
        });
        // Unity runs DebugDraw sinks inside the function body each call.
        let body_draws = def.debug_draws.clone();
        for sid in &body_draws {
            self.lower_debug_draw(sid);
        }
        let ret = self
            .lower_input(&def.sid, "Any1")
            .unwrap_or_else(|| self.emit_const_null(&def.sid, "Any1"));
        self.call_stack.pop();
        ret
    }

    fn bin_f(&mut self, node_sid: &str, port_name: &str, op: OpCode) -> Reg {
        let a = self
            .lower_input(node_sid, "Float1")
            .unwrap_or_else(|| self.emit_const_float(node_sid, "Float1", 0.0));
        let b = self
            .lower_input(node_sid, "Float2")
            .unwrap_or_else(|| self.emit_const_float(node_sid, "Float2", 0.0));
        let dst = self.fresh_reg(RegisterKind::Float);
        self.ir.push(IrInst {
            dest: Some(dst),
            kind: RegisterKind::Float,
            op,
            args: vec![a, b],
            immediates: vec![],
            source_sid: node_sid.to_string(),
            source_port: port_name.to_string(),
        });
        dst
    }

    fn bin_v(&mut self, node_sid: &str, port_name: &str, op: OpCode) -> Reg {
        let a = self
            .lower_input(node_sid, "Vector31")
            .unwrap_or_else(|| self.emit_const_vec2(node_sid, "Vector31", Vec2::ZERO));
        let b = self
            .lower_input(node_sid, "Vector32")
            .unwrap_or_else(|| self.emit_const_vec2(node_sid, "Vector32", Vec2::ZERO));
        let kind = match op {
            OpCode::Dot | OpCode::Distance => RegisterKind::Float,
            _ => RegisterKind::Vector,
        };
        let dst = self.fresh_reg(kind);
        self.ir.push(IrInst {
            dest: Some(dst),
            kind,
            op,
            args: vec![a, b],
            immediates: vec![],
            source_sid: node_sid.to_string(),
            source_port: port_name.to_string(),
        });
        dst
    }

    fn unary_v(&mut self, node_sid: &str, port_name: &str, op: OpCode) -> Reg {
        let v = self
            .lower_input(node_sid, "Vector31")
            .unwrap_or_else(|| self.emit_const_vec2(node_sid, "Vector31", Vec2::ZERO));
        let kind = match op {
            OpCode::Magnitude => RegisterKind::Float,
            _ => RegisterKind::Vector,
        };
        let dst = self.fresh_reg(kind);
        self.ir.push(IrInst {
            dest: Some(dst),
            kind,
            op,
            args: vec![v],
            immediates: vec![],
            source_sid: node_sid.to_string(),
            source_port: port_name.to_string(),
        });
        dst
    }

    fn emit_move(
        &mut self,
        node_sid: &str,
        port_name: &str,
        value: Reg,
        kind: RegisterKind,
    ) -> Reg {
        let dst = self.fresh_reg(kind);
        self.ir.push(IrInst {
            dest: Some(dst),
            kind,
            op: OpCode::Move,
            args: vec![value],
            immediates: vec![],
            source_sid: node_sid.to_string(),
            source_port: port_name.to_string(),
        });
        dst
    }

    fn emit_load_api(
        &mut self,
        node_sid: &str,
        port_name: &str,
        slot: ApiSlot,
        kind: RegisterKind,
    ) -> Reg {
        let dst = self.fresh_reg(kind);
        self.ir.push(IrInst {
            dest: Some(dst),
            kind,
            op: OpCode::LoadApi,
            args: vec![],
            immediates: vec![slot.get() as u32],
            source_sid: node_sid.to_string(),
            source_port: port_name.to_string(),
        });
        dst
    }

    fn emit_const_float(&mut self, node_sid: &str, port_name: &str, f: f32) -> Reg {
        let dst = self.fresh_reg(RegisterKind::Float);
        self.ir.push(IrInst {
            dest: Some(dst),
            kind: RegisterKind::Float,
            op: OpCode::ConstFloat,
            args: vec![],
            immediates: vec![f.to_bits()],
            source_sid: node_sid.to_string(),
            source_port: port_name.to_string(),
        });
        dst
    }

    fn emit_const_bool(&mut self, node_sid: &str, port_name: &str, b: bool) -> Reg {
        let dst = self.fresh_reg(RegisterKind::Bool);
        self.ir.push(IrInst {
            dest: Some(dst),
            kind: RegisterKind::Bool,
            op: OpCode::ConstBool,
            args: vec![],
            immediates: vec![if b { 1 } else { 0 }],
            source_sid: node_sid.to_string(),
            source_port: port_name.to_string(),
        });
        dst
    }

    fn emit_const_vec2(&mut self, node_sid: &str, port_name: &str, v: Vec2) -> Reg {
        self.emit_const_vec3(node_sid, port_name, vec3_from_pitch(v))
    }

    fn emit_const_vec3(&mut self, node_sid: &str, port_name: &str, v: Vec3) -> Reg {
        let dst = self.fresh_reg(RegisterKind::Vector);
        self.ir.push(IrInst {
            dest: Some(dst),
            kind: RegisterKind::Vector,
            op: OpCode::ConstVec,
            args: vec![],
            immediates: vec![v.x.to_bits(), v.y.to_bits(), v.z.to_bits()],
            source_sid: node_sid.to_string(),
            source_port: port_name.to_string(),
        });
        dst
    }

    fn emit_const_null(&mut self, node_sid: &str, port_name: &str) -> Reg {
        let dst = self.fresh_reg(RegisterKind::Null);
        self.ir.push(IrInst {
            dest: Some(dst),
            kind: RegisterKind::Null,
            op: OpCode::ConstNull,
            args: vec![],
            immediates: vec![],
            source_sid: node_sid.to_string(),
            source_port: port_name.to_string(),
        });
        dst
    }
}

fn parse_float(s: &str) -> f32 {
    let t = s.trim().replace(',', ".");
    t.parse().unwrap_or(0.0)
}

fn compare_float_op(op: i32) -> (OpCode, u32) {
    match op {
        1 => (OpCode::Lt, 0),
        2 => (OpCode::Gt, 0),
        3 => (OpCode::Le, 0),
        4 => (OpCode::Ge, 0),
        _ => (OpCode::Eq, 1),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::load::{index_graph, RawConnection, RawGraph, RawNode, RawPort};
    use crate::graph_vm::builder::ProgramBuilder;
    use crate::graph_vm::context::ExecutionContext;
    use crate::graph_vm::interpreter::Interpreter;
    use crate::graph_vm::program::Backend;

    fn port(id: &str, sid: &str, pol: i32, node: &str) -> RawPort {
        RawPort {
            id: id.into(),
            sid: sid.into(),
            polarity: pol,
            node_sid: node.into(),
        }
    }

    fn node(id: &str, sid: &str, modifier: &str, ports: Vec<RawPort>) -> RawNode {
        RawNode {
            id: id.into(),
            sid: sid.into(),
            modifier: serde_json::json!(modifier),
            owner_function_sid: String::new(),
            ports,
        }
    }

    #[test]
    fn hand_built_add_floats_lower_and_run() {
        let raw = RawGraph {
            nodes: vec![
                node("Float", "fa", "3", vec![port("Float1", "fao", 1, "fa")]),
                node("Float", "fb", "4", vec![port("Float1", "fbo", 1, "fb")]),
                node(
                    "AddFloats",
                    "add",
                    "",
                    vec![
                        port("Float1", "add_a", 0, "add"),
                        port("Float2", "add_b", 0, "add"),
                        port("Float1", "add_o", 1, "add"),
                    ],
                ),
                node("Float", "z0", "0", vec![port("Float1", "z0o", 1, "z0")]),
                node(
                    "ConstructVector3",
                    "cv",
                    "",
                    vec![
                        port("Vector31", "cvo", 1, "cv"),
                        port("Float1", "cvx", 0, "cv"),
                        port("Float2", "cvy", 0, "cv"),
                        port("Float3", "cvz", 0, "cv"),
                    ],
                ),
                node("Bool", "bf", "1", vec![port("Bool1", "bfo", 1, "bf")]),
                node(
                    "SoccerController1",
                    "c1",
                    "",
                    vec![
                        port("Vector31", "c1m", 0, "c1"),
                        port("Bool1", "c1s", 0, "c1"),
                        port("Bool2", "c1i", 0, "c1"),
                    ],
                ),
            ],
            connections: vec![
                RawConnection {
                    port0: "fao".into(),
                    port1: "add_a".into(),
                },
                RawConnection {
                    port0: "fbo".into(),
                    port1: "add_b".into(),
                },
                RawConnection {
                    port0: "add_o".into(),
                    port1: "cvx".into(),
                },
                RawConnection {
                    port0: "z0o".into(),
                    port1: "cvy".into(),
                },
                RawConnection {
                    port0: "z0o".into(),
                    port1: "cvz".into(),
                },
                RawConnection {
                    port0: "cvo".into(),
                    port1: "c1m".into(),
                },
                RawConnection {
                    port0: "bfo".into(),
                    port1: "c1s".into(),
                },
                RawConnection {
                    port0: "bfo".into(),
                    port1: "c1i".into(),
                },
            ],
        };
        let graph = index_graph(raw, "add_test".into());
        let compiled = Lowerer::compile_for(graph, crate::mode::GameSpec::soccer());
        assert!(!compiled.controllers.instructions.is_empty());
        let program = ProgramBuilder.pack(&compiled);
        let mut ctx = ExecutionContext::new(
            crate::api::TeamApi::empty(crate::brain::TeamId::Home),
            compiled.vars.len(),
            program.register_count as usize,
        );
        ctx.init_api_slots(&compiled.apis);
        let mut interp = Interpreter::default();
        interp.execute_controllers(&program, &mut ctx);
        assert!((ctx.output.commands[0].move_to.x - 7.0).abs() < 1e-4);
    }

    #[test]
    fn unity_sqrt_dropdown_index_10_via_runtime() {
        // Unity AIA saves Operation(sqrt) as modifier "10", not the label "sqrt".
        let raw = RawGraph {
            nodes: vec![
                node("Float", "f2", "2", vec![port("Float1", "f2o", 1, "f2")]),
                node(
                    "Operation",
                    "op",
                    "10",
                    vec![
                        port("Float1", "opi", 0, "op"),
                        port("Float1", "opo", 1, "op"),
                    ],
                ),
                node("Float", "z0", "0", vec![port("Float1", "z0o", 1, "z0")]),
                node(
                    "ConstructVector3",
                    "cv",
                    "",
                    vec![
                        port("Vector31", "cvo", 1, "cv"),
                        port("Float1", "cvx", 0, "cv"),
                        port("Float2", "cvy", 0, "cv"),
                        port("Float3", "cvz", 0, "cv"),
                    ],
                ),
                node("Bool", "bf", "1", vec![port("Bool1", "bfo", 1, "bf")]),
                node(
                    "SoccerController1",
                    "c1",
                    "",
                    vec![
                        port("Vector31", "c1m", 0, "c1"),
                        port("Bool1", "c1s", 0, "c1"),
                        port("Bool2", "c1i", 0, "c1"),
                    ],
                ),
            ],
            connections: vec![
                RawConnection {
                    port0: "f2o".into(),
                    port1: "opi".into(),
                },
                RawConnection {
                    port0: "opo".into(),
                    port1: "cvx".into(),
                },
                RawConnection {
                    port0: "z0o".into(),
                    port1: "cvy".into(),
                },
                RawConnection {
                    port0: "z0o".into(),
                    port1: "cvz".into(),
                },
                RawConnection {
                    port0: "cvo".into(),
                    port1: "c1m".into(),
                },
                RawConnection {
                    port0: "bfo".into(),
                    port1: "c1s".into(),
                },
                RawConnection {
                    port0: "bfo".into(),
                    port1: "c1i".into(),
                },
            ],
        };
        let graph = index_graph(raw, "sqrt_test".into());
        let compiled = Lowerer::compile_for(graph, crate::mode::GameSpec::soccer());
        let program = ProgramBuilder.pack(&compiled);
        let mut ctx = ExecutionContext::new(
            crate::api::TeamApi::empty(crate::brain::TeamId::Home),
            compiled.vars.len(),
            program.register_count as usize,
        );
        ctx.init_api_slots(&compiled.apis);
        let mut interp = Interpreter::default();
        interp.execute_controllers(&program, &mut ctx);
        let expect = 2.0_f32.sqrt();
        assert!(
            (ctx.output.commands[0].move_to.x - expect).abs() < 1e-4,
            "got {} want sqrt(2)={}",
            ctx.output.commands[0].move_to.x,
            expect
        );
    }

    fn node_owned(
        id: &str,
        sid: &str,
        modifier: &str,
        owner: &str,
        ports: Vec<RawPort>,
    ) -> RawNode {
        RawNode {
            id: id.into(),
            sid: sid.into(),
            modifier: serde_json::json!(modifier),
            owner_function_sid: owner.into(),
            ports,
        }
    }

    #[test]
    fn root_function_debug_draw_lowers_and_runs() {
        let raw = RawGraph {
            nodes: vec![
                node(
                    "CreateFunction",
                    "cf",
                    "Draw",
                    vec![
                        port("Any1", "cf_a1o", 1, "cf"),
                        port("Any2", "cf_a2o", 1, "cf"),
                        port("Any4", "cf_a4o", 1, "cf"),
                        port("Any1", "cf_ret", 0, "cf"),
                    ],
                ),
                node_owned("Relay", "ra", "", "cf", vec![port("Any1", "rao", 2, "ra")]),
                node_owned("Relay", "rb", "", "cf", vec![port("Any1", "rbo", 2, "rb")]),
                node_owned("Relay", "rc", "", "cf", vec![port("Any1", "rco", 2, "rc")]),
                node_owned(
                    "Float",
                    "fw",
                    "1",
                    "cf",
                    vec![port("Float1", "fwo", 1, "fw")],
                ),
                node_owned(
                    "DebugDrawLine",
                    "ddl",
                    "",
                    "cf",
                    vec![
                        port("Vector31", "ddla", 0, "ddl"),
                        port("Vector32", "ddlb", 0, "ddl"),
                        port("Float1", "ddlw", 0, "ddl"),
                        port("Color1", "ddlc", 0, "ddl"),
                    ],
                ),
                node("Float", "ax", "1", vec![port("Float1", "axo", 1, "ax")]),
                node("Float", "az", "2", vec![port("Float1", "azo", 1, "az")]),
                node("Float", "ay", "0", vec![port("Float1", "ayo", 1, "ay")]),
                node(
                    "ConstructVector3",
                    "pa",
                    "",
                    vec![
                        port("Vector31", "pao", 1, "pa"),
                        port("Float1", "pax", 0, "pa"),
                        port("Float2", "pay", 0, "pa"),
                        port("Float3", "paz", 0, "pa"),
                    ],
                ),
                node("Float", "bx", "5", vec![port("Float1", "bxo", 1, "bx")]),
                node("Float", "bz", "6", vec![port("Float1", "bzo", 1, "bz")]),
                node("Float", "by", "0", vec![port("Float1", "byo", 1, "by")]),
                node(
                    "ConstructVector3",
                    "pb",
                    "",
                    vec![
                        port("Vector31", "pbo", 1, "pb"),
                        port("Float1", "pbx", 0, "pb"),
                        port("Float2", "pby", 0, "pb"),
                        port("Float3", "pbz", 0, "pb"),
                    ],
                ),
                node(
                    "Color",
                    "col",
                    "Orange",
                    vec![port("Color1", "colo", 1, "col")],
                ),
                node(
                    "Function",
                    "fn",
                    "Draw",
                    vec![
                        port("Any1", "fna1", 0, "fn"),
                        port("Any2", "fna2", 0, "fn"),
                        port("Any4", "fna4", 0, "fn"),
                        port("Any1", "fno", 1, "fn"),
                    ],
                ),
            ],
            connections: vec![
                RawConnection {
                    port0: "cf_a1o".into(),
                    port1: "rao".into(),
                },
                RawConnection {
                    port0: "cf_a2o".into(),
                    port1: "rbo".into(),
                },
                RawConnection {
                    port0: "cf_a4o".into(),
                    port1: "rco".into(),
                },
                RawConnection {
                    port0: "rao".into(),
                    port1: "ddla".into(),
                },
                RawConnection {
                    port0: "rbo".into(),
                    port1: "ddlb".into(),
                },
                RawConnection {
                    port0: "rco".into(),
                    port1: "ddlc".into(),
                },
                RawConnection {
                    port0: "fwo".into(),
                    port1: "ddlw".into(),
                },
                RawConnection {
                    port0: "axo".into(),
                    port1: "pax".into(),
                },
                RawConnection {
                    port0: "ayo".into(),
                    port1: "pay".into(),
                },
                RawConnection {
                    port0: "azo".into(),
                    port1: "paz".into(),
                },
                RawConnection {
                    port0: "bxo".into(),
                    port1: "pbx".into(),
                },
                RawConnection {
                    port0: "byo".into(),
                    port1: "pby".into(),
                },
                RawConnection {
                    port0: "bzo".into(),
                    port1: "pbz".into(),
                },
                RawConnection {
                    port0: "pao".into(),
                    port1: "fna1".into(),
                },
                RawConnection {
                    port0: "pbo".into(),
                    port1: "fna2".into(),
                },
                RawConnection {
                    port0: "colo".into(),
                    port1: "fna4".into(),
                },
            ],
        };

        let graph = index_graph(raw, "vm_draw_test".into());
        let compiled = Lowerer::compile_for(graph, crate::mode::GameSpec::soccer());
        let has_draw = compiled
            .controllers
            .instructions
            .iter()
            .any(|i| i.op == OpCode::DebugDrawLine);
        assert!(
            has_draw,
            "controllers IR must include DebugDrawLine from Function body"
        );

        let program = ProgramBuilder.pack(&compiled);
        let mut ctx = ExecutionContext::new(
            crate::api::TeamApi::empty(crate::brain::TeamId::Home),
            compiled.vars.len(),
            program.register_count as usize,
        );
        ctx.init_api_slots(&compiled.apis);
        crate::debug_draw::begin_frame();
        let mut interp = Interpreter::default();
        interp.execute_settle(&program, &mut ctx);
        interp.execute_controllers(&program, &mut ctx);
        let snap = crate::debug_draw::snapshot();
        assert_eq!(snap.lines.len(), 1);
        let line = &snap.lines[0];
        assert!((line.a.x - 1.0).abs() < 1e-4 && (line.a.y - 2.0).abs() < 1e-4);
        assert!((line.b.x - 5.0).abs() < 1e-4 && (line.b.y - 6.0).abs() < 1e-4);
        assert_eq!(line.rgba, crate::debug_draw::named_rgba("Orange"));
    }

    /// Feedback cycles are broken with prev-tick latches (Unity Memory cells).
    /// A mutual AddFloats loop must compile without overflowing the stack and
    /// without tripping [`RECURSION_LIMIT_HIT`].
    #[test]
    fn a_cycle_lowers_via_prev_tick_latch_without_overflowing() {
        // a.Float1 <- b.Float1out, b.Float1 <- a.Float1out : feedback loop.
        let raw = RawGraph {
            nodes: vec![
                node(
                    "AddFloats",
                    "a",
                    "",
                    vec![
                        port("Float1", "a_in1", 0, "a"),
                        port("Float2", "a_in2", 0, "a"),
                        port("Float1", "a_out", 1, "a"),
                    ],
                ),
                node(
                    "AddFloats",
                    "b",
                    "",
                    vec![
                        port("Float1", "b_in1", 0, "b"),
                        port("Float2", "b_in2", 0, "b"),
                        port("Float1", "b_out", 1, "b"),
                    ],
                ),
                node("Bool", "bf", "1", vec![port("Bool1", "bfo", 1, "bf")]),
                node("Float", "z", "0", vec![port("Float1", "zo", 1, "z")]),
                node(
                    "ConstructVector3",
                    "cv",
                    "",
                    vec![
                        port("Vector31", "cvo", 1, "cv"),
                        port("Float1", "cvx", 0, "cv"),
                        port("Float2", "cvy", 0, "cv"),
                        port("Float3", "cvz", 0, "cv"),
                    ],
                ),
                node(
                    "SoccerController1",
                    "c1",
                    "",
                    vec![
                        port("Vector31", "c1m", 0, "c1"),
                        port("Bool1", "c1s", 0, "c1"),
                        port("Bool2", "c1i", 0, "c1"),
                    ],
                ),
            ],
            connections: vec![
                RawConnection {
                    port0: "b_out".into(),
                    port1: "a_in1".into(),
                },
                RawConnection {
                    port0: "a_out".into(),
                    port1: "b_in1".into(),
                },
                RawConnection {
                    port0: "a_out".into(),
                    port1: "cvx".into(),
                },
                RawConnection {
                    port0: "zo".into(),
                    port1: "cvy".into(),
                },
                RawConnection {
                    port0: "zo".into(),
                    port1: "cvz".into(),
                },
                RawConnection {
                    port0: "cvo".into(),
                    port1: "c1m".into(),
                },
                RawConnection {
                    port0: "bfo".into(),
                    port1: "c1s".into(),
                },
                RawConnection {
                    port0: "bfo".into(),
                    port1: "c1i".into(),
                },
            ],
        };
        let graph = crate::graph::load::index_graph(raw, "cycle".into());
        let _ = take_recursion_limit_hit();

        let handle = std::thread::Builder::new()
            .stack_size(1 << 21) // 2 MB
            .spawn(move || {
                let result = Lowerer::compile_for(graph, crate::mode::GameSpec::soccer());
                assert!(
                    !take_recursion_limit_hit(),
                    "feedback cycle must lower via latch, not trip depth cap"
                );
                let has_latch_load = result
                    .controllers
                    .instructions
                    .iter()
                    .chain(result.settle.instructions.iter())
                    .any(|i| i.op == OpCode::LoadVar && i.source_port == "latch");
                assert!(
                    has_latch_load,
                    "expected LoadVar latch for cycle back-edge"
                );
            })
            .expect("spawn");
        assert!(handle.join().is_ok(), "lowering a cycle overflowed the stack");
    }

    /// Deep demand chains lower on the dedicated 8 MB worker stack instead of
    /// the caller's (possibly 1 MB) stack. A 1000-deep linear chain must
    /// compile AND run correctly; a 2000-deep chain must trip the loud
    /// MAX_LOWER_DEPTH guard (ConstNull + flag) rather than killing the
    /// process with STATUS_STACK_OVERFLOW. graphc caps its own desc chains
    /// at 800, so neither shape arises from the compiler — this pins the
    /// sim-side safety net for hand-built saves.
    #[test]
    fn deep_linear_chain_lowers_loudly_instead_of_crashing() {
        fn chain_raw(depth: usize) -> RawGraph {
            let mut nodes = vec![
                node("Float", "one", "1", vec![port("Float1", "oneo", 1, "one")]),
                node("Float", "z", "0", vec![port("Float1", "zo", 1, "z")]),
                node(
                    "ConstructVector3",
                    "cv",
                    "",
                    vec![
                        port("Vector31", "cvo", 1, "cv"),
                        port("Float1", "cvx", 0, "cv"),
                        port("Float2", "cvy", 0, "cv"),
                        port("Float3", "cvz", 0, "cv"),
                    ],
                ),
                node("Bool", "bf", "1", vec![port("Bool1", "bfo", 1, "bf")]),
                node(
                    "SoccerController1",
                    "c1",
                    "",
                    vec![
                        port("Vector31", "c1m", 0, "c1"),
                        port("Bool1", "c1s", 0, "c1"),
                        port("Bool2", "c1i", 0, "c1"),
                    ],
                ),
            ];
            let mut connections = vec![
                RawConnection {
                    port0: "cvo".into(),
                    port1: "c1m".into(),
                },
                RawConnection {
                    port0: "bfo".into(),
                    port1: "c1s".into(),
                },
                RawConnection {
                    port0: "bfo".into(),
                    port1: "c1i".into(),
                },
                RawConnection {
                    port0: "zo".into(),
                    port1: "cvy".into(),
                },
                RawConnection {
                    port0: "zo".into(),
                    port1: "cvz".into(),
                },
            ];
            for i in 0..depth {
                let sid = format!("a{i}");
                nodes.push(RawNode {
                    id: "AddFloats".into(),
                    sid: sid.clone(),
                    modifier: serde_json::json!(""),
                    owner_function_sid: String::new(),
                    ports: vec![
                        RawPort {
                            id: "Float1".into(),
                            sid: format!("a{i}_in1"),
                            polarity: 0,
                            node_sid: sid.clone(),
                        },
                        RawPort {
                            id: "Float2".into(),
                            sid: format!("a{i}_in2"),
                            polarity: 0,
                            node_sid: sid.clone(),
                        },
                        RawPort {
                            id: "Float1".into(),
                            sid: format!("a{i}_out"),
                            polarity: 1,
                            node_sid: sid.clone(),
                        },
                    ],
                });
                // Float2 always reads the shared const 1: a_i = prev + 1.
                connections.push(RawConnection {
                    port0: "oneo".into(),
                    port1: format!("a{i}_in2"),
                });
                if i == 0 {
                    connections.push(RawConnection {
                        port0: "oneo".into(),
                        port1: "a0_in1".into(),
                    });
                } else {
                    connections.push(RawConnection {
                        port0: format!("a{}_out", i - 1),
                        port1: format!("a{i}_in1"),
                    });
                }
            }
            connections.push(RawConnection {
                port0: format!("a{}_out", depth - 1),
                port1: "cvx".into(),
            });
            RawGraph { nodes, connections }
        }

        // 1000 < 1500 cap: compiles clean and runs the full chain.
        // a_0 = 2, a_i = a_{i-1} + 1 → a_999 = 1001.
        let _ = take_recursion_limit_hit();
        let graph = crate::graph::load::index_graph(chain_raw(1000), "deep1000".into());
        let compiled = Lowerer::compile_for(graph, crate::mode::GameSpec::soccer());
        assert!(
            !take_recursion_limit_hit(),
            "1000-deep chain must stay under the depth cap"
        );
        let program = ProgramBuilder.pack(&compiled);
        let mut ctx = ExecutionContext::new(
            crate::api::TeamApi::empty(crate::brain::TeamId::Home),
            compiled.vars.len(),
            program.register_count as usize,
        );
        ctx.init_api_slots(&compiled.apis);
        let mut interp = Interpreter::default();
        interp.execute_controllers(&program, &mut ctx);
        assert!(
            (ctx.output.commands[0].move_to.x - 1001.0).abs() < 1e-2,
            "deep chain ran wrong: {:?}",
            ctx.output.commands[0].move_to
        );

        // 2000 > 1500 cap: loud guard (flag + Null at the cutoff), no crash.
        let _ = take_recursion_limit_hit();
        let graph = crate::graph::load::index_graph(chain_raw(2000), "deep2000".into());
        let _ = Lowerer::compile_for(graph, crate::mode::GameSpec::soccer());
        assert!(
            take_recursion_limit_hit(),
            "2000-deep chain must trip the depth guard loudly"
        );
    }
}
