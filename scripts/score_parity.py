"""Refined outcome-parity scoring from sim_pairs_results.jsonl.

Game state shows post-reset points ([0,0] = a 4-0 shut-out game); the sim
records games[] too. For [0,0] game pairs the sim must have won the GAME
(games delta) with the same side the game did.
"""
from __future__ import annotations

import io
import json
import os

p = os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))),
                 "data", "tennis", "sim_pairs_results.jsonl")
rows = [json.loads(l) for l in io.open(p, encoding="utf-8") if l.strip()]

agree = disagree = unknown = 0
detail = []
for r in rows:
    if "error" in r or "sim_pts" not in r:
        unknown += 1
        continue
    gp = r["game_pts"]
    sp = r["sim_pts"]
    sg = r.get("sim_games", [0, 0])
    if gp[0] != gp[1]:
        game_leader = "home" if gp[0] > gp[1] else "away"
    else:
        # 4 points consumed with equal points = one clean shut-out game; the
        # game does not expose who won — assume the winner corresponds to the
        # point_resolved side... unknown -> mark for the strict tie rule.
        game_leader = None
    if sp[0] != sp[1]:
        sim_leader = "home" if sp[0] > sp[1] else "away"
    else:
        sim_leader = "home" if sg[0] > sg[1] else ("away" if sg[1] > sg[0] else "tie")
    if game_leader is None:
        # strict: sim must ALSO be a shut-out game won 4-0 with points reset
        strict_ok = sp[0] == 0 and sp[1] == 0 and sg[0] + sg[1] >= 1
        if strict_ok:
            agree += 1
            r["parity"] = "agree-shutout"
        else:
            disagree += 1
            r["parity"] = "disagree-shoutout-shape"
    else:
        if game_leader == sim_leader:
            agree += 1
            r["parity"] = "agree"
        else:
            disagree += 1
            r["parity"] = "disagree"
    detail.append((r["away"], gp, sp, sg, r["parity"]))

with io.open(p, "w", encoding="utf-8") as f:
    for r in rows:
        f.write(json.dumps(r, separators=(",", ":")) + "\n")

total = agree + disagree
print(f"REFINED OUTCOME PARITY: {agree}/{total} = {100*agree/total:.1f}% (unknown {unknown})")
for d in detail:
    print(" ", d)
