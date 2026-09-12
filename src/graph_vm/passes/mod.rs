//! Optimization passes — one at a time after O0 verify is green.

pub mod const_fold;
pub mod cse;
pub mod fusion;
pub mod reg_alloc;
pub mod relay_removal;

use crate::graph_vm::ir::LoweredIR;

pub use const_fold::ConstFold;
pub use cse::Cse;
pub use fusion::Fusion;
pub use reg_alloc::RegAlloc;
pub use relay_removal::RelayRemoval;

pub trait Pass {
    fn name(&self) -> &'static str;
    fn run(&self, ir: &mut LoweredIR);
}

#[derive(Default)]
pub struct PassManager {
    pub passes: Vec<Box<dyn Pass>>,
}

impl PassManager {
    pub fn run_all(&self, ir: &mut LoweredIR) {
        for p in &self.passes {
            p.run(ir);
        }
    }

    /// O1 pipeline: ConstFold → RelayRemoval → CSE → Fusion → RegAlloc.
    pub fn o1() -> Self {
        Self::o1_prefix(5)
    }

    /// The O1 pipeline truncated to its first `k` passes. Diagnostics only:
    /// running a save through each prefix pinpoints *which* pass changes
    /// behaviour (`k >= 5` == [`Self::o1`]).
    pub fn o1_prefix(k: usize) -> Self {
        let mut passes: Vec<Box<dyn Pass>> = Vec::new();
        if k > 0 {
            passes.push(Box::new(ConstFold));
        }
        if k > 1 {
            passes.push(Box::new(RelayRemoval));
        }
        if k > 2 {
            passes.push(Box::new(Cse));
        }
        if k > 3 {
            passes.push(Box::new(Fusion));
        }
        if k > 4 {
            passes.push(Box::new(RegAlloc));
        }
        Self { passes }
    }

    /// Names of the passes in execution order (for reports).
    pub fn names(&self) -> Vec<&'static str> {
        self.passes.iter().map(|p| p.name()).collect()
    }

    /// Compatibility alias for the renamed O1 pipeline.
    pub fn o1_const_fold_only() -> Self {
        Self::o1()
    }
}
