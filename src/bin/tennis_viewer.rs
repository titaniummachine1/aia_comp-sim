//! Tennis viewer — non-headless 2D visualization of the tennis world
//! (same 2D-in-Bevy approach as the soccer viewer).
//!
//! Layout (world meters → screen at PPM):
//! - X = court depth (−14 baseline … 0 net … +14), right = away side
//! - Y = court width (±6 singles lines)
//! - Only the SINGLES court is drawn: baselines, sidelines, service lines
//!   (x = ±7), center service line (z = 0, between the service lines).
//!   The doubles alleys are deliberately NOT drawn — not legal target area.
//! - Net = grey slab at x = 0.
//! - Scoreboard: 3 set rings (home fills left→center, away right→center;
//!   center ring lit = set/match decided at 2).
//! - Players: white translucent circle = full strike range (2.6 m) around the
//!   racket, green translucent = perfect racket radius (1.0 m), small disc = the
//!   racket itself (0.55 m forward of the player). Charge bar UNDER the
//!   player, stamina bar ABOVE.
//!
//! Usage: `cargo run --bin tennis_viewer -- [--home titanium54] [--away aia3] [--seed 7]`

use bevy::prelude::*;
use bevy::math::primitives::Circle;
use bevy::render::mesh::{Mesh, PrimitiveTopology};

use aia_comp_sim::brain::TeamId;
use aia_comp_sim::tennis::court::Side;
use aia_comp_sim::tennis::params::*;
use aia_comp_sim::tennis::vm::{BrainSide, TennisBrain};
use aia_comp_sim::tennis::world::{Phase, TennisWorld};

const PPM: f32 = 28.0;
/// Scoreboard: three set rings spread between the two score labels.
/// Outer outline radius / lit inner-disc radius (screen px).
const SET_RING_R: f32 = 0.75 * PPM;
const SET_RING_INNER: f32 = 0.55 * PPM;
const SET_RING_SPACING: f32 = 3.2 * PPM;

#[derive(Resource)]
struct Match {
    world: TennisWorld,
    home: Option<BrainSide>,
    away: Option<BrainSide>,
    accumulator: f32,
    points_done: usize,
    in_pause: bool,
}

#[derive(Component)]
struct CourtMark;

#[derive(Component)]
struct PlayerViz(Side);

#[derive(Component)]
struct StrikeRing(Side);

#[derive(Component)]
struct PerfectRing(Side);

#[derive(Component)]
struct RacketDot(Side);

#[derive(Component)]
struct ChargeBar(Side);

#[derive(Component)]
struct StaminaBar(Side);

#[derive(Component)]
struct BallViz;

/// Ground-projection disc under the ball: the top-down view drops the height
/// axis, so the shadow is what shows where the ball actually IS on the court,
/// while the ball disc itself lifts/scales with `pos.y`.
#[derive(Component)]
struct BallShadow;

#[derive(Component)]
struct SetRingBody;

#[derive(Component)]
struct SetRingFill(usize);

#[derive(Component)]
struct ScoreText(Side);

#[derive(Component)]
struct PhaseText;

#[derive(Resource, Default)]
struct VizHandles {
    ball: Handle<ColorMaterial>,
    strike: Handle<ColorMaterial>,
    perfect: Handle<ColorMaterial>,
    racket: Handle<ColorMaterial>,
    charge: Handle<ColorMaterial>,
    stamina: Handle<ColorMaterial>,
    ring_full: Handle<ColorMaterial>,
    ring_empty: Handle<ColorMaterial>,
    ring_fill_home: Handle<ColorMaterial>,
    ring_fill_away: Handle<ColorMaterial>,
    text_home: Handle<ColorMaterial>,
    text_away: Handle<ColorMaterial>,
}

fn uv_rect(w: f32, h: f32) -> Mesh {
    let mut m = Mesh::new(PrimitiveTopology::TriangleStrip, default());
    let hw = w / 2.0;
    let hh = h / 2.0;
    m.insert_attribute(
        Mesh::ATTRIBUTE_POSITION,
        vec![[hw, hh, 0.0], [-hw, hh, 0.0], [hw, -hh, 0.0], [-hw, -hh, 0.0]],
    );
    m.insert_attribute(
        Mesh::ATTRIBUTE_UV_0,
        vec![[1.0, 1.0], [0.0, 1.0], [1.0, 0.0], [0.0, 0.0]],
    );
    m
}

fn wx(x: f32) -> f32 {
    x * PPM
}

fn wz(z: f32) -> f32 {
    z * PPM
}

