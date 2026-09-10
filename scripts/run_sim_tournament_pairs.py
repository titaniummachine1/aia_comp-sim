"""Replay the game-tournament pairings (same bots, same seeds) in the sim and
score outcome parity against the game results.

Reads:  modhost/game_tournament_results.jsonl (game outcomes)
Writes: data/tennis/sim_pairs_results.jsonl + prints the parity summary.
"""
from __future__ import annotations

import io
import json
import os
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
SAVES = os.path.join(
    os.path.expanduser("~"),
    "AppData", "LocalLow", "Unicorn One", "AIComp", "Saves", "Tennis",
)
GAME_RESULTS = r"C:\gitProjects\AIA_tennis\modhost\game_tournament_results.jsonl"
OUT = os.path.join(ROOT, "data", "tennis", "sim_pairs_results.jsonl")

BIN = ["cargo", "run", "-q", "--bin", "tennis_tournament", "--"]


def match_result(winners, pts_to_win=4):
    """Walk a per-point winner sequence (0=home, 1=away, 4294967295=unknown)
    and return (home_games, away_games, home_pts, away_pts) of the live game."""
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


def game_leader_from_row(r):
    st = r.get("state") or {}
    pw = st.get("point_winners")
    if pw:
        hg, ag, hp, ap = match_result(pw)
        if hg != ag:
            return "home" if hg > ag else "away", [hg, ag]
        if hp != ap:
            return "home" if hp > ap else "away", [hg, ag]
        return "tie", [hg, ag]
    gp = [st.get("home_points", 0), st.get("away_points", 0)]
    if gp[0] != gp[1]:
        return "home" if gp[0] > gp[1] else "away", gp
    return None, gp  # unknown (legacy rows)


def main() -> None:
    home = sys.argv[1] if len(sys.argv) > 1 else "titanium54"
    game_rows = [
        json.loads(l)
        for l in io.open(GAME_RESULTS, encoding="utf-8", errors="replace")
        if l.strip()
    ]
    pairs = [
        r for r in game_rows
        if (r.get("state") or {}).get("done") and r["home"] == home
    ]
    print(f"game pairs to replay: {len(pairs)} (home={home})")

    os.makedirs(os.path.dirname(OUT), exist_ok=True)
    agree = disagree = unknown = 0
    with io.open(OUT, "a", encoding="utf-8") as out:
        for r in pairs:
            bot, seed = r["away"], str(r["seed"])
            g_leader, g_games = game_leader_from_row(r)
            rec = {"home": home, "away": bot, "seed": seed,
                   "game_leader": g_leader, "game_games": g_games,
                   "game_pts": [(r.get("state") or {}).get("home_points", 0),
                                (r.get("state") or {}).get("away_points", 0)]}
            if g_leader is None:
                rec["parity"] = "unknown-game-side"
                unknown += 1
                out.write(json.dumps(rec, separators=(",", ":")) + "\n")
                out.flush()
                print(f"  {bot:28} game side unknown (legacy row) — skipped")
                continue
            try:
                proc = subprocess.run(
                    BIN + ["--home", home, "--away", bot, "--seed", seed,
                           "--points", "8", "--max-ticks", "120000"],
                    capture_output=True, text=True, timeout=300,
                )
                line = proc.stdout.strip().splitlines()[-1] if proc.stdout.strip() else ""
                if proc.returncode == 0 and line.startswith("{"):
                    sim = json.loads(line)
                    rec["sim_pts"] = sim["score_pts"]
                    rec["sim_games"] = sim["games"]
                    # Match outcome: games won, current-game points as tiebreak.
                    sg = sim["games"]
                    sp = sim["score_pts"]
                    if sg[0] != sg[1]:
                        sim_leader = "home" if sg[0] > sg[1] else "away"
                    else:
                        sim_leader = "home" if sp[0] > sp[1] else (
                            "away" if sp[1] > sp[0] else "tie")
                    rec["sim_leader"] = sim_leader
                    if g_leader == sim_leader:
                        agree += 1
                        rec["parity"] = "agree"
                    else:
                        disagree += 1
                        rec["parity"] = "disagree"
                else:
                    err = [
                        l for l in proc.stderr.strip().splitlines()
                        if "RUST_BACKTRACE" not in l and l.strip()
                    ]
                    rec["error"] = (err[-1] if err else "?")[:200]
                    unknown += 1
            except Exception as e:
                rec["error"] = f"{type(e).__name__}: {e}"
                unknown += 1
            out.write(json.dumps(rec, separators=(",", ":")) + "\n")
            out.flush()
            print(f"  {bot:28} game={rec['game_leader']}({rec['game_games'][0]}-{rec['game_games'][1]}) "
                  f"sim={rec.get('sim_pts','?')} "
                  f"{rec.get('parity', rec.get('error','?'))}")

    total = agree + disagree
    print(f"\nOUTCOME PARITY: {agree}/{total} = {100*agree/total:.1f}% "
          f"(unknown {unknown})" if total else "no comparisons")


if __name__ == "__main__":
    main()
