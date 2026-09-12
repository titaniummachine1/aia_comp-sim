"""Distributional outcome parity: game restart-sweep vs sim seed-sweep.

The exact per-seed metric (first-server 8/78, HANDOFF s11) is brittle while RNG
draw sites and the seed->first-server mapping are unpinned: one world-model
divergence shifts the RNG stream and flips every later point. This scores the
*shape* instead, which is stable and directly falsifiable:

  * home win rate          P(match leader == home)
  * server-hold rate       P(match leader == the match's first server)
  * mean points for/against, mean match length
  * total-variation distance between the game and sim leader distributions

Exact per-seed agreement is printed alongside (never as the headline).

Reads (read-only):  modhost/restart_sweep.jsonl      (game, multi-seed)
                    data/tennis/sim_seed_sweep.jsonl (sim, produced by
                    run_sim_seed_sweep.py)
Usage:
  python scripts/score_distribution.py
  python scripts/score_distribution.py --min-seeds 8
"""

from __future__ import annotations

import argparse
import io
import json
import os
import sys
from collections import Counter, defaultdict

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
sys.path.insert(0, HERE)
from run_sim_tournament_pairs import match_result  # noqa: E402
import run_sim_seed_sweep as sweep  # noqa: E402

SIM = os.path.join(ROOT, "data", "tennis", "sim_seed_sweep.jsonl")
LEADERS = ("home", "away", "tie")


def leader_of(winners: list[int]) -> str:
    hg, ag, hp, ap = match_result(winners)
    if hg != ag:
        return "home" if hg > ag else "away"
    if hp != ap:
        return "home" if hp > ap else "away"
    return "tie"


def summarize(rows: list[dict]) -> dict:
    """rows: {first_server: 'home'|'away'|..., winners: [0/1...]}"""
    n = len(rows)
    if not n:
        return {"n": 0}
    dist = Counter(leader_of(r["winners"]) for r in rows)
    holds = sum(1 for r in rows
                if r["first_server"] in ("home", "away")
                and leader_of(r["winners"]) == r["first_server"])
    n_srv = sum(1 for r in rows if r["first_server"] in ("home", "away"))
    home_pts = away_pts = 0
    total_pts = []
    for r in rows:
        w = list(r["winners"])
        home_pts += w.count(0)
        away_pts += w.count(1)
        total_pts.append(len(w))
    return {
        "n": n,
        "dist": {k: dist.get(k, 0) / n for k in LEADERS},
        "home_win": dist.get("home", 0) / n,
        "away_win": dist.get("away", 0) / n,
        "tie": dist.get("tie", 0) / n,
        "hold": (holds / n_srv) if n_srv else None,
        "mean_home_pts": home_pts / n,
        "mean_away_pts": away_pts / n,
        "mean_len": sum(total_pts) / n,
    }


def tv(a: dict, b: dict) -> float:
    return 0.5 * sum(abs(a.get(k, 0.0) - b.get(k, 0.0)) for k in LEADERS)


def fs_key(srv) -> str:
    return sweep.first_server_env(srv) or "sim-default"


def load_game(path: str) -> list[dict]:
    rows = []
    for r in sweep.load_game_rows(path):
        rows.append({"home": r["home"], "away": r["away"],
                     "seed": str(r["seed"]),
                     "first_server": fs_key(r["first_server"]),
                     "winners": r["game_winners"]})
    return rows


def load_sim(path: str) -> list[dict]:
    rows = []
    try:
        fp = io.open(path, encoding="utf-8", errors="replace")
    except FileNotFoundError:
        return rows
    for line in fp:
        if not line.strip():
            continue
        r = json.loads(line)
        if "sim_winners" not in r:
            continue
        rows.append({"home": r["home"], "away": r["away"],
                     "seed": str(r["seed"]),
                     "first_server": r.get("first_server", "sim-default"),
                     "winners": [int(w) for w in r["sim_winners"]],
                     "parity": r.get("parity")})
    fp.close()
    return rows


def key(r: dict) -> tuple:
    return (r["home"], r["away"], r["seed"], r["first_server"])


def fmt(s: dict) -> str:
    if not s["n"]:
        return "n=0"
    hold = "n/a" if s["hold"] is None else f"{100 * s['hold']:.0f}%"
    return (f"n={s['n']:<3} home={100 * s['home_win']:.0f}% "
            f"hold={hold:<4} pts={s['mean_home_pts']:.1f}-{s['mean_away_pts']:.1f}")


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--game", default=sweep.GAME_SWEEP)
    ap.add_argument("--sim", default=SIM)
    ap.add_argument("--min-seeds", type=int, default=1,
                    help="flag pairings below this many matched seeds")
    a = ap.parse_args()

    game = load_game(a.game)
    sim = load_sim(a.sim)
    g_by = {key(r): r for r in game}
    s_by = {key(r): r for r in sim}

    matched = [k for k in s_by if k in g_by]
    if not matched:
        print(f"no matched seeds.\n  game rows: {len(game)} ({a.game})"
              f"\n  sim  rows: {len(sim)} ({a.sim})")
        print("  run: python scripts/run_sim_seed_sweep.py")
        return

    by_pair = defaultdict(list)
    for k in matched:
        by_pair[(k[0], k[1])].append(k)

    print(f"matched seeds: {len(matched)} across {len(by_pair)} pairing(s)\n")
    print(f"{'pairing':34} {'game':34} {'sim':34} {'TV':>5}  exact")
    g_pool, s_pool = [], []
    thin = 0
    for (home, away) in sorted(by_pair):
        ks = by_pair[(home, away)]
        gr = [g_by[k] for k in ks]
        sr = [s_by[k] for k in ks]
        gsum, ssum = summarize(gr), summarize(sr)
        g_pool += gr
        s_pool += sr
        exact = sum(1 for k in ks if s_by[k].get("parity") == "agree-seq")
        lead = sum(1 for k in ks
                   if str(s_by[k].get("parity", "")).startswith("agree"))
        if len(ks) < a.min_seeds:
            thin += 1
        print(f"{home[:16] + ' vs ' + away[:14]:34} {fmt(gsum):34} "
              f"{fmt(ssum):34} {tv(gsum['dist'], ssum['dist']):5.2f}  "
              f"{lead}/{len(ks)} (seq {exact})")

    gp, sp = summarize(g_pool), summarize(s_pool)
    print(f"\nPOOLED  game: {fmt(gp)}")
    print(f"        sim : {fmt(sp)}")
    print(f"        TV distance = {tv(gp['dist'], sp['dist']):.3f}  "
          f"(0 = identical outcome distribution)")
    if thin:
        print(f"\nWARNING: {thin} pairing(s) below --min-seeds {a.min_seeds}; "
              f"rates are noise at this sample size.")


if __name__ == "__main__":
    main()
