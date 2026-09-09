//! Auto-generated SoccerGet* dropdown labels (index -> label).
//! Source: AIGamePyLibrary.data.DROPDOWN_OPTIONS
//! Regenerate: python scripts/gen_graph_dropdowns.py
//!
//! RelativePosition table below is hand-maintained (not in SoccerGet* regen).

use bevy::prelude::Vec2;

pub const SOCCER_GET_BOOL: &[&str] = &[
    "Team Has Ball",
    "Opponent Has Ball",
    "Is Ball Loose",
    "Team Player 1 Has Ball",
    "Team Player 2 Has Ball",
    "Team Player 3 Has Ball",
    "Team Player 4 Has Ball",
    "Opponent Player 1 Has Ball",
    "Opponent Player 2 Has Ball",
    "Opponent Player 3 Has Ball",
    "Opponent Player 4 Has Ball",
    "Is Ball Nearby Team Player 1",
    "Is Ball Nearby Team Player 2",
    "Is Ball Nearby Team Player 3",
    "Is Ball Nearby Team Player 4",
    "Is Ball Nearby Opponent Player 1",
    "Is Ball Nearby Opponent Player 2",
    "Is Ball Nearby Opponent Player 3",
    "Is Ball Nearby Opponent Player 4",
    "Is Team Player 1 Closest Teammate to Ball",
    "Is Team Player 2 Closest Teammate to Ball",
    "Is Team Player 3 Closest Teammate to Ball",
    "Is Team Player 4 Closest Teammate to Ball",
    "Is Opponent Player 1 Closest Opponent to Ball",
    "Is Opponent Player 2 Closest Opponent to Ball",
    "Is Opponent Player 3 Closest Opponent to Ball",
    "Is Opponent Player 4 Closest Opponent to Ball",
    "Is Team Player 1 Open",
    "Is Team Player 2 Open",
    "Is Team Player 3 Open",
    "Is Team Player 4 Open",
    "Is Opponent Player 1 Open",
    "Is Opponent Player 2 Open",
    "Is Opponent Player 3 Open",
    "Is Opponent Player 4 Open",
    "Is Kickoff",
    "Is Team Kicking off",
    "Is Opponent Kicking off",
    "Team Is Winning",
    "Opponent Is Winning",
    "Team Scored Last Point",
    "Opponent Scored Last Point",
    "Ball On Team Side",
    "Ball On Opponent Side",
    "Is Ball Headed Towards Team Goal",
    "Is Ball Headed Towards Opponent Goal",
    "Is Home Team",
    "Is Away Team",
    "Is Active Graph",
    "Can Pass to Teammate 1",
    "Can Pass to Teammate 2",
    "Can Pass to Teammate 3",
    "Can Pass to Teammate 4",
    "Opponent Can Intercept Loose Ball 1",
    "Opponent Can Intercept Loose Ball 2",
    "Opponent Can Intercept Loose Ball 3",
    "Opponent Can Intercept Loose Ball 4",
];

