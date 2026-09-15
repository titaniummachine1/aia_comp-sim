"""Score the titanium ladder: read-only table over data/tennis/titanium_ladder.jsonl.

Usage: python scripts/score_titanium_ladder.py [HOME] [--version v015|v014|all]

Prints per-opponent winner, sets, points, world version, ticks, and the
point-ending reason tally (who won the point x why) — the "how did we lose"
view. No sim runs.

Rows are version-tagged: pre-2026-09-14 rows have no `game_version` and are
v014 physics (the old default). v014 and v015 rows are NOT comparable, and the
same home+away+seed can legitimately appear twice (once per version). The
default filter is v015 = the live game's world model.
"""
from __future__ import annotations

import io
import json
import os
import sys
from collections import Counter

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
IN = os.path.join(ROOT, "data", "tennis", "titanium_ladder.jsonl")


def row_version(r):
    return r.get("game_version", "v014")


def main():
    args = [a for a in sys.argv[1:]]
    home = "titanium66"
    version = "v015"
    i = 0
    while i < len(args):
        if args[i] == "--version" and i + 1 < len(args):
            version = args[i + 1]
            i += 2
        else:
            home = args[i]
            i += 1

    if not os.path.exists(IN):
        print(f"no results yet: {IN}")
        return
    rows = []
    with io.open(IN, encoding="utf-8") as fh:
        for line in fh:
            line = line.strip()
            if line:
                rows.append(json.loads(line))
    known = sorted({row_version(r) for r in rows})
    if version != "all":
        rows = [r for r in rows if row_version(r) == version]
    rows.sort(key=lambda r: (r.get("away", ""), row_version(r)))
    print(f"{IN}\nversions present: {known}; showing: {version}; home: {home}")

    w = l = 0
    print(f"\n{'':2}{'opponent':30} {'winner':16} {'sets':9} {'pts':9} "
          f"{'ver':>5} {'ticks':>8}  point endings (reason=home/away)")
    for r in rows:
        away = r.get("away", "?")
        winner = r.get("winner") or ("ERR:" + str(r.get("error", "?"))[:40])
        mark = "W" if winner == home else (
            "L" if not str(winner).startswith("ERR") else "!")
        if mark == "W":
            w += 1
        elif mark == "L":
            l += 1
        why = []
        for entry in r.get("point_reasons") or []:
            try:
                reason, counts = entry[0], entry[1]
            except (TypeError, ValueError, IndexError):
                continue
            why.append(f"{reason}={counts[0]}/{counts[1]}")
        print(f"{mark:2}{away:30} {winner:16} {str(r.get('sets', '?')):9} "
              f"{str(r.get('score_pts', '?')):9} {row_version(r):>5} "
              f"{str(r.get('ticks', '?')):>8}  {' '.join(why)}")
    print(f"\n{home}: {w}W-{l}L across {len(rows)} rows (version={version})")

    # Aggregate how every point ended, by winner — the weakness map.
    agg = Counter()
    for r in rows:
        for entry in r.get("point_reasons") or []:
            try:
                reason, counts = entry[0], entry[1]
            except (TypeError, ValueError, IndexError):
                continue
            agg[(home, reason)] += counts[0]
            agg[(r.get("away", "?"), reason)] += counts[1]
    if agg:
        print("\npoint-ending tally (winner, reason, points):")
        for (who, reason), n in sorted(agg.items(), key=lambda kv: (-kv[1], kv[0])):
            print(f"  {who:24} {reason:14} {n:4}")


if __name__ == "__main__":
    main()