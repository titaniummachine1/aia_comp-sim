//! Game-mode gate: which graph nodes belong to which game, and which
//! game-version ABI resolves them.
//!
//! Three orthogonal choices, all explicit — the core VM is a node engine
//! (like Lua without bindings) until a spec is attached:
//!
//! - [`GameMode`] — which game (Soccer / Tennis). Decides the owned node
//!   prefix (`Soccer*` / `Tennis*`) and the dense sensor catalogs.
//! - [`GameVersion`] — which build of that game. Game updates reorder
//!   dropdown tables between releases (tennis v0.012 builder order vs the
//!   v0.14 runtime order); the version picks the index→label table and the
//!   alias layer that keeps old graphs working (backwards compatibility).
//! - [`GameVariant`] — which label-table source within a version. `Standard`
//!   is the shipped library/builder tables; `SafeV01Capture` is the strict
//!   partial runtime-capture ABI (unknown labels rejected, never invented).
//!
//! [`GameSpec::latest`] is the default choice for simulations; unit tests of
//! pure graph logic may omit the spec entirely and run the VM with no API.

use crate::graph::load::TeamGraph;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GameMode {
    Soccer,
    Tennis,
    /// RacingV2 — ABI-only gate (no parity simulator exists for racing; the
    /// admit list below is pylib-proven but nothing semantic is verified).
    Racing,
}

impl GameMode {
    /// Prefix owning this mode's node types.
    pub fn node_prefix(self) -> &'static str {
        match self {
            GameMode::Soccer => "Soccer",
            GameMode::Tennis => "Tennis",
            GameMode::Racing => "RacingV2",
        }
    }

    /// The controller node type this mode's graphs drive.
    pub fn controller_node(self) -> &'static str {
        match self {
            GameMode::Soccer => "SoccerController1",
            GameMode::Tennis => "TennisController",
            GameMode::Racing => "ModularCarController",
        }
    }

    /// Whether an offline simulator verifies this mode's graphs.
    /// RacingV2 is ABI-only: the gate admits pylib-proven node kinds but
    /// nothing below has ever been simulated or measured against the game.
    pub fn parity(self) -> Parity {
        match self {
            GameMode::Soccer | GameMode::Tennis => Parity::Verified,
            GameMode::Racing => Parity::None,
        }
    }
}

/// Simulator parity status — what admitting a graph is actually worth.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Parity {
    /// Offline sim matches the game (physics + graph semantics) on fixtures.
    Verified,
    /// Node ABI admitted; nothing semantic verified. Emitting graphs for
    /// in-game testing is fine; trusting numbers is not.
    None,
}

/// Game builds. Dropdown tables and sensor semantics are per-version; alias
/// layers keep older graphs loading against newer versions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GameVersion {
    /// Soccer: current builder/library tables.
    SoccerV05,
    /// Tennis: AIGamePyLibrary builder order ("v0.012 labels, live
    /// TennisGet*Gate.BuildItems order") — full tables, what graphs on disk
    /// are written against.
    TennisV012,
    /// Tennis: v0.14 live runtime order (35/51/15/5). Partial: only labels
    /// pinned by the committed runtime capture (`tennis-sim
    /// evidence/parity-v014-vm/runtime-items.md`) are admitted, with the four
    /// documented LabelAliases bridging v0.12 spellings. Unknown → error.
    TennisV014,
    /// Tennis: v0.15 live runtime order. The node/label ABI is
    /// **v0.14-identical** (the v0.15 re-mine confirmed the shot solver and the
    /// operation dropdowns are unchanged), so it reuses the v0.14 admit/alias
    /// tables. v0.15's real differences are *world-model* (ball position is
    /// interpolated between frames; the time-to-ground cache was fixed) — see
    /// `docs/GAME_VERSIONS.md`.
    TennisV015,
    /// RacingV2 v0.22. ABI-only: the admit list below is complete against the
    /// 2026 AIGamePyLibrary fork + 14 real car graphs, but nothing semantic
    /// is verified — there is no racing simulator.
    RacingV022,
}

