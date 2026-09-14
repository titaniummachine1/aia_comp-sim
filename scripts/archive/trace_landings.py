"""Analyze a --trace JSONL: per-point landings, winners, miss distances."""
import io
import json
import math
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
path = sys.argv[1] if len(sys.argv) > 1 else os.path.join(
    HERE, "..", "..", "data", "tennis", "trace_pusher.jsonl")

prev_pts = [0, 0]
prev_games = [0, 0]
prev_bounces = 0
prev_ball = None
landings = []  # (tick, x, z, point_idx)
npoints = 0
winners = []

with io.open(path, encoding="utf-8") as fh:
    for line in fh:
        t = json.loads(line)
        pts = t["points"]
        if pts != prev_pts or t["games"] != prev_games:
            npoints += 1
            if t["games"][0] > prev_games[0]:
                winners.append(0)
            elif t["games"][1] > prev_games[1]:
                winners.append(1)
            elif pts[0] > prev_pts[0]:
                winners.append(0)
            elif pts[1] > prev_pts[1]:
                winners.append(1)
            prev_pts = list(pts)
            prev_games = list(t["games"])
        # landing = ball was up, now at/below floor with bounces increment
        b = t["ball"]
        if prev_ball is not None and prev_ball[1] > 0.35 and b[1] <= 0.35 \
                and t["bounces"] != prev_bounces:
            landings.append((t["tick"], round(b[0], 2), round(b[2], 2), npoints))
        prev_bounces = t["bounces"]
        prev_ball = b

print(f"points={npoints} winners(home=0): {winners}")
print("AO court: |x|<=14, |z|<=6 singles; service boxes x in 7..14")
for tick, x, z, p in landings:
    inout = "IN " if abs(x) <= 14 and abs(z) <= 6 else "OUT"
    print(f"  pt{p:>2} tick{tick:>6} landing=({x:>7},{z:>6}) {inout}")