pub const SOCCER_GET_FLOAT: &[&str] = &[
    "Team Score",
    "Opponent Score",
    "Team Shots",
    "Opponent Shots",
    "Team Possession %",
    "Opponent Possession %",
    "Team Attacking %",
    "Opponent Attacking %",
    "Ball Speed",
    "Player Interact Radius",
    // NO ENTRY HERE. "Player With Ball Shot Charge %" used to sit at index 10
    // and does not exist in the game's dropdown at all. Because graph files
    // store the dropdown INDEX (AIGamePyLibrary `_normalize_modifier` writes
    // the position, not the label), that one phantom entry shifted every label
    // from 10 onward by one, so 14 of the 15 floats the live champion reads
    // resolved to the wrong value:
    //   Ball Carrier Stamina  -> Player With Ball Shot Charge %  (phantom)
    //   Team Player 1 Stamina -> Goal Height                     (a constant!)
    //   Opponent Player 1 Stamina -> Distance from TP4 to nearest Opponent
    // Every stamina duel and shot-charge decision was therefore gated against
    // nonsense. Verified against AIGamePyLibrary's DROPDOWN_OPTIONS, which is
    // what the builder writes; `api/coverage_test.rs` now pins the ordering.
    "Ball Carrier Stamina",
    "Ball Carrier Shot Charge",
    "Teammate 1 Shot Charge",
    "Teammate 2 Shot Charge",
    "Teammate 3 Shot Charge",
    "Teammate 4 Shot Charge",
    "Field Width",
    "Field Depth",
    "Kickoff Circle Radius",
    "Goal Width",
    "Goal Height",
    "Team Player 1 Stamina",
    "Team Player 2 Stamina",
    "Team Player 3 Stamina",
    "Team Player 4 Stamina",
    "Distance from Team Player 1 to nearest Opponent",
    "Distance from Team Player 2 to nearest Opponent",
    "Distance from Team Player 3 to nearest Opponent",
    "Distance from Team Player 4 to nearest Opponent",
    "Opponent Player 1 Stamina",
    "Opponent Player 2 Stamina",
    "Opponent Player 3 Stamina",
    "Opponent Player 4 Stamina",
    "Opponent Nearest Teammate Player 1 Stamina",
    "Opponent Nearest Teammate Player 2 Stamina",
    "Opponent Nearest Teammate Player 3 Stamina",
    "Opponent Nearest Teammate Player 4 Stamina",
    "Stamina of last defending opponent",
    "Current Simulation Time",
    "Max Simulation Time",
    "Simulation Time Remaining",
    "Delta Time",
    "Fixed Delta Time",
    "Pi",
];

pub const SOCCER_GET_TRANSFORM: &[&str] = &[
    "Ball",
    "Team Player 1",
    "Team Player 2",
    "Team Player 3",
    "Team Player 4",
    "Opponent Player 1",
    "Opponent Player 2",
    "Opponent Player 3",
    "Opponent Player 4",
    "Teammate Nearest Team Player 1",
    "Teammate Nearest Team Player 2",
    "Teammate Nearest Team Player 3",
    "Teammate Nearest Team Player 4",
    "Opponent Nearest Team Player 1",
    "Opponent Nearest Team Player 2",
    "Opponent Nearest Team Player 3",
    "Opponent Nearest Team Player 4",
    "Team Goal Center",
    "Team Goal Left Post",
    "Team Goal Right Post",
    "Opponent Goal Center",
    "Opponent Goal Left Post",
    "Opponent Goal Right Post",
    "Opponent Nearest Team Goal",
    "Opponent Nearest Opponent Goal",
    "Teammate Nearest Team Goal",
    "Teammate Nearest Opponent Goal",
];

pub const SOCCER_GET_VECTOR3: &[&str] = &[
    "Ball Velocity",
    "Clear direction from team carrier",
    "Backwards clear direction from team carrier",
    "Clear direction from team carrier (avoid goal lines)",
    "Clear direction from team carrier (avoid sidelines)",
    "Clear direction from team carrier (avoid all walls)",
    "Upper Corner Home Side",
    "Lower Corner Home Side",
    "Upper Midfield",
    "Lower Midfield",
    "Upper Corner Away Side",
    "Lower Corner Away Side",
    "Upper Corner Opposing Side",
    "Lower Corner Opposing Side",
    "Upper Corner Team Side",
    "Lower Corner Team Side",
    "Center Field",
    "Get nearest open teammate",
    "Get furthest open teammate",
    "Get most open teammate",
    "Get nearest open opponent",
    "Get furthest open opponent",
    "Get most open opponent",
    "Direction of clear teammate from Teammate 1",
    "Direction of clear teammate from Teammate 2",
    "Direction of clear teammate from Teammate 3",
    "Direction of clear teammate from Teammate 4",
    "Direction of clear teammate from Opponent 1",
    "Direction of clear teammate from Opponent 2",
    "Direction of clear teammate from Opponent 3",
    "Direction of clear teammate from Opponent 4",
    "Direction of ball from Teammate 1",
    "Direction of ball from Teammate 2",
    "Direction of ball from Teammate 3",
    "Direction of ball from Teammate 4",
    "Direction of ball from Opponent 1",
    "Direction of ball from Opponent 2",
    "Direction of ball from Opponent 3",
    "Direction of ball from Opponent 4",
    "Direction of team goal from Teammate 1",
    "Direction of team goal from Teammate 2",
    "Direction of team goal from Teammate 3",
    "Direction of team goal from Teammate 4",
    "Direction of opponent goal from Teammate 1",
    "Direction of opponent goal from Teammate 2",
    "Direction of opponent goal from Teammate 3",
    "Direction of opponent goal from Teammate 4",
    "Clear direction from Teammate 1",
    "Clear direction from Teammate 2",
    "Clear direction from Teammate 3",
    "Clear direction from Teammate 4",
    "Direction of teammate from Team Player 1",
    "Direction of teammate from Team Player 2",
    "Direction of teammate from Team Player 3",
    "Direction of teammate from Team Player 4",
    "Perfect Pass Direction to Teammate 1",
    "Perfect Pass Direction to Teammate 2",
    "Perfect Pass Direction to Teammate 3",
    "Perfect Pass Direction to Teammate 4",
    "Ball Stop Position",
    "Loose Ball Intercept Position Teammate 1",
    "Loose Ball Intercept Position Teammate 2",
    "Loose Ball Intercept Position Teammate 3",
    "Loose Ball Intercept Position Teammate 4",
];