impl GameVersion {
    pub fn mode(self) -> GameMode {
        match self {
            GameVersion::SoccerV05 => GameMode::Soccer,
            GameVersion::TennisV012
            | GameVersion::TennisV014
            | GameVersion::TennisV015 => GameMode::Tennis,
            GameVersion::RacingV022 => GameMode::Racing,
        }
    }

    /// Newest version of a mode — the default for new work.
    pub fn latest_of(mode: GameMode) -> GameVersion {
        match mode {
            GameMode::Soccer => GameVersion::SoccerV05,
            GameMode::Tennis => GameVersion::TennisV015,
            GameMode::Racing => GameVersion::RacingV022,
        }
    }

    /// Tennis v0.15+ interpolates the ball position between frames so a fast
    /// ball can still be struck; the world uses this to pick a swept contact
    /// test instead of an end-of-tick point test. (`docs/GAME_VERSIONS.md`.)
    pub fn interpolates_ball(self) -> bool {
        matches!(self, GameVersion::TennisV015)
    }
}

/// Label-table source within a version.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum GameVariant {
    /// Shipped builder/library tables.
    #[default]
    Standard,
    /// Tennis v0.14 only: the owned Safe-v01 runtime-capture ABI
    /// (`legacy-simulator-label-table-v1` vs
    /// `v014-owned-safe-runtime-capture-v1` in the C++ project). Strictly
    /// partial; requires the capture payload to be supplied by the caller.
    SafeV01Capture,
}

/// A fully-qualified game ABI: mode + version + variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GameSpec {
    pub mode: GameMode,
    pub version: GameVersion,
    pub variant: GameVariant,
}

impl GameSpec {
    /// Default spec for a mode: the latest version, standard tables.
    pub fn latest(mode: GameMode) -> Self {
        Self {
            mode,
            version: GameVersion::latest_of(mode),
            variant: GameVariant::Standard,
        }
    }

    pub fn soccer() -> Self {
        Self::latest(GameMode::Soccer)
    }

    /// Tennis with the full builder-order tables (graphs built by
    /// AIGamePyLibrary).
    pub fn tennis_builder() -> Self {
        Self {
            mode: GameMode::Tennis,
            version: GameVersion::TennisV012,
            variant: GameVariant::Standard,
        }
    }

    /// Tennis v0.14 (explicit pinned target — the simulator's physics is
    /// v0.14-pinned, so this stays available even though [`Self::tennis_v015`]
    /// is now the latest).
    pub fn tennis_v014() -> Self {
        Self {
            mode: GameMode::Tennis,
            version: GameVersion::TennisV014,
            variant: GameVariant::Standard,
        }
    }

    /// Tennis v0.15f — the current target (latest tennis version). Node/label
    /// ABI is v0.14-identical; world-model differences live in `tennis::world`.
    pub fn tennis_v015() -> Self {
        Self::latest(GameMode::Tennis)
    }

    /// RacingV2 v0.22 — the current racing ABI target. ABI-only: admitting a
    /// graph here means its node kinds, ports and dropdown indices match the
    /// game; it says NOTHING about physics — no simulator exists. Compilers
    /// targeting this spec must carry the parity warning.
    pub fn racing() -> Self {
        Self::latest(GameMode::Racing)
    }

    pub fn is_pure(&self) -> bool {
        false
    }

    /// Non-fatal advisory for this spec: `Some` when the target has no parity
    /// simulator. Compilers print this with every emit.
    pub fn parity_warning(&self) -> Option<String> {
        match self.mode.parity() {
            Parity::Verified => None,
            Parity::None => Some(format!(
                "NO PARITY SIMULATOR for {:?}/v0.22: RacingV2 node ABI is \
                 admitted but unverified — test every change in-game. Only \
                 soccer v0.12 and tennis v0.14/v15f are parity-backed.",
                self.mode,
            )),
        }
    }
}

