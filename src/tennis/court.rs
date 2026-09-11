//! Court geometry â€” mirrored from `tennis-sim/native/recovered_court_v014.hpp`
//! (GetDiagonalServiceBox / GetServiceBoxBounds / serve-receive stances) and
//! the v0.12 spawn model (`core.hpp`).

use bevy::prelude::Vec2;
use bevy::prelude::Vec3;

use super::params::*;

/// Which half a player defends: Home at X < 0, Away at X > 0.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Home,
    Away,
}

impl Side {
    /// Sign of this side's baseline X direction (Home â’1, Away +1).
    pub fn sign(self) -> f32 {
        match self {
            Side::Home => -1.0,
            Side::Away => 1.0,
        }
    }

    pub fn other(self) -> Self {
        match self {
            Side::Home => Side::Away,
            Side::Away => Side::Home,
        }
    }
}

/// Ball position is genuinely 3D in tennis (Y = height).
pub type Pos3 = Vec3;

/// Is the landing point inside the court (singles lines), with the game's
/// line tolerance: half-extents grown by `line_width/2` plus the ball's out
/// radius (`ball_scale*0.55*0.5*0.98828125` â‰ 0.1685) plus the out margin.
pub fn is_in_court_xz(x: f32, z: f32) -> bool {
    let half_len = LINE_WIDTH * 0.5 + COURT_LENGTH * 0.5;
    let half_wid = LINE_WIDTH * 0.5 + COURT_SINGLES_WIDTH * 0.5;
    x.abs() <= half_len + BALL_OUT_RADIUS && z.abs() <= half_wid + BALL_OUT_RADIUS
}

/// Diagonal service box for the receiving side (v0.14 GetDiagonalServiceBox):
/// X from the net (0) to Â±`length/4`, Z = the receiver's half, every edge
/// grown by `line_width/2` (0.075). `ad_court` flips the Z half.
///
/// Returns `(min, max)` corners in world space.
pub fn service_box(receiver: Side, ad_court: bool) -> (Vec2, Vec2) {
    let grow = LINE_WIDTH * 0.5;
    let x_max = receiver.sign() * (COURT_LENGTH * 0.25) + receiver.sign() * grow;
    let x_min = if receiver.sign() > 0.0 { 0.0 - grow } else { 0.0 + grow };
    let (z_lo, z_hi) = if ad_court { (-1.0, 0.0) } else { (0.0, 1.0) };
    let z_min = z_lo * COURT_SINGLES_WIDTH * 0.5 - grow;
    let z_max = z_hi * COURT_SINGLES_WIDTH * 0.5 + grow;
    (
        Vec2::new(x_min.min(x_max), z_min),
        Vec2::new(x_min.max(x_max), z_max),
    )
}

/// Is a serve landing point inside the legal diagonal box.
pub fn is_serve_in(receiver: Side, ad_court: bool, x: f32, z: f32) -> bool {
    let (min, max) = service_box(receiver, ad_court);
    x >= min.x && x <= max.x && z >= min.y && z <= max.y
}

/// Center of the legal serve target area (midpoint of the expanded box).
pub fn legal_serve_target(receiver: Side, ad_court: bool) -> Vec2 {
    let (min, max) = service_box(receiver, ad_court);
    Vec2::new((min.x + max.x) * 0.5, (min.y + max.y) * 0.5)
}

/// Server stance (v0.14 `reference_serve_stand_v014`):
/// X = Â±(length/2 + 0.9166667) behind the baseline; Z = Â±singles/2 Ă— 0.45,
/// ad-court sign flip decides which half.
pub fn serve_stance(server: Side, ad_court: bool) -> Vec2 {
    let x = server.sign() * (COURT_LENGTH * 0.5 + 0.916_666_7);
    let z_sign = if ad_court { -1.0 } else { 1.0 };
    Vec2::new(x, z_sign * COURT_SINGLES_WIDTH * 0.5 * 0.45)
}

/// Receiver stance (v0.14 `reference_receive_stand_v014`):
/// X = â“(length/4 + 1.15) on the opposite side; same Z rule as the server.
pub fn receive_stance(server: Side, ad_court: bool) -> Vec2 {
    let x = server.other().sign() * (COURT_LENGTH * 0.25 + 1.15);
    let z_sign = if ad_court { -1.0 } else { 1.0 };
    Vec2::new(x, z_sign * COURT_SINGLES_WIDTH * 0.5 * 0.45)
}

/// Center of a side's back court (v0.14 `reference_center_of_back_v014`):
/// 0.375 Ă— length from the net, on the player's side.
pub fn center_of_back(side: Side) -> Vec2 {
    Vec2::new(side.sign() * COURT_LENGTH * 0.375, 0.0)
}

/// Center of a side's half (midway between net and baseline).
pub fn center_of_half(side: Side) -> Vec2 {
    Vec2::new(side.sign() * COURT_LENGTH * 0.25, 0.0)
}

/// Default aim target when the graph gives none (v0.14
/// `reference_default_aim_target_v014` rally case): center Â± length Ă— 0.48.
pub fn default_aim_target(attacker: Side) -> Vec2 {
    Vec2::new(attacker.other().sign() * COURT_LENGTH * 0.48, 0.0)
}

/// Serve-area depth behind the baseline (game-measured 2026-09-11 with
/// in-editor debug lines): home server X in [-16.25, -14.0], away mirrored.
/// The server stays outside the playable area; the boundary is inclusive
/// (standing exactly on the line is legal).
pub const SERVE_AREA_BACK: f32 = 2.25;

