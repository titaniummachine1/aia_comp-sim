//! Compile + run tennis graphs on the shared node VM.
//!
//! The VM is game-agnostic: a [`Lowerer`] compiled with
//! [`GameMode::Tennis`](crate::mode::GameMode::Tennis) interns `TennisGet*`
//! labels into the tennis dense catalogs, and the interpreter's
//! `EmitTennisController` produces a [`TennisCommand`]. There is no default
//! mode — callers must pick, and the lowerer rejects foreign-mode nodes.

use std::sync::Arc;

use crate::api::DenseTeamApi;
use crate::brain::{BrainOutput, TeamId, TennisCommand};
use crate::graph::load::TeamGraph;
use crate::graph_vm::builder::ProgramBuilder;
use crate::graph_vm::context::ExecutionContext;
use crate::graph_vm::interpreter::Interpreter;
use crate::graph_vm::lower::{Lowerer, VariableTable};
use crate::graph_vm::passes::PassManager;
use crate::graph_vm::program::{Backend, RuntimeProgram};
use crate::graph_vm::value::VmValue;


/// A compiled tennis brain. Clone-cheap (program is Arc'd).
pub struct TennisBrain {
    program: Arc<RuntimeProgram>,
    vars: VariableTable,
    apis: crate::graph_vm::lower::ApiSlotTable,
    persistent: Vec<VmValue>,
    backend: Interpreter,
}

impl TennisBrain {
    pub fn compile(graph: TeamGraph) -> Self {
        let mut compiled = Lowerer::compile_for(graph, crate::mode::GameSpec::tennis_builder());
        let pm = PassManager::o1();
        pm.run_all(&mut compiled.settle);
        pm.run_all(&mut compiled.controllers);
        let vars = compiled.vars.clone();
        let apis = compiled.apis.clone();
        let program = Arc::new(ProgramBuilder.pack(&compiled));
        let persistent = (0..program.variable_count as usize)
            .map(|_| VmValue::Bool(false))
            .collect();
        Self {
            program,
            vars,
            apis,
            persistent,
            backend: Interpreter::default(),
        }
    }

    /// Run one think against the tennis snapshot for `team`.
    pub fn think(&mut self, api: DenseTeamApi) -> Option<TennisCommand> {
        let mut ctx = ExecutionContext::new(
            api,
            self.vars.len(),
            self.program.register_count as usize,
        );
        ctx.init_api_slots(&self.apis);
        ctx.state.vars.clone_from(&self.persistent);

        ctx.begin_pass(0);
        self.backend.execute_settle(&self.program, &mut ctx);
        ctx.begin_pass(8);
        self.backend.execute_controllers(&self.program, &mut ctx);
        self.persistent.clone_from(&ctx.state.vars);

        let out: BrainOutput = ctx.output;
        out.tennis_command
    }

    /// Reset persistent variables (new match).
    pub fn reset(&mut self) {
        self.persistent.fill(VmValue::Bool(false));
    }
}

/// A drive loop helper: one brain (per side) plus the world, stepping the
/// match with the brain's command and building the API each tick.
pub struct BrainSide {
    pub brain: TennisBrain,
    pub team: TeamId,
}

impl BrainSide {
    pub fn command_for(&mut self, world: &super::world::TennisWorld) -> Option<TennisCommand> {
        let side = match self.team {
            TeamId::Home => super::court::Side::Home,
            TeamId::Away => super::court::Side::Away,
        };
        let api = super::api::build_team_api(world, side);
        self.brain.think(api)
    }
}

/// Run a full point loop headlessly with two graph brains (or stock `None`).
pub fn step_match(
    world: &mut super::world::TennisWorld,
    home: &mut Option<BrainSide>,
    away: &mut Option<BrainSide>,
) {
    let home_cmd = home.as_mut().and_then(|b| b.command_for(world));
    let away_cmd = away.as_mut().and_then(|b| b.command_for(world));
    world.step([home_cmd, away_cmd]);
}