/// Node types shared by every AIComp game. Mirrors the arms implemented in
/// `graph::eval` and `graph_vm::lower` — keep in sync with those matches.
pub const COMMON_NODES: &[&str] = &[
    "Float",
    "Bool",
    "String",
    "Color",
    "GetVariable",
    "SetVariable",
    "Relay",
    "CreateFunction",
    "Function",
    "ConstructVector3",
    "Vector3Split",
    "AddFloats",
    "SubtractFloats",
    "MultiplyFloats",
    "DivideFloats",
    "Power",
    "Modulo",
    "AbsFloat",
    "Absolute",
    "ClampFloat",
    "Lerp",
    "Operation",
    "RandomFloat",
    "AddVector3",
    "SubtractVector3",
    "ScaleVector3",
    "Normalize",
    "Magnitude",
    "Distance",
    "DotProduct",
    "CrossProduct",
    "Not",
    "CompareFloats",
    "CompareBool",
    "ConditionalSetBool",
    "ConditionalSetFloat",
    "ConditionalSetFloatV2",
    "ConditionalSetVector3",
    "ConditionalSetString",
    "IsNull",
    "Keypress",
    "DebugDrawLine",
    "DebugDrawDisc",
    "TimePlot",
    "Debug",
    "Region",
    "Country",
    "Stat",
    "Spherecast",
    "RandomColor",
    "RelativePosition",
];

/// Why a node was rejected for a spec.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeRejection {
    /// Belongs to the other game (`SoccerGetFloat` in a tennis graph).
    OtherMode { owned_by: GameMode },
    /// Unknown to both the common set and any mode — probably a typo or an
    /// unimplemented node type.
    Unknown,
}

impl NodeRejection {
    pub fn reason(self, spec: GameSpec, node_id: &str) -> String {
        match self {
            NodeRejection::OtherMode { owned_by } => format!(
                "node {node_id:?} belongs to the {:?} game mode and is not valid in a {:?} graph",
                owned_by, spec.mode
            ),
            NodeRejection::Unknown => format!(
                "node {node_id:?} is not a known common node or a {:?} node",
                spec.mode
            ),
        }
    }
}

/// Mode-owned node types whose names do not start with the mode prefix.
const SOCCER_EXTRA_NODES: &[&str] = &["ConstructSoccerProperties"];
const TENNIS_EXTRA_NODES: &[&str] = &["ConstructTennisProperties"];
/// RacingV2 modular-car nodes: pylib-proven (ports + index modifiers verified
/// against 14 real car graphs), but NEVER sim-verified.
const RACING_EXTRA_NODES: &[&str] = &[
    "GetCarPart",
    "GetCarFromTransform",
    "ModularCarController",
    "Autosteer",
    "Autothrottle",
    "CarRaycasts",
    "HitInfo",
    "ConstructRacingV2Properties",
];

/// Classify a node type against a spec. `Ok(())` = legal.
pub fn check_node(spec: GameSpec, node_id: &str) -> Result<(), NodeRejection> {
    if COMMON_NODES.contains(&node_id) {
        return Ok(());
    }
    if SOCCER_EXTRA_NODES.contains(&node_id) {
        return match spec.mode {
            GameMode::Soccer => Ok(()),
            _ => Err(NodeRejection::OtherMode {
                owned_by: GameMode::Soccer,
            }),
        };
    }
    if TENNIS_EXTRA_NODES.contains(&node_id) {
        return match spec.mode {
            GameMode::Tennis => Ok(()),
            _ => Err(NodeRejection::OtherMode {
                owned_by: GameMode::Tennis,
            }),
        };
    }
    if RACING_EXTRA_NODES.contains(&node_id) {
        return match spec.mode {
            GameMode::Racing => Ok(()),
            _ => Err(NodeRejection::OtherMode {
                owned_by: GameMode::Racing,
            }),
        };
    }
    for m in [GameMode::Soccer, GameMode::Tennis, GameMode::Racing] {
        if node_id.starts_with(m.node_prefix()) {
            return if m == spec.mode {
                Ok(())
            } else {
                Err(NodeRejection::OtherMode { owned_by: m })
            };
        }
    }
    Err(NodeRejection::Unknown)
}