/// Movement clamp for a player (v0.14 recovered_move_geometry, non-serving
/// case: court bounds grown by `strike_radius`; serving player confined to
/// the measured serve-area box: X = baseline band behind the baseline,
/// Z = full half-width from center to the deuce/ad sideline
/// (deuce: [0, +6], ad: [-6, 0]). The game forces the server's mover
/// destination into this box even with no walk input.
pub fn clamp_player_position(side: Side, pos: Vec2, serving: bool, ad_court: bool) -> Vec2 {
    if serving {
        let baseline = side.sign() * COURT_LENGTH * 0.5;
        let (lo, hi) = if side.sign() > 0.0 {
            (baseline, baseline + SERVE_AREA_BACK)
        } else {
            (baseline - SERVE_AREA_BACK, baseline)
        };
        let half_wid = COURT_SINGLES_WIDTH * 0.5;
        let (zlo, zhi) = if ad_court {
            (-half_wid, 0.0)
        } else {
            (0.0, half_wid)
        };
        return Vec2::new(
            pos.x.clamp(lo.min(hi), lo.max(hi)),
            pos.y.clamp(zlo, zhi),
        );
    }
    let half_len = COURT_LENGTH * 0.5 + STRIKE_RADIUS;
    let half_wid = COURT_SINGLES_WIDTH * 0.5 + 0.35;
    Vec2::new(
        pos.x.clamp(-half_len, half_len),
        pos.y.clamp(-half_wid, half_wid),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn court_bounds_admit_baseline_shots_and_reject_wide_ones() {
        assert!(is_in_court_xz(13.9, 5.9));
        assert!(!is_in_court_xz(14.5, 0.0));
        assert!(!is_in_court_xz(0.0, 6.5));
        // The ball out radius (0.1685) extends the playable line; beyond that
        // the landing is out even before the scoring margin applies.
        assert!(is_in_court_xz(14.0 + 0.4 * BALL_OUT_RADIUS, 0.0));
        assert!(!is_in_court_xz(14.0 + 2.0 * BALL_OUT_RADIUS, 0.0));
    }

    #[test]
    fn service_box_is_diagonal_half_with_line_grow() {
        let (min, max) = service_box(Side::Away, false);
        assert!((min.x - (-0.075)).abs() < 1e-5);
        assert!((max.x - (7.0 + 0.075)).abs() < 1e-5);
        assert!((min.y - (-0.075)).abs() < 1e-5);
        assert!((max.y - (6.0 + 0.075)).abs() < 1e-5);
        let (min, max) = service_box(Side::Away, true);
        assert!(min.y < 0.0 && max.y <= 0.075);
        // Deuce-court serve landing deep and wide is in; the ad mirror is out.
        assert!(is_serve_in(Side::Away, false, 6.0, 4.0));
        assert!(!is_serve_in(Side::Away, false, 6.0, -4.0));
        let _ = max;
    }

    #[test]
    fn serve_area_is_measured_box() {
        // Home deuce: X in [-16.25, -14.0] (bounds inclusive), Z in [0, 6].
        let c = clamp_player_position(Side::Home, Vec2::new(-15.0, 3.0), true, false);
        assert!((c.x + 15.0).abs() < 1e-5 && (c.y - 3.0).abs() < 1e-5);
        let c = clamp_player_position(Side::Home, Vec2::new(-13.0, 3.0), true, false);
        assert!((c.x + 14.0).abs() < 1e-5, "{}", c.x);
        let c = clamp_player_position(Side::Home, Vec2::new(-17.0, 3.0), true, false);
        assert!((c.x + 16.25).abs() < 1e-5, "{}", c.x);
        let c = clamp_player_position(Side::Home, Vec2::new(-15.0, -2.0), true, false);
        assert!(c.y.abs() < 1e-5, "{}", c.y);
        let c = clamp_player_position(Side::Home, Vec2::new(-14.0, 6.0), true, false);
        assert!((c.x + 14.0).abs() < 1e-5 && (c.y - 6.0).abs() < 1e-5);
        // Ad court: Z in [-6, 0].
        let c = clamp_player_position(Side::Home, Vec2::new(-15.0, -3.0), true, true);
        assert!((c.y + 3.0).abs() < 1e-5, "{}", c.y);
        let c = clamp_player_position(Side::Home, Vec2::new(-15.0, 2.0), true, true);
        assert!(c.y.abs() < 1e-5, "{}", c.y);
        // Away mirrors X: [14.0, 16.25].
        let c = clamp_player_position(Side::Away, Vec2::new(15.0, 3.0), true, false);
        assert!((c.x - 15.0).abs() < 1e-5, "{}", c.x);
        let c = clamp_player_position(Side::Away, Vec2::new(13.0, 3.0), true, false);
        assert!((c.x - 14.0).abs() < 1e-5, "{}", c.x);
        let c = clamp_player_position(Side::Away, Vec2::new(17.0, 3.0), true, false);
        assert!((c.x - 16.25).abs() < 1e-5, "{}", c.x);
    }

    #[test]
    fn stances_match_recovered_offsets() {
        let s = serve_stance(Side::Home, false);
        assert!((s.x - (-14.916_667)).abs() < 1e-4, "{}", s.x);
        assert!((s.y - 2.7).abs() < 1e-4, "{}", s.y); // Â±12/2*0.45
        let r = receive_stance(Side::Home, false);
        assert!((r.x - 8.15).abs() < 1e-4, "{}", r.x);
        let b = center_of_back(Side::Away);
        assert!((b.x - 10.5).abs() < 1e-5);
    }
}