fn load_bot(name: &str, team: TeamId) -> Option<(BrainSide, Vec<String>, Vec<String>)> {
    if name.is_empty() {
        return None;
    }
    let path = std::path::PathBuf::from(std::env::var("USERPROFILE").expect("USERPROFILE"))
        .join(r"AppData\LocalLow\Unicorn One\AIComp\Saves\Tennis")
        .join(format!("{name}.txt"));
    let graph = aia_comp_sim::graph::load::load_graph(
        &path,
        Some(aia_comp_sim::mode::GameSpec::tennis_builder()),
    )
    .unwrap_or_else(|e| panic!("load {name}: {e}"));
    aia_comp_sim::graph_vm::diagnostics::clear();
    let brain = TennisBrain::compile(graph);
    let stubbed = aia_comp_sim::graph_vm::diagnostics::stubbed();
    let unimpl: Vec<String> = stubbed
        .iter()
        .filter(|(_, t)| {
            *t == aia_comp_sim::graph_vm::diagnostics::Trust::Unimplemented
        })
        .map(|(n, _)| n.clone())
        .collect();
    let approx: Vec<String> = stubbed
        .iter()
        .filter(|(_, t)| {
            *t == aia_comp_sim::graph_vm::diagnostics::Trust::Approximated
        })
        .map(|(n, _)| n.clone())
        .collect();
    if !unimpl.is_empty() {
        warn!("{name}: unimplemented node types {unimpl:?}");
    }
    Some((BrainSide { brain, team }, unimpl, approx))
}

