"""Score the titanium ladder: read-only table over data/tennis/titanium_ladder.jsonl.

Usage: python scripts/score_titanium_ladder.py [HOME]
Prints per-opponent winner, sets, points, ticks + totals. No sim runs.
"""
from __future__ import annotations

import io
import json
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
IN = os.path.join(ROOT, "data", "tennis", "titanium_ladder.jsonl")
HOME = sys.argv[1] if len(sys.argv) > 1 else "titanium58"


def main():
    if not os.path.exists(IN):
        print(f"no results yet: {IN}")
        return
    rows = []
    with io.open(IN, encoding="utf-8") as fh:
        for line in fh:
            line = line.strip()
            if line:
                rows.append(json.loads(line))
    rows.sort(key=lambda r: r.get("away", ""))
    w = l = 0
    print(f"{'opponent':32} {'winner':16} {'sets':9} {'pts':9} {'ticks':>8}")
    for r in rows:
        away = r.get("away", "?")
        winner = r.get("winner") or ("ERR:" + r.get("error", "?")[:40])
        sets = r.get("sets", "?")
        pts = r.get("score_pts", "?")
        ticks = r.get("ticks", "?")
        mark = "W" if winner == HOME else ("L" if not str(winner).startswith("ERR") else "!")
        if mark == "W":
            w += 1
        elif mark == "L":
            l += 1
        print(f"{mark} {away:30} {winner:16} {str(sets):9} {str(pts):9} {ticks:>8}")
    print(f"\n{HOME}: {w}W-{l}L across {len(rows)} rows")


if __name__ == "__main__":
    main()
