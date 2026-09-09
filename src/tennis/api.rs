//! Tennis dense catalogs + team API snapshot.
//!
//! Same architecture as soccer's `api::dense` (label ↔ u16, no per-tick
//! string maps) but with the tennis label tables pinned to
//! `AIGamePyLibrary.data.DROPDOWN_OPTIONS` — the builder's order, which is
//! what graph JSON indices mean. Coverage tests pin counts and spot indices.

use std::collections::HashMap;
use std::sync::OnceLock;

use bevy::prelude::Vec2;

use crate::api::DenseTeamApi;
use crate::brain::TeamId;
use crate::graph::dropdowns::{
    TENNIS_GET_BOOL, TENNIS_GET_FLOAT, TENNIS_GET_TRANSFORM, TENNIS_GET_VECTOR3,
};

fn index_map<'a>(labels: &[&'a str]) -> HashMap<&'a str, u16> {
    labels
        .iter()
        .enumerate()
        .map(|(i, l)| (*l, i as u16))
        .collect()
}

fn bool_map() -> &'static HashMap<&'static str, u16> {
    static M: OnceLock<HashMap<&'static str, u16>> = OnceLock::new();
    M.get_or_init(|| index_map(TENNIS_GET_BOOL))
}

fn float_map() -> &'static HashMap<&'static str, u16> {
    static M: OnceLock<HashMap<&'static str, u16>> = OnceLock::new();
    M.get_or_init(|| index_map(TENNIS_GET_FLOAT))
}

fn transform_map() -> &'static HashMap<&'static str, u16> {
    static M: OnceLock<HashMap<&'static str, u16>> = OnceLock::new();
    M.get_or_init(|| index_map(TENNIS_GET_TRANSFORM))
}

fn vector_map() -> &'static HashMap<&'static str, u16> {
    static M: OnceLock<HashMap<&'static str, u16>> = OnceLock::new();
    M.get_or_init(|| index_map(TENNIS_GET_VECTOR3))
}

pub fn bool_index(label: &str) -> Option<u16> {
    bool_map().get(label).copied()
}
pub fn float_index(label: &str) -> Option<u16> {
    float_map().get(label).copied()
}
pub fn transform_index(label: &str) -> Option<u16> {
    transform_map().get(label).copied()
}
pub fn vector_index(label: &str) -> Option<u16> {
    vector_map().get(label).copied()
}

pub fn bool_count() -> usize {
    TENNIS_GET_BOOL.len()
}
pub fn float_count() -> usize {
    TENNIS_GET_FLOAT.len()
}
pub fn transform_count() -> usize {
    TENNIS_GET_TRANSFORM.len()
}
pub fn vector_count() -> usize {
    TENNIS_GET_VECTOR3.len()
}

/// Empty tennis dense snapshot (tennis catalog sizes).
pub fn empty_team_api(team: TeamId) -> DenseTeamApi {
    DenseTeamApi::empty_with_counts(
        team,
        bool_count(),
        float_count(),
        transform_count(),
        vector_count(),
    )
}

