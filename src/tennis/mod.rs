//! Tennis game mode — second game in the sim hub.
//!
//! Layers:
//! - [`params`] — measured constants pinned to the C++ sim's v0.14 fixtures.
//! - [`court`] — geometry: court, net plane, service boxes, stances, clamps.
//! - [`rng`] — SplitMix64 VM streams + Unity xorshift128 (bit-exact).
//! - [`shot_type`] / [`shot`] — physical shot types + the ballistic solver
//!   (aim → exact landing, net clearance, speed cap, steering).
//! - [`ball`] — flight forces (pace² scaling), net tape/rebound, floor bounce.
//! - [`score`] — points/games/serve rotation/faults.
//! - [`world`] — the headless match loop (serve lifecycle → rally → point).
//! - [`api`] — dense `TennisGet*` catalogs + per-tick snapshots.
//! - [`vm`] — compile/run tennis graphs on the shared node VM.
//!
//! Fidelity: the constants and collision/solver structures are ports of the
//! C++ recovered helpers (exact by source); flight integration and the
//! `TennisAuto*` helper policies are documented approximations until replayed
//! against the v0.14 runtime captures.

pub mod api;
pub mod ball;
pub mod court;
pub mod params;
pub mod rng;
pub mod score;
pub mod shot;
pub mod shot_type;
pub mod vm;
pub mod world;

pub use ball::{BallState, FlightEvent, FlightModel};
pub use court::Side;
pub use score::Score;
pub use shot_type::{ShotArg, ShotType};
pub use world::{EndReason, Phase, TennisWorld};

#[cfg(test)]
mod e2e_tests {
    use super::vm::{BrainSide, TennisBrain};
    use super::world::{Phase, TennisWorld};
    use crate::brain::TeamId;
    use crate::graph::load::load_graph;
    use crate::graph_vm::diagnostics;
    use crate::mode::GameSpec;

    fn titanium54_path() -> Option<std::path::PathBuf> {
        let base = std::env::var("USERPROFILE").ok()?;
        let p = std::path::PathBuf::from(base)
            .join(r"AppData\LocalLow\Unicorn One\AIComp\Saves\Tennis\titanium54.txt");
        p.exists().then_some(p)
    }

    /// The real champion graph, end to end: load with the tennis spec,
    /// compile on the shared VM, drive the tennis world for a while.
    /// Any unimplemented node type fails the run loudly (assert_sound).
    #[test]
    fn titanium54_runs_on_tennis_world() {
        let Some(path) = titanium54_path() else {
            return; // self-skip when the save is not installed (CI)
        };
        let graph = load_graph(&path, Some(GameSpec::tennis_builder()))
            .expect("titanium54 must load under the tennis spec");
        diagnostics::clear();
        let brain = TennisBrain::compile(graph);
        let mut home = Some(BrainSide {
            brain,
            team: TeamId::Home,
        });
        let mut away = None;

        let mut world = TennisWorld::new(20260907);
        for _ in 0..3000 {
            crate::tennis::vm::step_match(&mut world, &mut home, &mut away);
            if world.phase == Phase::Finished || world.score.points[0] + world.score.points[1] > 0
            {
                break;
            }
        }
        assert!(world.tick > 0);
        let unsound = diagnostics::unsound();
        assert!(
            unsound.is_empty(),
            "titanium54 touches unimplemented node types: {unsound:?}"
        );
    }

    /// A serve + rally point must resolve deterministically with stock bots.
    #[test]
    fn stock_match_resolves_points_deterministically() {
        let run = || {
            let mut world = TennisWorld::new(7);
            let mut points = 0usize;
            let mut in_pause = false;
            for _ in 0..60000 {
                crate::tennis::vm::step_match(&mut world, &mut None, &mut None);
                let pause = world.phase == Phase::PointPause;
                if pause && !in_pause {
                    points += 1;
                }
                in_pause = pause;
                if points >= 4 {
                    break;
                }
            }
            (points, world.score.games, world.tick)
        };
        let (a, ga, ta) = run();
        let (b, gb, tb) = run();
        assert_eq!(a, b, "same seed must resolve the same points");
        assert_eq!((ga, tb), (gb, ta), "same seed must be tick-identical");
        assert!(a >= 4, "points must resolve: {a} in {ta} ticks");
        assert!(ta < 60000, "points must resolve quickly, not stall");
    }

