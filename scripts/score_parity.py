"""Outcome-parity scoring from sim_pairs_results.jsonl.

run_sim_tournament_pairs.py now computes each row's verdict directly from the
game's per-point winner log (point_winners) vs the sim's games/points leader —
no shut-out guessing. This script just aggregates those verdicts.
"""
from __future__ import annotations

import io
import json
import os
from collections import Counter

p = os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))),
                 "data", "tennis", "sim_pairs_results.jsonl")
rows = [json.loads(l) for l in io.open(p, encoding="utf-8") if l.strip()]

counts = Counter = Counter(r.get("parity", "no-verdict") for r in rows)
agree = sum(v for k, v in counts.items() if k.startswith("agree"))
disagree = counts.get("disagree", 0)
unknown = sum(v for k, v in counts.items()
              if not k.startswith("agree") and k != "disagree")
total = agree + disagree
print(f"OUTCOME PARITY: {agree}/{total} = {100*agree/total:.1f}% "
      if total else "no comparisons", end="")
print(f" (unknown {unknown})")
for k, v in sorted(counts.items()):
    print(f"  {k}: {v}")
for r in rows:
    if r.get("parity") != "agree":
        print(" ", r.get("away"), r.get("game_games"), r.get("game_leader"),
              "->", r.get("sim_games"), r.get("sim_leader"),
              r.get("parity", r.get("error")))