"""Join strike log with trace landings: tier + z-error + landing effect.

Usage: python scripts/archive/strike_join.py <trace.jsonl> <strikes.jsonl> [HOME_NAME]
Side 0 = home. Reports per-side tier hist, contact geometry (z/x err,
height), first-landing in/out by tier, and landing-vs-aim distance.
"""
import io
import json
import math
import os
import sys

trace_path = sys.argv[1]
strikes_path = sys.argv[2]

landings = []  # (tick, x, z, point_idx)
prev_bounces = None
prev_ball = None
prev_pts = [0, 0]
prev_games = [0, 0]
npoints = 0
point_of_tick = {}
winners = []
with io.open(trace_path, encoding="utf-8") as fh:
    for line in fh:
        t = json.loads(line)
        point_of_tick[t["tick"]] = npoints
        if t["points"] != prev_pts or t["games"] != prev_games:
            if t["games"][0] > prev_games[0]:
                winners.append(0)
            elif t["games"][1] > prev_games[1]:
                winners.append(1)
            elif t["points"][0] > prev_pts[0]:
                winners.append(0)
            elif t["points"][1] > prev_pts[1]:
                winners.append(1)
            npoints += 1
            prev_pts = list(t["points"])
            prev_games = list(t["games"])
        b = t["ball"]
        if prev_ball is not None and prev_bounces is not None \
                and prev_ball[1] > 0.35 and b[1] <= 0.35 \
                and t["bounces"] != prev_bounces:
            landings.append((t["tick"], b[0], b[2], npoints))
        prev_bounces = t["bounces"]
        prev_ball = b

strikes = []
with io.open(strikes_path, encoding="utf-8") as fh:
    for line in fh:
        if line.strip():
            strikes.append(json.loads(line))

print(f"strikes={len(strikes)} landings={len(landings)} points={npoints}")
for side in (0, 1):
    ss = [s for s in strikes if s["side"] == side]
    tiers = {}
    for s in ss:
        tiers[s["tier"]] = tiers.get(s["tier"], 0) + 1
    zerr = [s["ball"][2] - s["racket"][2] for s in ss if not s["serving"]]
    xerr = [s["ball"][0] - s["racket"][0] for s in ss if not s["serving"]]
    h = [s["ball"][1] for s in ss if not s["serving"]]
    def stats(v):
        if not v:
            return "n/a"
        m = sum(v) / len(v)
        sd = math.sqrt(sum((x - m) ** 2 for x in v) / len(v))
        return f"mean={m:+.2f} sd={sd:.2f} max|.|={max(abs(x) for x in v):.2f}"
    print(f"--- side {side}: n={len(ss)} tiers={tiers}")
    print(f"    z_err (ball-racket z, rally): {stats(zerr)}")
    print(f"    x_err (ball-racket x, rally): {stats(xerr)}")
    print(f"    contact height (rally):       {stats(h)}")

# join: each strike -> first landing after its tick
li = 0
in_by_tier = {}
out_by_tier = {}
land_dist = []  # |landing - aim| for rally strikes with a landing
for k, s in enumerate(strikes):
    end = strikes[k + 1]["tick"] if k + 1 < len(strikes) else 10 ** 12
    land = None
    for (lt, lx, lz, lp) in landings:
        if s["tick"] < lt < end:
            land = (lx, lz)
            break
    if land is None:
        continue
    inout = abs(land[0]) <= 14 and abs(land[1]) <= 6
    d = in_by_tier if inout else out_by_tier
    key = (s["side"], s["tier"])
    d[key] = d.get(key, 0) + 1
    if not s["serving"]:
        land_dist.append(math.hypot(land[0] - s["aim"][0], land[1] - s["aim"][1]))
print("--- landings by (side, tier): IN", in_by_tier)
print("--- landings by (side, tier): OUT", out_by_tier)
if land_dist:
    m = sum(land_dist) / len(land_dist)
    print(f"--- rally |landing-aim|: mean={m:.2f}m over {len(land_dist)} shots")
print("--- OUT landing coords (side, tier, x, z):")
scoring_vs = {0: [], 1: []}  # per striking side: scoring shots (contact, landing)
for k, s in enumerate(strikes):
    end = strikes[k + 1]["tick"] if k + 1 < len(strikes) else 10 ** 12
    for (lt, lx, lz, lp) in landings:
        if s["tick"] < lt < end:
            if not (abs(lx) <= 14 and abs(lz) <= 6):
                print(f"    side={s['side']} tier={s['tier']} landing=({lx},{lz}) aim={s['aim']}")
            pli = point_of_tick.get(lt, -1)
            win = winners[pli] if 0 <= pli < len(winners) else -1
            if win == s["side"]:
                scoring_vs[s["side"]].append((s["ball"], (lx, lz), s["tier"]))
            break
print("--- SCORING shots (won the point): contact -> landing [tier]")
for side in (0, 1):
    print(f"    side {side}: {len(scoring_vs[side])}")
    for (c, l, t) in scoring_vs[side]:
        print(f"      [{t}] contact=({c[0]:+.2f},{c[1]:+.2f},{c[2]:+.2f}) "
              f"landing=({l[0]:+.2f},{l[1]:+.2f})")
