//! Physical shot arguments — the game's `ComputeShotVelocity` argument
//! domain, empirically verified against the live-captured 168-case matrix.
//!
//! Game ARG order (NOT the graph dropdown order):
//! `0 Topspin, 1 Slice, 2 Flat, 3 Lob, 4 Drop, 5 CurveLeft, 6 CurveRight`
//! (7 = trick variants resolve to one of the above via the seeded roll).
//!
//! The graph dropdown order (what `Shot: N` means in graph JSON) is
//! `0 Topspin, 1 Slice, 2 Flat, 3 Trick, 4 Drop, 5 Lob, 6 Curve Left,
//! 7 Curve Right` — [`ShotType`] below. Conversion to the game argument
//! lives here and is the only place the two orders meet. The recovered
//! mapping (`0→0, 1→1, 2→2, 4→4, 5→3, 6→5, 7→6`) is confirmed by the
//! fixture matrix: dropdown Lob(5) behaves as arg 3, CurveL(6) as arg 5.

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

    /// Graph dropdown order → game physical argument.
    /// `Topspin→0, Slice→1, Flat→2, Trick→3 (variant roll upstream),
    /// Drop→4, Lob→3?` — no: empirically Lob rolls to arg 3 and Trick is
    /// resolved by the game's variant roll before the solver; the mapping
    /// below follows the recovered table `0→0, 1→1, 2→2, 4→4, 5→3, 6→5,
    /// 7→6` with Trick→3 pending its RNG resolution.
    pub fn game_arg(self) -> ShotArg {
        match self {
            ShotType::Topspin => ShotArg::Topspin,
            ShotType::Slice => ShotArg::Slice,
            ShotType::Flat => ShotArg::Flat,
            ShotType::Trick => ShotArg::Lob, // trick variant roll: placeholder
            ShotType::Drop => ShotArg::Drop,
            ShotType::Lob => ShotArg::Lob,
            ShotType::CurveLeft => ShotArg::CurveLeft,
            ShotType::CurveRight => ShotArg::CurveRight,
        }
    }

    pub fn from_physical(v: i32) -> ShotType {
        match v {
            0 => ShotType::Topspin,
            1 => ShotType::Slice,
            2 => ShotType::Flat,
            3 => ShotType::Lob,
            4 => ShotType::Drop,
            5 => ShotType::CurveLeft,
            6 => ShotType::CurveRight,
            _ => ShotType::Trick,
        }
    }
}

/// The game solver's argument domain (empirical behavior table).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShotArg {
    Topspin = 0,
    Slice = 1,
    Flat = 2,
    Lob = 3,
    Drop = 4,
    CurveLeft = 5,
    CurveRight = 6,
}

impl ShotArg {
    /// (speed base, lift) — pinned words verified live.
    pub fn table(self) -> (f32, f32) {
        match self {
            ShotArg::Topspin => (24.0, 0.08),
            ShotArg::Slice => (16.0, 0.04),
            ShotArg::Flat => (28.0, 0.02),
            ShotArg::Lob => (12.0, 6.0),
            ShotArg::Drop => (7.6, 0.27),
            ShotArg::CurveLeft | ShotArg::CurveRight => (24.0, 0.08),
        }
    }

    /// Full charge range only for the straight family (0, 2).
    pub fn full_charge(self) -> bool {
        matches!(self, ShotArg::Topspin | ShotArg::Flat)
    }

    pub fn curve_sign(self) -> f32 {
        match self {
            ShotArg::CurveLeft => -1.0,
            ShotArg::CurveRight => 1.0,
            _ => 0.0,
        }
    }

    pub fn is_curve(self) -> bool {
        matches!(self, ShotArg::CurveLeft | ShotArg::CurveRight)
    }
}

/// Curve acceleration magnitude for a charged shot at pace 1.
pub fn curve_accel(arg: ShotArg, q: f32, speed: f32) -> f32 {
    let q = q.clamp(0.0, 1.0);
    match arg {
        ShotArg::CurveLeft | ShotArg::CurveRight => (1.3 + 1.4 * q) * CURVE_BASE_CURVE_SHOTS,
        ShotArg::Lob | ShotArg::Drop => (0.45 + 0.4 * q) * CURVE_BASE_SOFT_SHOTS,
        ShotArg::Slice => (0.85 + 0.55 * q) * speed * 0.12,
        _ => 0.0,
    }
}

use super::params::{CURVE_BASE_CURVE_SHOTS, CURVE_BASE_SOFT_SHOTS};