fn setup(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>, mut mats: ResMut<Assets<ColorMaterial>>) {
    commands.spawn((
        Camera2d,
        Transform::from_xyz(0.0, 0.0, 1000.0),
        Projection::Orthographic(OrthographicProjection {
            // 1.0 = 1 world metre == PPM screen pixels. The apron is
            // 38 m x 24 m = 1064 x 672 px at PPM 28, which fills the
            // 1280x760 window. (scale > 1 zooms OUT in Bevy.)
            scale: 1.0,
            ..OrthographicProjection::default_2d()
        }),
    ));

    // Court background (apron, blue) + playable court (brighter blue, singles
    // only). Blue-on-blue with white lines: the apron is the whole visible
    // background, so it doubles as the backdrop the user asked for.
    let apron = mats.add(Color::srgb(0.11, 0.30, 0.50));
    let court = mats.add(Color::srgb(0.24, 0.54, 0.82));
    let line_mat = mats.add(Color::srgb(0.94, 0.93, 0.88));
    let net_mat = mats.add(Color::srgb(0.45, 0.47, 0.5));

    commands.spawn((
        CourtMark,
        Mesh2d(meshes.add(uv_rect(38.0 * PPM, 24.0 * PPM))),
        MeshMaterial2d(apron),
        Transform::from_xyz(0.0, 0.0, -2.0),
    ));
    commands.spawn((
        CourtMark,
        Mesh2d(meshes.add(uv_rect(COURT_LENGTH * PPM, COURT_SINGLES_WIDTH * PPM))),
        MeshMaterial2d(court),
        Transform::from_xyz(0.0, 0.0, -1.0),
    ));

    // White lines: thin rectangles.
    let lw = 0.12 * PPM;
    let mut marks: Vec<(Handle<ColorMaterial>, Vec2, Vec3)> = Vec::new();
    let half_l = COURT_LENGTH / 2.0;
    let half_w = COURT_SINGLES_WIDTH / 2.0;
    let service_x = COURT_LENGTH / 4.0;
    // Baselines
    marks.push((line_mat.clone(), Vec2::new(lw, COURT_SINGLES_WIDTH * PPM), Vec3::new(wx(-half_l), 0.0, 0.5)));
    marks.push((line_mat.clone(), Vec2::new(lw, COURT_SINGLES_WIDTH * PPM), Vec3::new(wx(half_l), 0.0, 0.5)));
    // Sidelines
    marks.push((line_mat.clone(), Vec2::new(COURT_LENGTH * PPM, lw), Vec3::new(0.0, wz(-half_w), 0.5)));
    marks.push((line_mat.clone(), Vec2::new(COURT_LENGTH * PPM, lw), Vec3::new(0.0, wz(half_w), 0.5)));
    // Service lines (x = ±7)
    for sx in [-service_x, service_x] {
        marks.push((line_mat.clone(), Vec2::new(lw, COURT_SINGLES_WIDTH * PPM), Vec3::new(wx(sx), 0.0, 0.5)));
    }
    // Center service line (z = 0, between service lines)
    marks.push((line_mat.clone(), Vec2::new(2.0 * service_x * PPM, lw), Vec3::new(0.0, 0.0, 0.5)));
    for (mat, size, pos) in marks {
        commands.spawn((
            CourtMark,
            Mesh2d(meshes.add(uv_rect(size.x, size.y))),
            MeshMaterial2d(mat),
            Transform::from_xyz(pos.x, pos.y, pos.z),
        ));
    }

    // Net: grey slab across the middle.
    commands.spawn((
        CourtMark,
        Mesh2d(meshes.add(uv_rect(0.5 * PPM, COURT_DOUBLES_WIDTH * PPM))),
        MeshMaterial2d(net_mat),
        Transform::from_xyz(0.0, 0.0, 0.6),
    ));

    // Players (both sides) + their reach rings. Everything player-shaped is a
    // CIRCLE: the strike range (2.6 m) and perfect radius (1.0 m) are circles
    // in the world model, so drawing them as squares misrepresents them.
    for side in [Side::Home, Side::Away] {
        let x0 = side.sign() * COURT_LENGTH * 0.5;
        commands.spawn((
            PlayerViz(side),
            Mesh2d(meshes.add(Circle::new(0.8))),
            MeshMaterial2d(mats.add(Color::srgb(0.20, 0.35, 0.85))),
            Transform::from_xyz(wx(x0), 0.0, 1.0).with_scale(Vec3::splat(PPM)),
        ));
        commands.spawn((
            StrikeRing(side),
            Mesh2d(meshes.add(Circle::new(STRIKE_RADIUS))),
            MeshMaterial2d(mats.add(Color::srgba(1.0, 1.0, 1.0, 0.14))),
            Transform::from_xyz(wx(x0), 0.0, 0.9).with_scale(Vec3::splat(PPM)),
        ));
        commands.spawn((
            PerfectRing(side),
            Mesh2d(meshes.add(Circle::new(PERFECT_RADIUS))),
            MeshMaterial2d(mats.add(Color::srgba(0.2, 1.0, 0.3, 0.22))),
            Transform::from_xyz(wx(x0), 0.0, 0.95).with_scale(Vec3::splat(PPM)),
        ));
        commands.spawn((
            RacketDot(side),
            Mesh2d(meshes.add(Circle::new(0.35))),
            MeshMaterial2d(mats.add(Color::srgba(1.0, 0.9, 0.2, 0.9))),
            Transform::from_xyz(wx(x0), 0.0, 1.1).with_scale(Vec3::splat(PPM)),
        ));
        // Charge bar (below), stamina bar (above).
        commands.spawn((
            ChargeBar(side),
            Mesh2d(meshes.add(uv_rect(1.0, 1.0))),
            MeshMaterial2d(mats.add(Color::srgba(1.0, 0.85, 0.1, 0.9))),
            Transform::from_xyz(wx(x0), wz(-1.4), 1.1),
        ));
        commands.spawn((
            StaminaBar(side),
            Mesh2d(meshes.add(uv_rect(1.0, 1.0))),
            MeshMaterial2d(mats.add(Color::srgba(0.2, 1.0, 0.4, 0.9))),
            Transform::from_xyz(wx(x0), wz(1.4), 1.1),
        ));
    }

    // Ball + its ground shadow. The sim is fully 3D (gravity, bounce, net-tape
    // height, strike height); this view is top-down, so height is rendered as:
    // shadow = the ball's true court XZ (ground truth), ball disc = lifted
    // up-screen and grown with `pos.y` (closer to a top-down camera).
    commands.spawn((
        BallShadow,
        Mesh2d(meshes.add(Circle::new(BALL_RADIUS * 0.9))),
        MeshMaterial2d(mats.add(Color::srgba(0.0, 0.0, 0.0, 0.32))),
        Transform::from_xyz(0.0, 0.0, 1.5).with_scale(Vec3::splat(PPM)),
    ));
    commands.spawn((
        BallViz,
        Mesh2d(meshes.add(Circle::new(BALL_RADIUS))),
        MeshMaterial2d(mats.add(Color::srgb(0.98, 0.98, 0.9))),
        Transform::from_xyz(0.0, 0.0, 2.0).with_scale(Vec3::splat(PPM)),
    ));

    // Scoreboard: three set rings spread BETWEEN the score labels (not clumped
    // in the middle). Each ring = a pale outline body (always visible) + one
    // lit inner disc (hidden until its set is won). Ring colours: home = orange,
    // away = blue, centre = white (lit when the match is decided at 2 sets).
    let vh = mats.add(ColorMaterial {
        color: Color::srgba(1.0, 1.0, 1.0, 0.95),
        ..default()
    });
    let ve = mats.add(ColorMaterial {
        color: Color::srgba(1.0, 1.0, 1.0, 0.18),
        ..default()
    });
    let fh = mats.add(ColorMaterial {
        color: Color::srgba(0.95, 0.5, 0.1, 0.95),
        ..default()
    });
    let fa = mats.add(ColorMaterial {
        color: Color::srgba(0.15, 0.65, 0.98, 0.95),
        ..default()
    });
    for i in 0..3 {
        let x = (i as f32 - 1.0) * SET_RING_SPACING;
        commands.spawn((
            SetRingBody,
            Mesh2d(meshes.add(Circle::new(1.0))),
            MeshMaterial2d(ve.clone()),
            Transform::from_xyz(x, 11.0 * PPM, 1.0).with_scale(Vec3::splat(SET_RING_R)),
        ));
        let lit = if i == 0 {
            fh.clone()
        } else if i == 2 {
            fa.clone()
        } else {
            vh.clone()
        };
        commands.spawn((
            SetRingFill(i),
            Mesh2d(meshes.add(Circle::new(1.0))),
            MeshMaterial2d(lit),
            Transform::from_xyz(x, 11.0 * PPM, 1.1).with_scale(Vec3::splat(0.0)),
        ));
    }

    commands.spawn((
        ScoreText(Side::Home),
        Text2d::new("0"),
        Transform::from_xyz(-3.5 * PPM, 11.0 * PPM, 1.3)
            .with_scale(Vec3::splat(1.0)),
    ));
    commands.spawn((
        ScoreText(Side::Away),
        Text2d::new("0"),
        Transform::from_xyz(3.5 * PPM, 11.0 * PPM, 1.3),
    ));
    commands.spawn((
        PhaseText,
        Text2d::new(""),
        Transform::from_xyz(0.0, -11.0 * PPM, 1.3),
    ));
}

