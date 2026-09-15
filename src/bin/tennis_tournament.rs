//! Headless tennis tournament match: two graph bots (or stock) play N points
//! in the tennis world; prints a JSON summary line for batch collection.
//!
//! Usage:
//!   tennis_tournament --home titanium54 --away aia3 --seed 7 --points 4
//!   tennis_tournament --home titanium54 --away "" --seed 7   (away = stock)
//! Optional --trace out.jsonl dumps per-tick state (sim "timeplots").

use aia_comp_sim::tennis::vm::{BrainSide, TennisBrain};
use aia_comp_sim::tennis::world::{Phase, TennisWorld};
use aia_comp_sim::tennis::court::Side;
use aia_comp_sim::tennis::score::MatchRules;

thread_local! {
    static UNIMPLEMENTED: std::cell::RefCell<Vec<String>> =
        const { std::cell::RefCell::new(Vec::new()) };
    static APPROXIMATED: std::cell::RefCell<Vec<String>> =
        const { std::cell::RefCell::new(Vec::new()) };
}

fn saves_dir() -> std::path::PathBuf {
    let base = std::env::var("USERPROFILE").expect("USERPROFILE");
    std::path::PathBuf::from(base)
        .join(r"AppData\LocalLow\Unicorn One\AIComp\Saves\Tennis")
}

fn load_bot(
    name: &str,
    team: aia_comp_sim::brain::TeamId,
) -> Option<(BrainSide, Vec<String>, Vec<String>)> {
    if name.is_empty() || name == "stock" || name == "none" {
        return None;
    }
    let path = saves_dir().join(format!("{name}.txt"));
    let graph = aia_comp_sim::graph::load::load_graph(
        &path,
        Some(aia_comp_sim::mode::GameSpec::tennis_builder()),
    )
    .unwrap_or_else(|e| panic!("load {name}: {e}"));
    aia_comp_sim::graph_vm::diagnostics::clear();
    let brain = TennisBrain::compile(graph);
    let stubbed = aia_comp_sim::graph_vm::diagnostics::stubbed();
    let unimplemented: Vec<String> = stubbed
        .iter()
        .filter(|(_, t)| *t == aia_comp_sim::graph_vm::diagnostics::Trust::Unimplemented)
        .map(|(n, _)| n.clone())
        .collect();
    let approximated: Vec<String> = stubbed
        .iter()
        .filter(|(_, t)| *t == aia_comp_sim::graph_vm::diagnostics::Trust::Approximated)
        .map(|(n, _)| n.clone())
        .collect();
    if !unimplemented.is_empty() {
        eprintln!("WARN {name}: unimplemented node types {unimplemented:?}");
    }
    // Stash reported via the return value — the match summary includes it.
    Some((BrainSide { brain, team }, unimplemented, approximated))
}