    /// Every user-owned bot graph in the Tennis saves folder must load under
    /// the tennis spec, compile on the shared VM, and drive the world without
    /// any unimplemented node type (the "perfectly simulates all bots" gate).
    #[test]
    fn owned_tennis_bots_run_on_the_vm() {
        let Some(base) = std::env::var("USERPROFILE")
            .ok()
            .map(std::path::PathBuf::from)
        else {
            return;
        };
        let saves = base.join(r"AppData\LocalLow\Unicorn One\AIComp\Saves\Tennis");
        // User-owned graphs only (titanium lineage + stock AIA).
        let bots = ["titanium54.txt", "aia3.txt"];
        diagnostics::clear();
        for bot in bots {
            let path = saves.join(bot);
            if !path.exists() {
                continue;
            }
            let graph = load_graph(&path, Some(GameSpec::tennis_builder()))
                .unwrap_or_else(|e| panic!("{bot} must load: {e}"));
            let mut brain = TennisBrain::compile(graph);
            let mut home = Some(BrainSide {
                brain,
                team: TeamId::Home,
            });
            let mut away: Option<BrainSide> = None;
            let mut world = TennisWorld::new(20260907);
            let mut commands_seen = 0;
            for _ in 0..1200 {
                let cmd = home
                    .as_mut()
                    .map(|b| b.command_for(&world))
                    .unwrap_or(None);
                if cmd.is_some() {
                    commands_seen += 1;
                }
                world.step([cmd, None]);
            }
            assert!(world.tick > 0, "{bot}: world must step");
            assert!(commands_seen > 0, "{bot}: brain must produce commands");
        }
        let unsound = diagnostics::unsound();
        assert!(
            unsound.is_empty(),
            "owned bots touch unimplemented node types: {unsound:?}"
        );
    }

    /// graphc-compiled tennis demo (aia_graphc/examples/demo_tennis.py):
    /// AST frontend -> description IR -> graphc-rs save. Must load under the
    /// tennis spec, compile on the shared VM, and drive the world — the
    /// TennisGet* sensors + TennisController path end to end.
    #[test]
    fn graphc_tennis_demo_replays() {
        let path = std::path::PathBuf::from(
            r"C:\gitProjects\aia_graphc\examples\graphc_demo_tennis.txt",
        );
        if !path.exists() {
            eprintln!("skip: graphc tennis demo not compiled at {path:?}");
            return;
        }
        let graph = load_graph(&path, Some(GameSpec::tennis_builder()))
            .expect("load graphc tennis demo");
        diagnostics::clear();
        let mut home = Some(BrainSide {
            brain: TennisBrain::compile(graph),
            team: TeamId::Home,
        });
        let mut away: Option<BrainSide> = None;
        let mut world = TennisWorld::new(20260907);
        let mut commands_seen = 0;
        let mut aims = Vec::new();
        for _ in 0..1200 {
            let cmd = home
                .as_mut()
                .map(|b| b.command_for(&world))
                .unwrap_or(None);
            if let Some(cmd) = cmd {
                commands_seen += 1;
                if aims.last() != Some(&cmd.move_or_aim) {
                    aims.push(cmd.move_or_aim);
                }
            }
            world.step([cmd, None]);
        }
        assert!(world.tick > 0, "world must step");
        assert!(commands_seen > 0, "graphc tennis bot must produce commands");
        println!("graphc tennis demo distinct aims: {aims:?}");
        let unsound = diagnostics::unsound();
        assert!(
            unsound.is_empty(),
            "graphc tennis bot touches unimplemented node types: {unsound:?}"
        );
    }
}