fn points_label(p: u32, adv: bool) -> String {
    match p {
        0 => "0".into(),
        1 => "15".into(),
        2 => "30".into(),
        3 => "40".into(),
        _ if adv => "Ad".into(),
        _ => "40".into(),
    }
}

fn sync(
    mut set: ParamSet<(
        Query<(&PlayerViz, &mut Transform)>,
        Query<(&StrikeRing, &mut Transform)>,
        Query<(&PerfectRing, &mut Transform)>,
        Query<(&RacketDot, &mut Transform)>,
        Query<(&ChargeBar, &mut Transform)>,
        Query<(&StaminaBar, &mut Transform)>,
        Query<&mut Transform, With<BallViz>>,
        Query<&mut Transform, With<BallShadow>>,
    )>,
    mut match_res: ResMut<Match>,
    time: Res<Time>,
) {
    let m = &mut *match_res;
    // Step the world in real time (fixed dt accumulator).
    m.accumulator += time.delta_secs();
    let mut steps = 0;
    while m.accumulator >= FIXED_DT && steps < 4 {
        let home_cmd = m.home.as_mut().and_then(|b| b.command_for(&m.world));
        let away_cmd = m.away.as_mut().and_then(|b| b.command_for(&m.world));
        m.world.step([home_cmd, away_cmd]);
        m.accumulator -= FIXED_DT;
        steps += 1;

        let pause = m.world.phase == Phase::PointPause;
        if pause && !m.in_pause {
            m.points_done += 1;
        }
        m.in_pause = pause;
    }

    let w = &m.world;
    for (viz, mut t) in set.p0().iter_mut() {
        let p = w.player(viz.0);
        t.translation.x = wx(p.pos.x);
        t.translation.y = wz(p.pos.y);
    }
    // Reach rings follow the RACKET CENTER — the point the world model
    // actually measures from (`ball.distance(racket_center()) <= STRIKE_RADIUS`)
    // — which sits RACKET_FORWARD toward the opponent of the player, not on
    // the player disc itself. (`racket()` is the public read of that point.)
    for (viz, mut t) in set.p1().iter_mut() {
        let r = w.player(viz.0).racket();
        t.translation.x = wx(r.x);
        t.translation.y = wz(r.z);
    }
    for (viz, mut t) in set.p2().iter_mut() {
        let r = w.player(viz.0).racket();
        t.translation.x = wx(r.x);
        t.translation.y = wz(r.z);
    }
    for (viz, mut t) in set.p3().iter_mut() {
        let p = w.player(viz.0);
        let r = p.racket();
        t.translation.x = wx(r.x);
        t.translation.y = wz(r.z);
    }
    for (viz, mut t) in set.p4().iter_mut() {
        let p = w.player(viz.0);
        let width = 1.6 * PPM * p.charge.clamp(0.02, 1.0);
        t.translation.x = wx(p.pos.x) - (1.6 * PPM - width) / 2.0;
        t.translation.y = wz(p.pos.y) - 1.3 * PPM;
        t.scale = Vec3::new(width, 0.22 * PPM, 1.0);
    }
    for (viz, mut t) in set.p5().iter_mut() {
        let p = w.player(viz.0);
        t.translation.x = wx(p.pos.x);
        t.translation.y = wz(p.pos.y) + 1.3 * PPM;
        t.scale = Vec3::new(1.6 * PPM, 0.22 * PPM, 1.0);
    }
    // Ball: lifted up-screen and grown with height (top-down camera: higher =
    // closer). The shadow stays at the true court XZ and shrinks with height,
    // so lobs / the toss / net-tape flights read as 3D, not as XZ teleports.
    const HEIGHT_LIFT: f32 = 14.0; // screen px per metre of height
    if let Ok(mut t) = set.p6().single_mut() {
        let y = w.ball.pos.y.max(0.0);
        t.translation.x = wx(w.ball.pos.x);
        t.translation.y = wz(w.ball.pos.z) - y * HEIGHT_LIFT;
        t.scale = Vec3::splat(PPM * (1.0 + 0.10 * y));
    }
    if let Ok(mut t) = set.p7().single_mut() {
        let y = w.ball.pos.y.max(0.0);
        t.translation.x = wx(w.ball.pos.x);
        t.translation.y = wz(w.ball.pos.z);
        t.scale = Vec3::splat(PPM * (1.0 - 0.06 * y.min(6.0)).max(0.35));
    }
}