/// Auto-generated TennisGet* dropdown labels (index -> label).
/// Source: AIGamePyLibrary.data.DROPDOWN_OPTIONS ("Tennis v0.012 labels,
/// live TennisGet*Gate.BuildItems order") — the same pinning rule as soccer:
/// graph JSON stores the dropdown INDEX, so this table must match the
/// builder's order exactly or every label after an insertion shifts.
/// Regenerate: mirror scripts/gen_graph_dropdowns.py with the Tennis keys.
pub const TENNIS_GET_BOOL: &[&str] = &[
    "Is Playing",
    "Is Home",
    "Is Self Server For Set",
    "Is Opponent Server For Set",
    "Is Serve Phase",
    "Is Self Actively Serving",
    "Is Second Serve",
    "Is Ad Court Serve",
    "Is Ball Playable",
    "Was Last Shot Topspin",
    "Was Last Shot Slice",
    "Was Last Shot Flat",
    "Was Last Shot Lob",
    "Was Last Shot Drop",
    "Was Last Shot Curve Left",
    "Was Last Shot Curve Right",
    "Was Last Shot Trick",
    "Is Self Charging",
    "Is Opponent Charging",
    "Is Self Winning",
    "Is Opponent Winning",
    "Is Tied",
    "Is Deuce",
    "Is Game Point",
    "Is Break Point",
    "Is Match Point",
    "Ball On Self Side",
    "Ball Incoming",
    "Ball In Swing Range",
    "Ball Has Bounced",
    "Must Wait For Bounce",
    "Ball Has Charged Effect",
    "Self Scored Last Point",
    "Opponent Scored Last Point",
    "Self Has Advantage",
];

pub const TENNIS_GET_FLOAT: &[&str] = &[
    "Shot: Topspin",
    "Shot: Slice",
    "Shot: Flat",
    "Shot: Drop",
    "Shot: Lob",
    "Shot: Curve Left",
    "Shot: Curve Right",
    "Shot: Random",
    "Shot: Last Self Shot",
    "Shot: Last Opponent Shot",
    "Shot: Most Used Self Shot",
    "Shot: Most Used Opponent Shot",
    "Shot: Most Scored Self Shot",
    "Shot: Most Scored Opponent Shot",
    "Shot: Ball",
    "Self Swing Charge Pct",
    "Opponent Swing Charge Pct",
    "Self Stamina Pct",
    "Opponent Stamina Pct",
    "Deuce Fatigue",
    "Rally Fatigue",
    "Court Width",
    "Court Depth",
    "Net Height",
    "Ball Speed",
    "Ball Time To Ground",
    "Ball Time To 2nd Bounce",
    "Self Time To Destination",
    "Serve Number",
    "Self Points",
    "Opponent Points",
    "Self Set Score",
    "Opponent Set Score",
    "Self Aces",
    "Opponent Aces",
    "Self Faults",
    "Opponent Faults",
    "Self Double Faults",
    "Opponent Double Faults",
    "Self Fouls",
    "Opponent Fouls",
    "Self Outs",
    "Opponent Outs",
    "Self Charged Shots",
    "Opponent Charged Shots",
    "Current Simulation Time",
    "Sim Tick",
    "Delta Time",
    "Fixed Delta Time",
];

