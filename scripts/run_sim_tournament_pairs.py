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
            gp = [r["state"]["home_points"], r["state"]["away_points"]]
            rec = {"home": home, "away": bot, "seed": seed, "game_pts": gp}
            try:
                proc = subprocess.run(
                    BIN + ["--home", home, "--away", bot, "--seed", seed,
                           "--points", "4", "--max-ticks", "60000"],
                    capture_output=True, text=True, timeout=180,
                )
                line = proc.stdout.strip().splitlines()[-1] if proc.stdout.strip() else ""
                if proc.returncode == 0 and line.startswith("{"):
                    sim = json.loads(line)
                    rec["sim_pts"] = sim["score_pts"]
                    rec["sim_games"] = sim["games"]
                    # game points are cumulative per points_played (4 points);
                    # sim score_pts is per current game — compare leader.
                    game_leader = "home" if gp[0] > gp[1] else ("away" if gp[1] > gp[0] else "tie")
                    sim_leader = "home" if sim["score_pts"][0] > sim["score_pts"][1] else (
                        "away" if sim["score_pts"][1] > sim["score_pts"][0] else "tie")
                    rec["game_leader"] = game_leader
                    rec["sim_leader"] = sim_leader
                    if game_leader == sim_leader:
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
            print(f"  {bot:28} game={gp} sim={rec.get('sim_pts','?')} "
                  f"{rec.get('parity', rec.get('error','?'))}")

    total = agree + disagree
    print(f"\nOUTCOME PARITY: {agree}/{total} = {100*agree/total:.1f}% "
          f"(unknown {unknown})" if total else "no comparisons")


if __name__ == "__main__":
    main()
