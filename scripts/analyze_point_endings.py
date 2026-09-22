"""Classify every point ending in a sim run.

Two sources, best first:

  1. `--points <file>` — the `tennis_tournament --trace-points` log. It is
     authoritative: winner + PointReason + striker + ball at resolution.
  2. the raw per-tick trace. It has NO reason field, so endings are inferred
     from bounce geometry. NOTE the inference cannot see a SET-winning point
     (games reset 2 -> 0), so use the point log for anything that matters.

Usage: python scripts/analyze_point_endings.py <trace.jsonl> [--strikes file]
       [--points file] [--home NAME] [--away NAME]

Prints a per-point row (winner, reason, striker, bounce geometry, end-ball,
`ARENA-OUT` flag for a ball that left the |x|>28 / |z|>18 play volume), a
tally by ending class, bounce landings by half, and strike tiers + the late
contact rate — so a shutout or a repeated point-losing pattern is visible at
a glance.
"""
from __future__ import annotations

import io
import json
import os
import sys
from collections import Counter

# Court dims (court.rs): singles half length 14, singles half width 6, plus
# the line half-width tolerance. Display/tally only — the authoritative
# reason comes from the point log when it is supplied.
HL = 14.0
HW = 6.0
ARENA_X = 28.0
ARENA_Z = 18.0


def in_out(x: float, z: float) -> str:
    if abs(x) > ARENA_X or abs(z) > ARENA_Z:
        return "ARENA-OUT"
    return "IN" if abs(x) <= HL + 0.2 and abs(z) <= HW + 0.2 else "OUT"


def _inferred(lb, bounces):
    if lb is None:
        return "no-bounce"
    if bounces is not None and bounces >= 2:
        return "2nd-bounce"
    return "1st-bounce-OUT" if in_out(lb[1], lb[2]) != "IN" else "other"