fn main() {
    let mut home_name = String::from("titanium54");
    let mut away_name = String::new();
    let mut seed: u64 = 7;
    let mut points: usize = 4;
    let mut max_ticks: u64 = 120_000;
    let mut trace: Option<String> = None;
    // Per-strike contact log (tier + contact geometry + aim, one JSON per
    // strike). Analysis joins each strike with its first-bounce landing.
    let mut trace_strikes: Option<String> = None;
    // Optional custom format config. WITHOUT it the sim enforces official
    // rules (best-of-3, cross-set serve rotation, per-point side alternation).
    let mut rules_path: Option<String> = None;
    // World model version. DEFAULT v015 = the live game's model: the v0.15
    // free and Patreon builds share the world model, and its sole delta from
    // v0.14 is the swept (frame-interpolated) ball contact, which
    // `TennisWorld::for_spec` turns on. Running the old v0.14 default is what
    // made the sim report "titanium66 beats Unlucky 2-1" while the real game
    // bagelled it 6-0 (2026-09-14). Pass `--game-version v014` only to
    // reproduce pre-2026-09-14 rows.
    let mut game_version = String::from("v015");
    // Per-point outcome log (winner + reason + striker + ball), see PointEvent.
    let mut trace_points: Option<String> = None;
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--home" => home_name = args.next().unwrap_or(home_name),
            "--away" => away_name = args.next().unwrap_or(away_name),
            "--seed" => seed = args.next().and_then(|v| v.parse().ok()).unwrap_or(seed),
            "--points" => points = args.next().and_then(|v| v.parse().ok()).unwrap_or(points),
            "--max-ticks" => max_ticks = args.next().and_then(|v| v.parse().ok()).unwrap_or(max_ticks),
            "--trace" => trace = args.next(),
            "--trace-strikes" => trace_strikes = args.next(),
            "--rules" => rules_path = args.next(),
            "--game-version" => game_version = args.next().unwrap_or(game_version),
            "--trace-points" => trace_points = args.next(),
            _ => {}
        }
    }

    let (home_brain, home_unimpl, home_approx) =
        load_bot(&home_name, aia_comp_sim::brain::TeamId::Home)
            .map(|(b, u, a)| (Some(b), u, a))
            .unwrap_or((None, Vec::new(), Vec::new()));
    let mut home = home_brain;
    let mut away: Option<BrainSide> =
        load_bot(&away_name, aia_comp_sim::brain::TeamId::Away).map(|(b, _, _)| b);

    let (spec, world_spec_name): (aia_comp_sim::mode::GameSpec, &str) =
        match game_version.as_str() {
            "v014" => (aia_comp_sim::mode::GameSpec::tennis_v014(), "v014"),
            "v015" | "latest" => (aia_comp_sim::mode::GameSpec::tennis_v015(), "v015"),
            other => panic!("--game-version: unknown {other:?} (expected v014 | v015)"),
        };

    let mut world = match rules_path {
        Some(p) => {
            let rules = MatchRules::load_json(std::path::Path::new(&p))
                .unwrap_or_else(|e| panic!("--rules: {e}"));
            TennisWorld::for_spec_with_rules(seed, spec, rules)
        }
        None => TennisWorld::for_spec(seed, spec),
    };
    if trace_strikes.is_some() {
        std::env::set_var("AIA_STRIKE_LOG", "1");
    }
    let mut trace_out: Option<std::io::BufWriter<std::fs::File>> = trace
        .as_ref()
        .map(|p| std::io::BufWriter::new(std::fs::File::create(p).expect("trace file")));

    let mut completed_points = 0usize;
    let mut in_pause = false;
    // Optional per-tick graph-channel capture (TimePlot sinks): set
    // AIA_TRACE_CHANNELS=1 to append "hch"/"ach" {name:value} maps per tick.
    // Lets sim traces speak the same channel language as the game's native
    // timeplot exports (channel-diff without same-seed replay).
    let trace_channels: bool = std::env::var("AIA_TRACE_CHANNELS").as_deref() == Ok("1");
    fn side_channels() -> String {
        let snap = aia_comp_sim::debug_draw::snapshot();
        let mut out = String::from("{");
        let mut first = true;
        for (k, v) in &snap.plots {
            if !first {
                out.push(',');
            }
            first = false;
            out.push('"');
            out.push_str(&k.replace('"', "'"));
            out.push_str("\":");
            out.push_str(&format!("{v:.4}"));
        }
        out.push('}');
        out
    }
    // Per-point winner log for exact outcome-parity comparison with the
    // game's point_winners[] (0=home, 1=away). award_point resets points on
    // a game win, so winners are detected from per-step deltas: a games
    // increment marks a game-winning point; otherwise a points increment.
    let mut point_winners: Vec<u8> = Vec::new();
    while world.end.is_none() && world.tick < max_ticks && completed_points < points {
        let prev_points = world.score.points;
        let prev_games = world.score.games;
        let (home_cmd, home_ch, away_cmd, away_ch) = if trace_channels {
            aia_comp_sim::debug_draw::begin_frame();
            let hc = home.as_mut().and_then(|b| b.command_for(&world));
            let hch = side_channels();
            aia_comp_sim::debug_draw::begin_frame();
            let ac = away.as_mut().and_then(|b| b.command_for(&world));
            let ach = side_channels();
            (hc, hch, ac, ach)
        } else {
            let hc = home.as_mut().and_then(|b| b.command_for(&world));
            let ac = away.as_mut().and_then(|b| b.command_for(&world));
            (hc, String::from("{}"), ac, String::from("{}"))
        };
        let home_cmd = home_cmd;
        let away_cmd = away_cmd;
        world.step([home_cmd, away_cmd]);

        if world.score.games[0] > prev_games[0] {
            point_winners.push(0);
        } else if world.score.games[1] > prev_games[1] {
            point_winners.push(1);
        } else if world.score.points[0] > prev_points[0] {
            point_winners.push(0);
        } else if world.score.points[1] > prev_points[1] {
            point_winners.push(1);
        }

        if let Some(w) = trace_out.as_mut() {
            use std::io::Write;
            if trace_channels {
                let _ = writeln!(
                    w,
                    "{{\"tick\":{},\"phase\":\"{:?}\",\"ball\":[{:.4},{:.4},{:.4}],\"vel\":[{:.4},{:.4},{:.4}],\"bounces\":{},\"points\":[{},{}],\"games\":[{},{}],\"serve\":\"{:?}\",\"home\":[{:.3},{:.3}],\"away\":[{:.3},{:.3}],\"hs\":{},\"as\":{},\"hc\":{:.2},\"ac\":{:.2},\"haim\":[{:.2},{:.2}],\"aaim\":[{:.2},{:.2}],\"hch\":{},\"ach\":{}}}",
                    world.tick,
                    world.phase,
                    world.ball.pos.x, world.ball.pos.y, world.ball.pos.z,
                    world.ball.vel.x, world.ball.vel.y, world.ball.vel.z,
                    world.ball.bounces,
                    world.score.points[0], world.score.points[1],
                    world.score.games[0], world.score.games[1],
                    world.score.server(),
                    world.player(Side::Home).pos.x, world.player(Side::Home).pos.y,
                    world.player(Side::Away).pos.x, world.player(Side::Away).pos.y,
                    world.player(Side::Home).holding as u8, world.player(Side::Away).holding as u8,
                    world.player(Side::Home).charge, world.player(Side::Away).charge,
                    world.last_cmd_aim[0].x, world.last_cmd_aim[0].y,
                    world.last_cmd_aim[1].x, world.last_cmd_aim[1].y,
                    home_ch, away_ch,
                );
            } else {
                let _ = writeln!(
                    w,
                    "{{\"tick\":{},\"phase\":\"{:?}\",\"ball\":[{:.4},{:.4},{:.4}],\"vel\":[{:.4},{:.4},{:.4}],\"bounces\":{},\"points\":[{},{}],\"games\":[{},{}],\"serve\":\"{:?}\",\"home\":[{:.3},{:.3}],\"away\":[{:.3},{:.3}],\"hs\":{},\"as\":{},\"hc\":{:.2},\"ac\":{:.2},\"haim\":[{:.2},{:.2}],\"aaim\":[{:.2},{:.2}]}}",
                    world.tick,
                    world.phase,
                    world.ball.pos.x, world.ball.pos.y, world.ball.pos.z,
                    world.ball.vel.x, world.ball.vel.y, world.ball.vel.z,
                    world.ball.bounces,
                    world.score.points[0], world.score.points[1],
                    world.score.games[0], world.score.games[1],
                    world.score.server(),
                    world.player(Side::Home).pos.x, world.player(Side::Home).pos.y,
                    world.player(Side::Away).pos.x, world.player(Side::Away).pos.y,
                    world.player(Side::Home).holding as u8, world.player(Side::Away).holding as u8,
                    world.player(Side::Home).charge, world.player(Side::Away).charge,
                    world.last_cmd_aim[0].x, world.last_cmd_aim[0].y,
                    world.last_cmd_aim[1].x, world.last_cmd_aim[1].y,
                );
            }
        }

        let pause = world.phase == Phase::PointPause;
        if pause && !in_pause {
            completed_points += 1;
        }
        in_pause = pause;
    }

    let winner = world.end.map(|e| match e {
        aia_comp_sim::tennis::EndReason::SetWon(s) => match s {
            Side::Home => home_name.clone(),
            Side::Away => away_name.clone(),
        },
    });

    let winners_json = serde_json::to_string(&point_winners).unwrap_or_default();
    // Per-point outcome classification (winner x reason) — the ladder could
    // not say WHY a point was lost before this; now every row carries it.
    let mut reason_tally: std::collections::BTreeMap<&'static str, [u32; 2]> =
        std::collections::BTreeMap::new();
    for e in &world.point_log {
        let slot = reason_tally.entry(e.reason.label()).or_insert([0, 0]);
        slot[e.winner as usize] += 1;
    }
    let reasons_json = serde_json::to_string(
        &reason_tally
            .iter()
            .map(|(k, v)| ((*k).to_string(), vec![v[0], v[1]]))
            .collect::<Vec<_>>(),
    )
    .unwrap_or_default();
    if let Some(p) = trace_points.as_ref() {
        use std::io::Write;
        let mut f = std::io::BufWriter::new(std::fs::File::create(p).expect("points file"));
        for e in &world.point_log {
            let striker = match e.striker {
                Some(Side::Home) => "home",
                Some(Side::Away) => "away",
                None => "none",
            };
            let _ = writeln!(
                f,
                "{{\"tick\":{},\"winner\":\"{}\",\"reason\":\"{}\",\"striker\":\"{}\",\"ball\":[{:.3},{:.3},{:.3}],\"bounces\":{},\"rally_hits\":{},\"sets\":[{},{}],\"games\":[{},{}],\"points\":[{},{}]}}",
                e.tick,
                if e.winner == Side::Home { "home" } else { "away" },
                e.reason.label(),
                striker,
                e.ball[0], e.ball[1], e.ball[2],
                e.bounces,
                e.rally_hits,
                e.sets[0], e.sets[1],
                e.games[0], e.games[1],
                e.points[0], e.points[1],
            );
        }
    }
    if let Some(p) = trace_strikes.as_ref() {
        use std::io::Write;
        let mut f = std::io::BufWriter::new(std::fs::File::create(p).expect("strikes file"));
        for s in &world.strike_log {
            let _ = writeln!(
                f,
                "{{\"tick\":{},\"side\":{},\"serving\":{},\"tier\":\"{}\",\"ball\":[{:.3},{:.3},{:.3}],\"racket\":[{:.3},{:.3},{:.3}],\"aim\":[{:.2},{:.2}],\"q\":{:.2}}}",
                s.tick, s.side, s.serving, s.tier,
                s.ball[0], s.ball[1], s.ball[2],
                s.racket[0], s.racket[1], s.racket[2],
                s.aim[0], s.aim[1], s.charge,
            );
        }
    }
    println!(
        "{{\"home\":\"{}\",\"away\":\"{}\",\"seed\":{},\"game_version\":\"{}\",\"ticks\":{},\"points_played\":{},\"score_pts\":[{},{}],\"games\":[{},{}],\"sets\":[{},{}],\"point_winners\":{},\"aces\":[{},{}],\"faults\":[{},{}],\"double_faults\":[{},{}],\"point_reasons\":{},\"finished\":{},\"winner\":\"{}\",\"home_unimplemented\":{},\"home_approximated\":{}}}",
        home_name,
        away_name,
        seed,
        world_spec_name,
        world.tick,
        completed_points,
        world.score.points[0], world.score.points[1],
        world.score.games[0], world.score.games[1],
        world.score.sets[0], world.score.sets[1],
        winners_json,
        world.score.aces[0], world.score.aces[1],
        world.score.faults[0], world.score.faults[1],
        world.score.double_faults[0], world.score.double_faults[1],
        reasons_json,
        world.end.is_some(),
        winner.unwrap_or_else(|| "none".to_string()),
        serde_json::to_string(&home_unimpl).unwrap_or_default(),
        serde_json::to_string(&home_approx).unwrap_or_default(),
    );
}