pub const TENNIS_GET_VECTOR3: &[&str] = &[
    "Ball Position",
    "Ball Velocity",
    "Predicted Bounce",
    "Predicted 2nd Bounce",
    "Center Of Half",
    "Center Of Back",
    "Serve Stance",
    "Receive Stance",
    "Legal Serve Target",
    "Random Aim Target",
    "Self Average Scoring Location",
    "Opponent Average Scoring Location",
    "Estimated Opponent Shot Location",
    "Trick Drop Modifier",
    "Trick Lob Modifier",
    "Trick Curve Left Modifier",
    "Trick Curve Right Modifier",
    "Camera Forward",
    "Camera Right",
];

pub const TENNIS_GET_TRANSFORM: &[&str] = &[
    "Self",
    "Opponent",
    "Self Racket Center",
    "Ball",
    "Camera",
];

/// `TennisAutoSwing` mode dropdown (graph input, not a sensor).
pub const TENNIS_AUTO_SWING: &[&str] = &["Normal Only", "Prefer Charge", "Random"];

/// Tennis v0.14 live runtime label positions that are pinned by the committed
/// capture digest (`tennis-sim/evidence/parity-v014-vm/runtime-items.md`).
/// Counts: Bool 35, Float 51, Vector3 15, Transform 5. Everything not listed
/// here is UNKNOWN in v0.14 — rejected, never invented (the C++ project's own
/// admission boundary).
pub mod tennis_v014 {
    /// (label, pinned index) pairs admitted by the runtime capture.
    pub const BOOL: &[(&str, usize)] = &[
        ("Is Playing", 0),
        ("Is Self Server For Set", 2),
        ("Is Self Actively Serving", 5),
        ("Was Last Shot Topspin", 9),
        ("Is Break Point", 24),
        ("Ball Incoming", 27),
        ("Ball In Swing Range", 28),
        ("Must Wait For Bounce", 30),
        ("Self Has Advantage", 34),
    ];
    pub const FLOAT: &[(&str, usize)] = &[
        ("Shot: Topspin", 0),
        ("Shot: Flat", 2),
        ("Shot: Drop", 3),
        ("Self Swing Charge Pct", 15),
        ("Court Width", 21),
        ("Ball Time To Ground", 25),
        ("Ball Time To 2nd Bounce", 26),
        ("Self Time To Destination", 27),
        ("Fixed Delta Time", 48),
        ("Gravity", 50),
    ];
    pub const VECTOR3: &[(&str, usize)] = &[
        ("Ball Position", 0),
        ("Ball Velocity", 1),
        ("Predicted Bounce", 2),
        ("Predicted 2nd Bounce", 3),
        ("Center Of Back", 5),
        ("Serve Stance", 6),
        ("Receive Stance", 7),
        ("Center Of Legal Serve Area", 8),
        ("Camera Forward", 13),
        ("Camera Right", 14),
    ];
    pub const TRANSFORM: &[(&str, usize)] = &[
        ("Self", 0),
        ("Opponent", 1),
        ("Self Racket Center", 2),
        ("Ball", 3),
        ("Camera", 4),
    ];

    /// Total runtime item counts (labels beyond the pinned positions exist in
    /// the game but their order is not committed — rejected until captured).
    pub const BOOL_COUNT: usize = 35;
    pub const FLOAT_COUNT: usize = 51;
    pub const VECTOR3_COUNT: usize = 15;
    pub const TRANSFORM_COUNT: usize = 5;

    /// The four documented LabelAliases: v0.12 spelling → v0.14 spelling.
    /// These bridge graphs written against the builder order onto the
    /// reordered runtime (`native/recovered_sensors_v014.hpp` lines 61-71).
    pub const LABEL_ALIASES: &[(&str, &str)] = &[
        ("Is Self Serving", "Is Self Actively Serving"),
        ("Time To Destination", "Self Time To Destination"),
        ("Time Ball To Ground", "Ball Time To Ground"),
        ("Legal Serve Target", "Center Of Legal Serve Area"),
    ];

