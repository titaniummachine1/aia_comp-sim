"""Rally-fallback sweep: replay game pairs in-sim under AIA_RALLY_DEFAULT /
AIA_MIN_CHARGE variants and score leader agreement vs the game.

Usage:
  python scripts/sweep_rally.py middle            # + optional: --points 8 --limit 10 --offset 0
  python scripts/sweep_rally.py corner --points 8
Env passed through to the tennis_tournament child (no rebuild per variant).

Reads:  C:\\gitProjects\\AIA_tennis\\modhost\\game_tournament_results.jsonl
Compares per-point-winner leader (games, then live points) — same rule as
run_sim_tournament_pairs.py. Prints agree/disagree per pair + summary.
"""
from __future__ import annotations

import io
import json
import os
import subprocess
import sys

GAME_RESULTS = r"C:\gitProjects\AIA_tennis\modhost\game_tournament_results.jsonl"
BIN = ["cargo", "run", "-q", "--bin", "tennis_tournament", "--"]


def match_result(winners, pts_to_win=4):
    hg = ag = hp = ap = 0
    for w in winners:
        if w == 4294967295:
            continue
        if w == 0:
            hp += 1
        elif w == 1:
            ap += 1
        else:
            continue
        if hp >= pts_to_win:
            hg += 1
            hp = ap = 0
        elif ap >= pts_to_win:
            ag += 1
            hp = ap = 0
    return hg, ag, hp, ap


def leader_of_seq(seq):
    hg, ag, hp, ap = match_result([w for w in seq if w != 4294967295])
    if hg != ag:
        return "home" if hg > ag else "away"
    if hp != ap:
        return "home" if hp > ap else "away"
    return "tie"


def game_leader_from_row(r):
    st = r.get("state") or {}
    pw = st.get("point_winners")
    if pw:
        return leader_of_seq(pw)
    gp = [st.get("home_points", 0), st.get("away_points", 0)]
    if gp[0] != gp[1]:
        return "home" if gp[0] > gp[1] else "away"
    return None


def main() -> None:
    raw = sys.argv[1:]
    if raw and not raw[0].startswith("--"):
        variant = raw[0]
        raw = raw[1:]
    else:
        variant = "middle"
    args = raw
    points = 8
    limit = 78
    offset = 0
    out_path = None
    i = 0
    while i < len(args):
        if args[i] == "--points" and i + 1 < len(args):
            points = int(args[i + 1]); i += 2
        elif args[i] == "--limit" and i + 1 < len(args):
            limit = int(args[i + 1]); i += 2
        elif args[i] == "--offset" and i + 1 < len(args):
            offset = int(args[i + 1]); i += 2
        elif args[i] == "--out" and i + 1 < len(args):
            out_path = args[i + 1]; i += 2
        else:
            i += 1
    game_rows = [json.loads(l) for l in io.open(GAME_RESULTS, encoding="utf-8", errors="replace") if l.strip()]
    pairs = [r for r in game_rows if (r.get("state") or {}).get("done") and r.get("home") == "titanium54"]
    pairs = pairs[offset:offset + limit]
    print(f"variant={variant} pairs={len(pairs)} points={points} (offset {offset})")
    env = dict(os.environ)
    if variant == "middle":
        env.pop("AIA_RALLY_DEFAULT", None)
    else:
        env["AIA_RALLY_DEFAULT"] = variant
    agree = disagree = unknown = 0
    verdicts = []
    for r in pairs:
        bot, seed = r["away"], str(r["seed"])
        g_seq = [(r.get("state") or {}).get("point_winners", [])]
        g_leader = game_leader_from_row(r)
        if g_leader is None:
            unknown += 1
            print(f"  {bot:28} seed={seed} game side unknown — skipped")
            continue
        try:
            proc = subprocess.run(
                BIN + ["--home", "titanium54", "--away", bot, "--seed", seed,
                       "--points", str(points), "--max-ticks", "120000"],
                capture_output=True, text=True, timeout=300, env=env,
                cwd=os.path.dirname(os.path.dirname(os.path.abspath(__file__))),
            )
            line = proc.stdout.strip().splitlines()[-1] if proc.stdout.strip() else ""
            if proc.returncode == 0 and line.startswith("{"):
                sim = json.loads(line)
                s_leader = leader_of_seq(sim.get("point_winners", []))
                mark = "AGREE" if s_leader == g_leader else "disagree"
                if s_leader == g_leader:
                    agree += 1
                else:
                    disagree += 1
                verdicts.append({"away": bot, "seed": seed, "game": g_leader,
                                 "sim": s_leader, "verdict": mark,
                                 "variant": variant,
                                 "latch": env.get("AIA_RALLY_LATCH", "on"),
                                 "min_charge": env.get("AIA_MIN_CHARGE", "0")})
                print(f"  {bot:28} seed={seed} game={g_leader} sim={s_leader} {mark}")
            else:
                unknown += 1
                print(f"  {bot:28} seed={seed} SIM ERROR rc={proc.returncode}")
        except Exception as e:
            unknown += 1
            print(f"  {bot:28} seed={seed} EXC {type(e).__name__}: {e}")
    total = agree + disagree
    pct = f"{100*agree/total:.1f}%" if total else "n/a"
    print(f"SUMMARY variant={variant}: agree={agree} disagree={disagree} unknown={unknown} parity={agree}/{total}={pct}")
    if out_path:
        with io.open(out_path, "a", encoding="utf-8") as f:
            for v in verdicts:
                f.write(json.dumps(v, separators=(",", ":")) + "\n")
        print(f"appended {len(verdicts)} verdicts to {out_path}")


if __name__ == "__main__":
    main()