fn sync_text(
    mut q_score: Query<(&ScoreText, &mut Text2d)>,
    mut q_phase: Query<&mut Text2d, (With<PhaseText>, Without<ScoreText>)>,
    mut q_fills: Query<(&SetRingFill, &mut Transform), Without<ScoreText>>,
    match_res: Res<Match>,
) {
    let w = &match_res.world;
    for (viz, mut t) in q_score.iter_mut() {
        let i = match viz.0 {
            Side::Home => 0,
            Side::Away => 1,
        };
        *t = Text2d::new(points_label(w.score.points[i], w.score.has_advantage(viz.0)));
    }
    if let Ok(mut t) = q_phase.single_mut() {
        *t = Text2d::new(format!(
            "{:?} · serve {:?} · serve #{}{} · ball y {:.2} m",
            w.phase,
            w.score.server(),
            w.score.serve_number,
            if w.score.ad_court() { " (ad court)" } else { "" },
            w.ball.pos.y
        ));
    }
    // Set rings: only the lit inner discs are touched here — the outline
    // bodies keep their spawn scale (the old code rescaled every ring
    // sub-spawn, which clumped the scoreboard into one blob).
    let sets = w.score.sets;
    for (fill, mut t) in q_fills.iter_mut() {
        let lit = match fill.0 {
            0 => sets[0] >= 1,
            2 => sets[1] >= 1,
            _ => sets[0] + sets[1] >= 2, // centre = match decided at 2 sets
        };
        t.scale = if lit {
            Vec3::splat(SET_RING_INNER)
        } else {
            Vec3::splat(0.0)
        };
    }
}

fn main() {
    let mut home = String::from("titanium54");
    let mut away = String::new();
    let mut seed: u64 = 7;
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--home" => home = args.next().unwrap_or(home),
            "--away" => away = args.next().unwrap_or(away),
            "--seed" => seed = args.next().and_then(|v| v.parse().ok()).unwrap_or(seed),
            _ => {}
        }
    }

    let (home_brain, _, _) = load_bot(&home, TeamId::Home)
        .map(|(b, u, a)| (Some(b), u, a))
        .unwrap_or((None, Vec::new(), Vec::new()));
    let away_brain = load_bot(&away, TeamId::Away).map(|(b, _, _)| b);

    let world = TennisWorld::new(seed);
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "AIA tennis sim — viewer".into(),
                resolution: (1280, 760).into(),
                ..default()
            }),
            ..default()
        }))
        .insert_resource(ClearColor(Color::srgb(0.08, 0.24, 0.40)))
        .insert_resource(Match {
            world,
            home: home_brain,
            away: away_brain,
            accumulator: 0.0,
            points_done: 0,
            in_pause: false,
        })
        .add_systems(Startup, setup)
        .add_systems(Update, (sync, sync_text))
        .run();
}
