"""Multi-seed sim sweep matched to the game's restart sweep.

The game side (`AIA_tennis/modhost/restart_sweep.py`) restarts ONE live match
per seed (no relaunch) and records, per seed, that match's `point_winners` and
its setup `serving_team`. This replays exactly those (home, away, seed) triples
headless with the SAME points target and the SAME first server
(`AIA_FIRST_SERVER`), so outcome *distributions* over seeds can be compared
apples-to-apples.

Why a distribution and not only the exact sequence: while RNG draw sites and
the seed->first-server mapping are still being pinned, any single world-model
divergence shifts the RNG stream and flips later points, so exact per-seed
equality is a brittle headline (HANDOFF s11: "exact metric unusable"). The
*shape* (home win rate, server-hold rate, point distribution) is stable and is
the metric this harness feeds; `score_distribution.py` reports both.

Reads:  --game  (default modhost/restart_sweep.jsonl)
Writes: data/tennis/sim_seed_sweep.jsonl   (append, resume-safe)

Usage:
  python scripts/run_sim_seed_sweep.py
  python scripts/run_sim_seed_sweep.py --game path\\to\\restart_sweep.jsonl --home titanium54
"""

from __future__ import annotations

import argparse
import io
import json
import os
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
sys.path.insert(0, HERE)
from run_sim_tournament_pairs import match_result, game_leader_from_row  # noqa: E402

GAME_SWEEP = r"C:\gitProjects\AIA_tennis\modhost\restart_sweep.jsonl"
OUT = os.path.join(ROOT, "data", "tennis", "sim_seed_sweep.jsonl")
BIN = ["cargo", "run", "-q", "--bin", "tennis_tournament", "--"]


def load_game_rows(path: str) -> list[dict]:
    """Completed, correctly-attributed game restart-sweep rows.

    `restart_epoch` is the attribution guard: the pre-fix driver read the boot
    match's snapshot for the first sweep seed (same seed as launch). Rows
    without an epoch cannot be trusted for (seed -> first_server), so they are
    skipped loudly rather than silently mislabelled.
    """
    rows = []
    skipped_stale = 0
    for line in io.open(path, encoding="utf-8", errors="replace"):
        if not line.strip():
            continue
        r = json.loads(line)
        st = r.get("state") or {}
        if st.get("restart_epoch") is None:
            skipped_stale += 1
            continue
        if not st.get("done") or not st.get("point_winners"):
            continue
        if st.get("point_winners_truncated"):
            continue
        rows.append({
            "home": r.get("home"),
            "away": r.get("away"),
            "seed": int(r.get("seed")),
            "points": int(r.get("points_target") or st.get("points_target") or 8),
            "first_server": r.get("serving_team"),
            "game_winners": [w for w in st.get("point_winners", [])
                             if w != 4294967295],
        })
    if skipped_stale:
        print(f"  note: skipped {skipped_stale} pre-fix rows (no restart_epoch)",
              file=sys.stderr)
    return rows


def load_done() -> set:
    done = set()
    try:
        for line in io.open(OUT, encoding="utf-8", errors="replace"):
            if line.strip():
                r = json.loads(line)
                done.add((r.get("home"), r.get("away"), str(r.get("seed")),
                          r.get("first_server", "sim-default")))
    except FileNotFoundError:
        pass
    return done


def first_server_env(srv):
    if srv == 0:
        return "home"
    if srv == 1:
        return "away"
    return None


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--game", default=GAME_SWEEP,
                    help="game restart-sweep jsonl (per-seed outcomes)")
    ap.add_argument("--home", default=None,
                    help="only replay pairings with this home bot")
    ap.add_argument("--max-ticks", type=int, default=120000)
    ap.add_argument("--timeout", type=float, default=300.0)
    a = ap.parse_args()

    rows = load_game_rows(a.game)
    if a.home:
        rows = [r for r in rows if r["home"] == a.home]
    print(f"game seeds to replay: {len(rows)}")
    os.makedirs(os.path.dirname(OUT), exist_ok=True)
    done = load_done()
    print(f"already scored: {len(done)}")

    agree = disagree = failed = skipped = 0
    with io.open(OUT, "a", encoding="utf-8") as out:
        for r in rows:
            home, away, seed = r["home"], r["away"], r["seed"]
            points = r["points"]
            fs = first_server_env(r["first_server"])
            fs_key = fs or "sim-default"
            if (home, away, str(seed), fs_key) in done:
                skipped += 1
                continue

            env = os.environ.copy()
            if fs:
                env["AIA_FIRST_SERVER"] = fs
            else:
                env.pop("AIA_FIRST_SERVER", None)

            rec = {"home": home, "away": away, "seed": str(seed),
                   "points": points, "first_server": fs_key,
                   "game_winners": r["game_winners"]}
            try:
                proc = subprocess.run(
                    BIN + ["--home", home, "--away", away, "--seed", str(seed),
                           "--points", str(points),
                           "--max-ticks", str(a.max_ticks)],
                    capture_output=True, text=True, timeout=a.timeout, env=env,
                )
                line = proc.stdout.strip().splitlines()[-1] if proc.stdout.strip() else ""
                if proc.returncode != 0 or not line.startswith("{"):
                    err = [l for l in proc.stderr.strip().splitlines()
                           if "RUST_BACKTRACE" not in l and l.strip()]
                    rec["error"] = (err[-1] if err else "?")[:200]
                    failed += 1
                else:
                    sim = json.loads(line)
                    s_seq = [int(w) for w in sim.get("point_winners", [])]
                    rec["sim_winners"] = s_seq
                    rec["sim_games"] = sim.get("games")
                    rec["sim_pts"] = sim.get("score_pts")
                    rec["sim_ticks"] = sim.get("ticks")
                    g_seq = r["game_winners"]
                    if g_seq and g_seq == s_seq:
                        rec["parity"] = "agree-seq"
                        agree += 1
                    else:
                        hg, ag, hp, ap = match_result(s_seq)
                        if hg != ag:
                            sim_leader = "home" if hg > ag else "away"
                        else:
                            sim_leader = ("home" if hp > ap else
                                          ("away" if ap > hp else "tie"))
                        g_leader, _ = game_leader_from_row(
                            {"state": {"point_winners": g_seq}})
                        rec["sim_leader"] = sim_leader
                        rec["game_leader"] = g_leader
                        if g_leader == sim_leader:
                            rec["parity"] = "agree-leader"
                            agree += 1
                        else:
                            rec["parity"] = "disagree"
                            disagree += 1
            except Exception as e:  # noqa: BLE001
                rec["error"] = f"{type(e).__name__}: {e}"
                failed += 1

            out.write(json.dumps(rec, separators=(",", ":")) + "\n")
            out.flush()
            print(f"  {away:28} seed={seed} srv={fs_key:11} "
                  f"game={r['game_winners']} sim={rec.get('sim_winners', '?')} "
                  f"{rec.get('parity', rec.get('error', '?'))}")

    total = agree + disagree
    pct = f"{100 * agree / total:.1f}%" if total else "n/a"
    print(f"\nexact (this run): {agree}/{total} = {pct} "
          f"(skipped {skipped}, failed {failed}) -> {OUT}")
    print("  distributional follow-up: python scripts/score_distribution.py")


if __name__ == "__main__":
    main()