/// Which side the graph brain is looking from.
pub fn build_team_api(world: &super::world::TennisWorld, side: super::court::Side) -> DenseTeamApi {
    let team = match side {
        super::court::Side::Home => TeamId::Home,
        super::court::Side::Away => TeamId::Away,
    };
    let opponent = side.other();
    let mut api = empty_team_api(team);

    let set = |api: &mut DenseTeamApi, label: &str, v: bool| {
        if let Some(i) = bool_index(label) {
            api.set_bool_id(i, v);
        }
    };
    let setf = |api: &mut DenseTeamApi, label: &str, v: f32| {
        if let Some(i) = float_index(label) {
            api.set_float_id(i, v);
        }
    };
    let setv = |api: &mut DenseTeamApi, label: &str, v: Vec2| {
        if let Some(i) = vector_index(label) {
            api.set_vector_id(i, Some(v));
        }
    };
    let sett = |api: &mut DenseTeamApi, label: &str, v: Vec2| {
        if let Some(i) = transform_index(label) {
            api.set_transform_id(i, v);
        }
    };

    let server = world.score.server();
    let receiver = world.score.receiver();
    let ad = world.score.ad_court();
    let i = side as usize;
    let o = opponent as usize;
    let me = world.player(side);
    let opp = world.player(opponent);

    // ---- Bools (35) ----
    set(&mut api, "Is Playing", world.end.is_none());
    set(&mut api, "Is Home", side == super::court::Side::Home);
    set(&mut api, "Is Self Server For Set", server == side);
    set(&mut api, "Is Opponent Server For Set", server == opponent);
    set(&mut api, "Is Serve Phase", matches!(world.phase, super::world::Phase::ServeSetup | super::world::Phase::Toss));
    set(&mut api, "Is Self Actively Serving", server == side && world.phase == super::world::Phase::Toss);
    set(&mut api, "Is Second Serve", world.score.serve_number == 2);
    set(&mut api, "Is Ad Court Serve", ad);
    set(&mut api, "Is Ball Playable", !world.ball_held && world.end.is_none());
    set(&mut api, "Was Last Shot Topspin", world.last_shot_idx(i) == Some(super::shot_type::ShotType::Topspin));
    set(&mut api, "Was Last Shot Slice", world.last_shot_idx(i) == Some(super::shot_type::ShotType::Slice));
    set(&mut api, "Was Last Shot Flat", world.last_shot_idx(i) == Some(super::shot_type::ShotType::Flat));
    set(&mut api, "Was Last Shot Lob", world.last_shot_idx(i) == Some(super::shot_type::ShotType::Lob));
    set(&mut api, "Was Last Shot Drop", world.last_shot_idx(i) == Some(super::shot_type::ShotType::Drop));
    set(&mut api, "Was Last Shot Curve Left", world.last_shot_idx(i) == Some(super::shot_type::ShotType::CurveLeft));
    set(&mut api, "Was Last Shot Curve Right", world.last_shot_idx(i) == Some(super::shot_type::ShotType::CurveRight));
    set(&mut api, "Was Last Shot Trick", world.last_shot_idx(i) == Some(super::shot_type::ShotType::Trick));
    set(&mut api, "Is Self Charging", world.is_charging(side));
    set(&mut api, "Is Opponent Charging", world.is_charging(opponent));
    set(&mut api, "Is Self Winning", world.score.is_winning(side));
    set(&mut api, "Is Opponent Winning", world.score.is_winning(opponent));
    set(&mut api, "Is Tied", world.score.points[i] == world.score.points[1 - i]);
    set(&mut api, "Is Deuce", world.score.is_deuce());
    set(&mut api, "Is Game Point", world.score.is_game_point(side));
    set(&mut api, "Is Break Point", world.score.is_break_point() && side == receiver);
    set(&mut api, "Is Match Point", world.score.is_match_point(side));
    set(&mut api, "Ball On Self Side", world.ball.pos.x * side.sign() > 0.0);
    set(&mut api, "Ball Incoming", world.ball.pos.x * side.sign() < 0.0 || (world.serve_in_flight && server == opponent));
    set(&mut api, "Ball In Swing Range", world_reaches(world, side));
    set(&mut api, "Ball Has Bounced", world.ball.bounces > 0 && !world.ball_held);
    set(&mut api, "Must Wait For Bounce", world.serve_in_flight && !world.serve_bounced && side == receiver);
    set(&mut api, "Ball Has Charged Effect", world.ball.charge >= 0.75);
    set(&mut api, "Self Scored Last Point", world.last_point_winner() == Some(side));
    set(&mut api, "Opponent Scored Last Point", world.last_point_winner() == Some(opponent));
    set(&mut api, "Self Has Advantage", world.score.has_advantage(side));

    // ---- Floats (48) ----
    for (n, label) in [
        "Shot: Topspin",
        "Shot: Slice",
        "Shot: Flat",
        "Shot: Drop",
        "Shot: Lob",
        "Shot: Curve Left",
        "Shot: Curve Right",
        "Shot: Random",
    ]
    .into_iter()
    .enumerate()
    {
        setf(&mut api, label, n as f32);
    }
    setf(&mut api, "Shot: Last Self Shot", world.last_shot_dropdown(side));
    setf(&mut api, "Shot: Last Opponent Shot", world.last_shot_dropdown(opponent));
    setf(&mut api, "Shot: Most Used Self Shot", 2.0);
    setf(&mut api, "Shot: Most Used Opponent Shot", 2.0);
    setf(&mut api, "Shot: Most Scored Self Shot", 2.0);
    setf(&mut api, "Shot: Most Scored Opponent Shot", 2.0);
    setf(&mut api, "Shot: Ball", world.ball.shot as usize as f32);
    setf(&mut api, "Self Swing Charge Pct", world.charge_pct(side));
    setf(&mut api, "Opponent Swing Charge Pct", world.charge_pct(opponent));
    setf(&mut api, "Self Stamina Pct", 1.0);
    setf(&mut api, "Opponent Stamina Pct", 1.0);
    setf(&mut api, "Deuce Fatigue", 0.0);
    setf(&mut api, "Rally Fatigue", 0.0);
    setf(&mut api, "Court Width", TENNIS_COURT_WIDTH);
    setf(&mut api, "Court Depth", TENNIS_COURT_DEPTH);
    setf(&mut api, "Net Height", TENNIS_NET_HEIGHT);
    setf(&mut api, "Ball Speed", world.ball_speed());
    let bounce1 = world.predicted_bounce(0);
    let bounce2 = world.predicted_bounce(1);
    setf(&mut api, "Ball Time To Ground", bounce1.map(|b| b.2).unwrap_or(0.0));
    setf(&mut api, "Ball Time To 2nd Bounce", bounce2.map(|b| b.2).unwrap_or(0.0));
    setf(
        &mut api,
        "Self Time To Destination",
        me.pos.distance(world_aim_target(world, side)) / super::params::WALK_SPEED.max(0.001),
    );
    setf(&mut api, "Serve Number", world.score.serve_number as f32);
    setf(&mut api, "Self Points", world.score.points[i] as f32);
    setf(&mut api, "Opponent Points", world.score.points[o] as f32);
    setf(&mut api, "Self Set Score", world.score.sets[i] as f32);
    setf(&mut api, "Opponent Set Score", world.score.sets[o] as f32);
    setf(&mut api, "Self Aces", world.score.aces[i] as f32);
    setf(&mut api, "Opponent Aces", world.score.aces[o] as f32);
    setf(&mut api, "Self Faults", world.score.faults[i] as f32);
    setf(&mut api, "Opponent Faults", world.score.faults[o] as f32);
    setf(&mut api, "Self Double Faults", world.score.double_faults[i] as f32);
    setf(&mut api, "Opponent Double Faults", world.score.double_faults[o] as f32);
    setf(&mut api, "Self Fouls", world.score.fouls[i] as f32);
    setf(&mut api, "Opponent Fouls", world.score.fouls[o] as f32);
    setf(&mut api, "Self Outs", world.score.outs[i] as f32);
    setf(&mut api, "Opponent Outs", world.score.outs[o] as f32);
    setf(&mut api, "Self Charged Shots", world.score.charged_shots[i] as f32);
    setf(&mut api, "Opponent Charged Shots", world.score.charged_shots[o] as f32);
    setf(&mut api, "Current Simulation Time", world.sim_time);
    setf(&mut api, "Sim Tick", world.tick as f32);
    setf(&mut api, "Delta Time", 0.0);
    setf(&mut api, "Fixed Delta Time", super::params::FIXED_DT);

    // ---- Vectors (19, pitch-plane) ----
    setv(&mut api, "Ball Position", Vec2::new(world.ball.pos.x, world.ball.pos.z));
    setv(&mut api, "Ball Velocity", Vec2::new(world.ball.vel.x, world.ball.vel.z));
    if let Some(b) = bounce1 {
        setv(&mut api, "Predicted Bounce", Vec2::new(b.0, b.1));
    }
    if let Some(b) = bounce2 {
        setv(&mut api, "Predicted 2nd Bounce", Vec2::new(b.0, b.1));
    }
    setv(&mut api, "Center Of Half", super::court::center_of_half(side));
    setv(&mut api, "Center Of Back", super::court::center_of_back(side));
    setv(&mut api, "Serve Stance", super::court::serve_stance(server, ad));
    setv(&mut api, "Receive Stance", super::court::receive_stance(server, ad));
    setv(&mut api, "Legal Serve Target", super::court::legal_serve_target(receiver, ad));
    setv(
        &mut api,
        "Random Aim Target",
        Vec2::new(world.random_aim_target().x, world.random_aim_target().z),
    );
    setv(&mut api, "Self Average Scoring Location", super::court::default_aim_target(side));
    setv(&mut api, "Opponent Average Scoring Location", super::court::default_aim_target(opponent));
    // Estimated Opponent Shot Location / Trick modifiers: null for now.
    setv(&mut api, "Camera Forward", Vec2::new(opponent.sign(), 0.0));
    setv(&mut api, "Camera Right", Vec2::new(0.0, -opponent.sign()));

    // ---- Transforms (5) ----
    sett(&mut api, "Self", me.pos);
    sett(&mut api, "Opponent", opp.pos);
    let racket = me.racket();
    sett(&mut api, "Self Racket Center", Vec2::new(racket.x, racket.z));
    sett(&mut api, "Ball", Vec2::new(world.ball.pos.x, world.ball.pos.z));
    sett(&mut api, "Camera", me.pos);

    api
}

fn world_reaches(world: &super::world::TennisWorld, side: super::court::Side) -> bool {
    let p = world.player(side);
    let racket = p.racket();
    world.ball.pos.distance(racket) <= super::params::STRIKE_RADIUS
}

fn world_aim_target(world: &super::world::TennisWorld, side: super::court::Side) -> Vec2 {
    world.player(side).last_aim
}

// Court constants re-exported for the sensor block above.
use super::params::{
    COURT_LENGTH as TENNIS_COURT_DEPTH, COURT_SINGLES_WIDTH as TENNIS_COURT_WIDTH,
    NET_HEIGHT as TENNIS_NET_HEIGHT,
};