/// Validate every node in a parsed graph for `spec`.
///
/// Returns the offending `(node_id, reason)` pairs, deduplicated by node id,
/// in first-seen order. An empty vec means the graph may run in this spec.
pub fn validate_graph(spec: GameSpec, graph: &TeamGraph) -> Vec<(String, String)> {
    let mut seen = std::collections::BTreeSet::new();
    let mut out = Vec::new();
    for node in graph.nodes.values() {
        if check_node(spec, &node.id).is_err() && seen.insert(node.id.clone()) {
            let why = match check_node(spec, &node.id) {
                Err(r) => r.reason(spec, &node.id),
                Ok(()) => unreachable!(),
            };
            out.push((node.id.clone(), why));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mode_nodes_reject_cross_mode() {
        let tennis = GameSpec::tennis_builder();
        assert!(check_node(tennis, "TennisGetFloat").is_ok());
        assert!(check_node(tennis, "TennisController").is_ok());
        assert_eq!(
            check_node(tennis, "SoccerGetFloat"),
            Err(NodeRejection::OtherMode {
                owned_by: GameMode::Soccer
            })
        );
        assert_eq!(
            check_node(GameSpec::soccer(), "TennisAutoSwing"),
            Err(NodeRejection::OtherMode {
                owned_by: GameMode::Tennis
            })
        );
    }

    #[test]
    fn common_nodes_legal_in_both() {
        for node in ["AddFloats", "ConstructVector3", "TimePlot", "RelativePosition"] {
            assert!(check_node(GameSpec::soccer(), node).is_ok(), "{node}");
            assert!(check_node(GameSpec::tennis_builder(), node).is_ok(), "{node}");
        }
    }

    #[test]
    fn unknown_nodes_rejected() {
        assert_eq!(
            check_node(GameSpec::soccer(), "SoccrGetFloat"),
            Err(NodeRejection::Unknown)
        );
        assert_eq!(
            check_node(GameSpec::tennis_v014(), "TotallyMadeUp"),
            Err(NodeRejection::Unknown)
        );
    }

    #[test]
    fn latest_defaults_are_per_mode() {
        assert_eq!(GameSpec::latest(GameMode::Tennis).version, GameVersion::TennisV015);
        assert_eq!(GameSpec::tennis_v015().version, GameVersion::TennisV015);
        assert_eq!(GameSpec::tennis_v014().version, GameVersion::TennisV014);
        assert_eq!(GameSpec::latest(GameMode::Soccer).version, GameVersion::SoccerV05);
        assert_eq!(
            GameSpec::tennis_builder().version,
            GameVersion::TennisV012
        );
        assert_eq!(
            GameSpec::latest(GameMode::Racing).version,
            GameVersion::RacingV022
        );
        assert_eq!(GameSpec::racing().version, GameVersion::RacingV022);
    }

    #[test]
    fn racing_mode_admits_pylib_proven_nodes_with_parity_warning() {
        let racing = GameSpec::racing();
        for node in [
            "RacingV2GetFloat",
            "RacingV2GetBool",
            "RacingV2GetCar",
            "RacingV2GetWaypoint",
            "RacingV2Waypoint",
            "ConstructRacingV2Properties",
            "GetCarPart",
            "GetCarFromTransform",
            "ModularCarController",
            "Autosteer",
            "Autothrottle",
            "CarRaycasts",
            "HitInfo",
            "RelativePosition",
            "TimePlot",
        ] {
            assert!(check_node(racing, node).is_ok(), "{node}");
        }
        // Cross-mode nodes are refused in racing graphs...
        assert_eq!(
            check_node(racing, "SoccerGetFloat"),
            Err(NodeRejection::OtherMode {
                owned_by: GameMode::Soccer
            })
        );
        // ...and racing nodes are refused in the parity modes.
        assert_eq!(
            check_node(GameSpec::soccer(), "RacingV2GetFloat"),
            Err(NodeRejection::OtherMode {
                owned_by: GameMode::Racing
            })
        );
        // Everyone agrees there is no racing simulator.
        assert_eq!(GameMode::Racing.parity(), Parity::None);
        assert_eq!(GameMode::Soccer.parity(), Parity::Verified);
        assert_eq!(GameMode::Tennis.parity(), Parity::Verified);
        assert!(GameSpec::soccer().parity_warning().is_none());
        let w = racing.parity_warning().expect("racing must warn");
        assert!(w.contains("NO PARITY SIMULATOR"), "{w}");
    }
}