def main() -> None:
    args = list(sys.argv[1:])
    trace_path = strikes_path = points_path = None
    home, away = "home", "away"
    i = 0
    while i < len(args):
        a = args[i]
        if a in ("--strikes", "--points", "--home", "--away") and i + 1 < len(args):
            val = args[i + 1]
            if a == "--strikes":
                strikes_path = val
            elif a == "--points":
                points_path = val
            elif a == "--home":
                home = val
            else:
                away = val
            i += 2
        else:
            trace_path = a
            i += 1
    if trace_path is None:
        print(__doc__)
        raise SystemExit(2)

    rows = []
    with io.open(trace_path, encoding="utf-8") as fh:
        for line in fh:
            if line.strip():
                rows.append(json.loads(line))
    if not rows:
        print(f"empty trace: {trace_path}")
        return

    names = {0: home, 1: away}

    # Bounce events from the trace (the bounces counter increments on landing).
    bounces = []  # (tick, x, z, bounces, half)
    prev = None
    for r in rows:
        if prev is not None and r["bounces"] > prev["bounces"]:
            b = r["ball"]
            bounces.append((r["tick"], b[0], b[2], r["bounces"], 0 if b[0] < 0 else 1))
        prev = r

    strikes = []
    if strikes_path and os.path.exists(strikes_path):
        with io.open(strikes_path, encoding="utf-8") as fh:
            for line in fh:
                if line.strip():
                    strikes.append(json.loads(line))

    point_log = []
    if points_path and os.path.exists(points_path):
        with io.open(points_path, encoding="utf-8") as fh:
            for line in fh:
                if line.strip():
                    point_log.append(json.loads(line))

    def last_bounce_at(tick):
        found = None
        for e in bounces:
            if e[0] <= tick:
                found = e
            else:
                break
        return found

    points = []
    if point_log:
        for n, e in enumerate(point_log, start=1):
            ball = e.get("ball") or [None, None, None]
            st = e.get("striker", "none")
            points.append({
                "idx": n,
                "tick": e.get("tick"),
                "winner": 0 if e.get("winner") == "home" else 1,
                "reason": e.get("reason", "?"),
                "striker": None if st == "none" else (0 if st == "home" else 1),
                "ball": ball,
                "bounces": e.get("bounces"),
                "rally_hits": e.get("rally_hits"),
                "last_bounce": last_bounce_at(e.get("tick", 0)),
            })
        src = "point log (authoritative)"
    else:
        # Inference fallback. A set-winning point shows up only as a games
        # reset (2 -> 0), which the deltas below detect explicitly.
        prev_pts, prev_games = [0, 0], [0, 0]
        n = 0
        for r in rows:
            winner = None
            if r["games"][0] > prev_games[0] or r["points"][0] > prev_pts[0]:
                winner = 0
            elif r["games"][1] > prev_games[1] or r["points"][1] > prev_pts[1]:
                winner = 1
            elif r["games"] != prev_games or r["points"] != prev_pts:
                winner = 0 if prev_games[0] > prev_games[1] else 1
            if winner is not None:
                n += 1
                points.append({
                    "idx": n,
                    "tick": r["tick"],
                    "winner": winner,
                    "reason": "?",
                    "striker": None,
                    "ball": r["ball"],
                    "bounces": r["bounces"],
                    "rally_hits": None,
                    "last_bounce": last_bounce_at(r["tick"]),
                })
            prev_pts, prev_games = list(r["points"]), list(r["games"])
        src = "trace inference (no reasons; set-winning points are inferred)"

    print(f"trace={os.path.basename(trace_path)} rows={len(rows)} "
          f"points={len(points)} bounces={len(bounces)} strikes={len(strikes)} "
          f"point_log={len(point_log)}")
    print(f"source: {src}")
    print(f"{'#':>3} {'tick':>6} {'win':>12} {'reason':>13} {'striker':>12} "
          f"{'hit':>4} {'last-bounce x,z':>17} {'end-ball x,z':>17} "
          f"{'flag':>9} {'b':>3}")
    for p in points:
        lb = p["last_bounce"]
        lb_s = f"{lb[1]:+.2f},{lb[2]:+.2f}" if lb else "none"
        b = p["ball"]
        has_ball = bool(b) and b[0] is not None
        end_s = f"{b[0]:+.2f},{b[2]:+.2f}" if has_ball else "-"
        flag = in_out(b[0], b[2]) if has_ball else (in_out(lb[1], lb[2]) if lb else "-")
        striker = names[p["striker"]] if p["striker"] is not None else "-"
        rh = p["rally_hits"] if p["rally_hits"] is not None else "-"
        print(f"{p['idx']:>3} {str(p['tick']):>6} {names[p['winner']]:>12} "
              f"{p['reason']:>13} {striker:>12} {str(rh):>4} {lb_s:>17} "
              f"{end_s:>17} {flag:>9} {str(p['bounces']):>3}")

    tally = Counter()
    for p in points:
        cls = p["reason"] if p["reason"] != "?" else _inferred(p["last_bounce"], p["bounces"])
        tally[(names[p["winner"]], cls)] += 1
    print("--- endings (winner, class):", dict(tally))

    bt = Counter()
    for (_, x, z, _, half) in bounces:
        bt[(names[half], in_out(x, z))] += 1
    print("--- bounce landings (half, in/out):", dict(bt))

    if strikes:
        st = Counter()
        for s in strikes:
            st[(names[s["side"]], s["tier"])] += 1
        print("--- strikes (side, tier):", dict(st))
        for side in (0, 1):
            tiers = Counter(s["tier"] for s in strikes if s["side"] == side)
            rally = sum(v for k, v in tiers.items() if k in ("early", "late", "perfect"))
            if rally:
                late = tiers.get("late", 0)
                print(f"    {names[side]}: rally contacts={rally} "
                      f"late={late} ({100.0 * late / rally:.0f}%) "
                      f"perfect={tiers.get('perfect', 0)} "
                      f"early={tiers.get('early', 0)}")


if __name__ == "__main__":
    main()