    /// v0.12 spelling → canonical runtime spelling, when an alias applies.
    pub fn alias(label: &str) -> &str {
        for (from, to) in LABEL_ALIASES {
            if *from == label {
                return to;
            }
        }
        label
    }

    /// Is a v0.14 runtime label admitted by the capture?
    pub fn is_admitted(label: &str) -> bool {
        BOOL.iter().chain(FLOAT.iter()).chain(VECTOR3.iter()).chain(TRANSFORM.iter()).any(|(l, _)| *l == label)
    }
}

/// Resolve a modifier for a specific game version.
///
/// - `TennisV012` (builder order): index → label via the full tables.
/// - `TennisV014` (runtime order): the JSON index is interpreted against the
///   v0.12 builder order (that is what builders write), the label is bridged
///   through [`tennis_v014::alias`], and admitted only if the v0.14 capture
///   pins it — otherwise the modifier is rejected (returned as-is is NOT an
///   option: silent index confusion is the phantom-entry bug class).
///   Callers treating an unchanged modifier as "unresolved" must handle this
///   via [`tennis_v014::is_admitted`].
/// - `SoccerV05`: current soccer tables.
pub fn resolve_for_version(
    version: crate::mode::GameVersion,
    node_id: &str,
    modifier: &str,
) -> String {
    use crate::mode::GameVersion;
    let resolved = resolve(node_id, modifier);
    match version {
        GameVersion::SoccerV05 | GameVersion::TennisV012 => resolved.to_string(),
        GameVersion::TennisV014 => {
            let aliased = tennis_v014::alias(resolved);
            aliased.to_string()
        }
    }
}

/// True when a resolved tennis label is admitted under the v0.14 runtime
/// capture. Used to reject unmapped labels loudly instead of guessing.
pub fn tennis_v014_admits(label: &str) -> bool {
    tennis_v014::is_admitted(label)
}

pub fn resolve<'a>(node_id: &str, modifier: &'a str) -> &'a str {
    let opts: &[&str] = match node_id {
        "SoccerGetBool" => SOCCER_GET_BOOL,
        "SoccerGetFloat" => SOCCER_GET_FLOAT,
        "SoccerGetTransform" => SOCCER_GET_TRANSFORM,
        "SoccerGetVector3" => SOCCER_GET_VECTOR3,
        "TennisGetBool" => TENNIS_GET_BOOL,
        "TennisGetFloat" => TENNIS_GET_FLOAT,
        "TennisGetTransform" => TENNIS_GET_TRANSFORM,
        "TennisGetVector3" => TENNIS_GET_VECTOR3,
        "TennisAutoSwing" => TENNIS_AUTO_SWING,
        "RelativePosition" => RELATIVE_POSITION,
        // Operation stays a Unity dropdown index in JSON; see [`OperationKind`].
        _ => return modifier,
    };
    if let Ok(i) = modifier.parse::<usize>() {
        if let Some(label) = opts.get(i) {
            return label;
        }
    }
    modifier
}

/// Unity `Operation` dropdown (AIGamePyLibrary `Operation(...).index`).
///
/// Discriminants **are** the wire values in graph JSON / IR immediates.
/// Writers use variants (`OperationKind::Sqrt`); never invent raw integers at
/// call sites — parse with [`OperationKind::from_modifier`] or
/// [`OperationKind::from_immediate`].
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum OperationKind {
    Abs = 0,
    Round = 1,
    Floor = 2,
    Ceil = 3,
    Sin = 4,
    Cos = 5,
    Tan = 6,
    Asin = 7,
    Acos = 8,
    Atan = 9,
    Sqrt = 10,
    Sign = 11,
    Ln = 12,
    Log10 = 13,
    /// `e^`
    Exp = 14,
    /// `10^`
    Pow10 = 15,
}

impl OperationKind {
    pub const COUNT: u32 = 16;

