//! Physical shot types — the game's `TennisShotType` enum
//! (`tennis-sim/native/protocol.hpp` lines 533-535).
//!
//! The graph dropdown order (what `Shot: N` means in graph JSON) is
//! `0 Topspin, 1 Slice, 2 Flat, 3 Trick, 4 Drop, 5 Lob, 6 Curve Left,
//! 7 Curve Right`, while the PHYSICAL enum orders Trick and Lob differently:
//! `Topspin 0, Slice 1, Flat 2, Trick 3, Drop 4, Lob 5, Curve Left 6,
//! Curve Right 7` with the option mapping `0→0, 1→1, 2→2, 4→4, 5→3, 6→5,
//! 7→6` (`recovered_vm_resolve_physical_shot_option`).
//!
//! To avoid the same class of bug as the soccer phantom dropdown entry, this
//! module uses ONE canonical order everywhere in the Rust port: the GRAPH
//! dropdown order (`ShotType` below). Conversion to the physical order lives
//! here and is the only place the two orders meet.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShotType {
    Topspin = 0,
    Slice = 1,
    Flat = 2,
    Trick = 3,
    Drop = 4,
    Lob = 5,
    CurveLeft = 6,
    CurveRight = 7,
}

impl ShotType {
    /// From the graph dropdown index (`Float1` on `TennisController`).
    /// Out-of-range / negative → `Random` handled by the caller.
    pub fn from_dropdown(index: f32) -> Option<ShotType> {
        Some(match index.round() as i32 {
            0 => ShotType::Topspin,
            1 => ShotType::Slice,
            2 => ShotType::Flat,
            3 => ShotType::Trick,
            4 => ShotType::Drop,
            5 => ShotType::Lob,
            6 => ShotType::CurveLeft,
            7 => ShotType::CurveRight,
            _ => return None,
        })
    }

    pub fn from_physical(v: i32) -> ShotType {
        match v {
            0 => ShotType::Topspin,
            1 => ShotType::Slice,
            2 => ShotType::Flat,
            3 => ShotType::Trick,
            4 => ShotType::Drop,
            5 => ShotType::Lob,
            6 => ShotType::CurveLeft,
            _ => ShotType::CurveRight,
        }
    }

    /// Graph dropdown index → physical enum value
    /// (`0→0, 1→1, 2→2, 4→4, 5→3, 6→5, 7→6`; 3 Trick stays trick).
    pub fn physical_value(self) -> i32 {
        match self {
            ShotType::Topspin => 0,
            ShotType::Slice => 1,
            ShotType::Flat => 2,
            ShotType::Trick => 3,
            ShotType::Drop => 4,
            ShotType::Lob => 5,
            ShotType::CurveLeft => 6,
            ShotType::CurveRight => 7,
        }
    }

    /// Lateral curve acceleration at pace 1 (m/s²), signed: + = ball's right.
    /// Curve shots: ±(1.3 + 1.4q) × 12.4; drop/lob: (0.45+0.4q) × 5.2 with
    /// sign from the aim side; slice: (0.85+0.55q) × magnitude factor;
    /// flat/topspin: none (`recovered_shot.hpp reference_curve`).
    pub fn curve_sign(self) -> f32 {
        match self {
            ShotType::CurveLeft => -1.0,
            ShotType::CurveRight => 1.0,
            _ => 0.0,
        }
    }

    pub fn is_curve(self) -> bool {
        matches!(self, ShotType::CurveLeft | ShotType::CurveRight)
    }
}

/// Curve acceleration magnitude for a charged shot at pace 1.
pub fn curve_accel(shot: ShotType, q: f32, speed: f32) -> f32 {
    let q = q.clamp(0.0, 1.0);
    match shot {
        ShotType::CurveLeft | ShotType::CurveRight => (1.3 + 1.4 * q) * CURVE_BASE_CURVE_SHOTS,
        ShotType::Drop | ShotType::Lob => (0.45 + 0.4 * q) * CURVE_BASE_SOFT_SHOTS,
        ShotType::Slice => (0.85 + 0.55 * q) * speed * 0.12,
        _ => 0.0,
    }
}

use super::params::{CURVE_BASE_CURVE_SHOTS, CURVE_BASE_SOFT_SHOTS};