    /// Unity / IR integer → kind. **Panics** if out of range (no silent fallback).
    pub fn from_immediate(kind: u32) -> Self {
        Self::try_from_immediate(kind).unwrap_or_else(|| {
            panic!(
                "Operation kind {kind} out of range 0..{} (Unity dropdown)",
                Self::COUNT
            )
        })
    }

    pub fn try_from_immediate(kind: u32) -> Option<Self> {
        Some(match kind {
            0 => Self::Abs,
            1 => Self::Round,
            2 => Self::Floor,
            3 => Self::Ceil,
            4 => Self::Sin,
            5 => Self::Cos,
            6 => Self::Tan,
            7 => Self::Asin,
            8 => Self::Acos,
            9 => Self::Atan,
            10 => Self::Sqrt,
            11 => Self::Sign,
            12 => Self::Ln,
            13 => Self::Log10,
            14 => Self::Exp,
            15 => Self::Pow10,
            _ => return None,
        })
    }

    /// Graph JSON modifier: `"10"` (Unity) or legacy label `"sqrt"`.
    /// **Panics** on unknown — never returns a default op.
    pub fn from_modifier(modifier: &str) -> Self {
        let m = modifier.trim();
        if let Ok(i) = m.parse::<u32>() {
            return Self::from_immediate(i);
        }
        match m.to_ascii_lowercase().as_str() {
            "abs" => Self::Abs,
            "round" => Self::Round,
            "floor" => Self::Floor,
            "ceil" => Self::Ceil,
            "sin" => Self::Sin,
            "cos" => Self::Cos,
            "tan" => Self::Tan,
            "asin" => Self::Asin,
            "acos" => Self::Acos,
            "atan" => Self::Atan,
            "sqrt" => Self::Sqrt,
            "sign" | "signum" => Self::Sign,
            "ln" => Self::Ln,
            "log10" => Self::Log10,
            "e^" => Self::Exp,
            "10^" => Self::Pow10,
            _ => panic!(
                "unknown Operation modifier {modifier:?}; expected index 0..{} or a Unity label",
                Self::COUNT
            ),
        }
    }

    #[inline]
    pub fn as_u32(self) -> u32 {
        self as u32
    }

    /// Human-readable Unity dropdown label; retained for tooling and probes.
    #[allow(dead_code)]
    pub fn label(self) -> &'static str {
        match self {
            Self::Abs => "abs",
            Self::Round => "round",
            Self::Floor => "floor",
            Self::Ceil => "ceil",
            Self::Sin => "sin",
            Self::Cos => "cos",
            Self::Tan => "tan",
            Self::Asin => "asin",
            Self::Acos => "acos",
            Self::Atan => "atan",
            Self::Sqrt => "sqrt",
            Self::Sign => "sign",
            Self::Ln => "ln",
            Self::Log10 => "log10",
            Self::Exp => "e^",
            Self::Pow10 => "10^",
        }
    }

    /// Apply the op. Exhaustive match — new variants must handle here.
    pub fn eval(self, a: f32) -> f32 {
        match self {
            Self::Abs => a.abs(),
            Self::Round => a.round(),
            Self::Floor => a.floor(),
            Self::Ceil => a.ceil(),
            Self::Sin => a.sin(),
            Self::Cos => a.cos(),
            Self::Tan => a.tan(),
            Self::Asin => a.asin(),
            Self::Acos => a.acos(),
            Self::Atan => a.atan(),
            Self::Sqrt => a.max(0.0).sqrt(),
            Self::Sign => {
                if a > 0.0 {
                    1.0
                } else if a < 0.0 {
                    -1.0
                } else {
                    0.0
                }
            }
            Self::Ln => a.ln(),
            Self::Log10 => a.log10(),
            Self::Exp => a.exp(),
            Self::Pow10 => 10f32.powf(a),
        }
    }
}

/// IR helper: immediate `u32` → eval (panics on bad kind).
pub fn eval_operation(a: f32, kind: u32) -> f32 {
    OperationKind::from_immediate(kind).eval(a)
}

/// Load helper: JSON modifier → wire `u32` for IR immediates.
#[allow(dead_code)]
pub fn operation_kind(modifier: &str) -> u32 {
    OperationKind::from_modifier(modifier).as_u32()
}

/// AIGamePyLibrary RelativePosition dropdown (+ World as used in Soccer UI).
pub const RELATIVE_POSITION: &[&str] = &[
    "Self",
    "Self + Forward",
    "Self + Backward",
    "Self + Left",
    "Self + Right",
    "Self + Up",
    "Self + Down",
    "Forward",
    "Backward",
    "Left",
    "Right",
    "Up",
    "Down",
    "World",
];

/// Pitch-plane meaning of a RelativePosition mode.
/// Forward = +X (goal axis); Left = +Z = our Y (sideline). Up/Down ignored in 2D.
/// No per-transform facing in TeamApi — offsets use world axes (good enough for Self/World).
pub fn relative_position_mode(mode: &str) -> RelativePosMode {
    match mode.trim() {
        "" | "Self" | "World" | "0" | "13" => RelativePosMode::WorldPos,
        "Self + Forward" | "1" => RelativePosMode::PosPlus(Vec2::X),
        "Self + Backward" | "2" => RelativePosMode::PosPlus(-Vec2::X),
        "Self + Left" | "3" => RelativePosMode::PosPlus(Vec2::Y),
        "Self + Right" | "4" => RelativePosMode::PosPlus(-Vec2::Y),
        "Self + Up" | "5" | "Self + Down" | "6" => RelativePosMode::WorldPos,
        "Forward" | "7" => RelativePosMode::DirOnly(Vec2::X),
        "Backward" | "8" => RelativePosMode::DirOnly(-Vec2::X),
        "Left" | "9" => RelativePosMode::DirOnly(Vec2::Y),
        "Right" | "10" => RelativePosMode::DirOnly(-Vec2::Y),
        "Up" | "11" | "Down" | "12" => RelativePosMode::DirOnly(Vec2::ZERO),
        _ => RelativePosMode::WorldPos,
    }
}

#[derive(Clone, Copy, Debug)]
pub enum RelativePosMode {
    /// Transform world position (Self / World).
    WorldPos,
    /// Position + world-axis unit offset.
    PosPlus(Vec2),
    /// Direction only (unit or zero).
    DirOnly(Vec2),
}

pub fn apply_relative_position(pos: Vec2, mode: &str) -> Vec2 {
    match relative_position_mode(mode) {
        RelativePosMode::WorldPos => pos,
        RelativePosMode::PosPlus(d) => pos + d,
        RelativePosMode::DirOnly(d) => d,
    }
}

#[cfg(test)]
mod relative_pos_tests {
    use super::*;

    #[test]
    fn self_and_world_are_world_pos() {
        let p = Vec2::new(3.0, 4.0);
        assert_eq!(apply_relative_position(p, "Self"), p);
        assert_eq!(apply_relative_position(p, "World"), p);
        assert_eq!(apply_relative_position(p, "13"), p);
    }

    #[test]
    fn self_plus_forward_offsets_goal_axis() {
        let p = Vec2::new(10.0, 0.0);
        assert_eq!(apply_relative_position(p, "Self + Forward"), Vec2::new(11.0, 0.0));
        assert_eq!(apply_relative_position(p, "Forward"), Vec2::X);
    }

    #[test]
    fn unity_operation_index_10_is_sqrt() {
        assert_eq!(OperationKind::from_modifier("10"), OperationKind::Sqrt);
        assert_eq!(OperationKind::from_modifier("sqrt"), OperationKind::Sqrt);
        let got = OperationKind::Sqrt.eval(2.0);
        assert!((got - 2.0_f32.sqrt()).abs() < 1e-6);
    }

    #[test]
    fn unity_operation_index_5_is_cos_not_sqrt() {
        assert_eq!(OperationKind::from_modifier("5"), OperationKind::Cos);
        let got = OperationKind::Cos.eval(0.0);
        assert!((got - 1.0).abs() < 1e-6);
    }

    #[test]
    #[should_panic(expected = "unknown Operation modifier")]
    fn unknown_operation_modifier_panics() {
        let _ = OperationKind::from_modifier("nope");
    }
}
