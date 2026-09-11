"""Classify how sim points end: serve-hold vs break, and WHERE returns die.

Runs tennis_tournament --trace, then walks ticks:
  serve strike -> bounce#1 (serve bounce in/out?) -> return strike (vel jump)
  -> next bounce landing (in/out via court margins) or arena exit -> point.

Prints per-point: winner, shots, return landing, verdict.
Usage: python scripts/return_forensics.py titanium54 titanium5 171068 --points 4
"""
from __future__ import annotations

import json
import os
import subprocess
import sys
import tempfile

BIN = ["cargo", "run", "-q", "--bin", "tennis_tournament", "--"]
# Court margins mirror court.rs: half_len=14+grown, singles half_wid=6+grown.
# Approx with line/out tolerance: x<=14.4, z<=6.4 counts IN.
IN_X, IN_Z = 14.4, 6.4
# Service box approx: receiver half x in [0,7.1], z half +/-6.1 by ad court.
BOX_X = 7.1


def in_court(x, z):
    return abs(x) <= IN_X and abs(z) <= IN_Z


def main() -> None:
    home, away, seed = sys.argv[1], sys.argv[2], sys.argv[3]
    points = 4
    for i, a in enumerate(sys.argv[4:]):
        if a == "--points":
            points = int(sys.argv[5 + i])
    env = dict(os.environ)
    env.pop("AIA_RALLY_LATCH", None)
    env.pop("AIA_RALLY_DEFAULT", None)
    env.pop("AIA_MIN_CHARGE", None)
    root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    with tempfile.NamedTemporaryFile(suffix=".jsonl", delete=False, mode="w") as tf:
        trace = tf.name
    proc = subprocess.run(
        BIN + ["--home", home, "--away", away, "--seed", seed,
               "--points", str(points), "--max-ticks", "120000",
               "--trace", trace],
        capture_output=True, text=True, timeout=300, env=env, cwd=root,
    )
    line = proc.stdout.strip().splitlines()[-1] if proc.stdout.strip() else ""
    summary = json.loads(line) if line.startswith("{") else {}
    rows = [json.loads(l) for l in open(trace, encoding="utf-8") if l.strip()]
    os.unlink(trace)
    print(f"summary: pts={summary.get('score_pts')} games={summary.get('games')} "
          f"winners={summary.get('point_winners')} faults={summary.get('faults')} "
          f"aces={summary.get('aces')}")
    # Walk: detect strikes (vel jumps while ball near a racket ~ in range),
    # bounces (bounces counter increments), point ends (points change).
    pts = summary.get("point_winners", [])
    # Per-tick: track bounces transitions and vel jumps.
    prev_b, prev_v = 0, None
    events = []  # (tick, kind, detail)
    for r in rows:
        b = r["bounces"]
        v = r["vel"]
        if b != prev_b:
            x, y, z = r["ball"]
            events.append((r["tick"], f"bounce{b}", f"at ({x:.1f},{z:.1f}) "
                           f"{'IN' if in_court(x, z) else 'OUT'} phase={r['phase']}"))
            prev_b = b
        if prev_v is not None:
            dv = sum((a - c) ** 2 for a, c in zip(v, prev_v)) ** 0.5
            if dv > 8.0 and r["phase"] in ("Toss", "Rally"):
                x, y, z = r["ball"]
                events.append((r["tick"], "strike", f"from ({x:.1f},{y:.1f},{z:.1f}) "
                               f"-> vel ({v[0]:.1f},{v[1]:.1f},{v[2]:.1f}) "
                               f"H({r['home'][0]:.1f},{r['home'][1]:.1f})"
                               f"hc={r['hc']:.2f} A({r['away'][0]:.1f},{r['away'][1]:.1f})"
                               f"ac={r['ac']:.2f}"))
        prev_v = v
    # Print events around each point end.
    pendientes = [r for r in rows]
    pt_bounds = []
    cur = (0, 0)
    for r in pendientes:
        p = tuple(r["points"])
        if p != cur:
            pt_bounds.append((r["tick"], cur, p))
            cur = p
    ev_i = 0
    for k, (tick, before, after) in enumerate(pt_bounds):
        w = pts[k] if k < len(pts) else "?"
        print(f"--- point {k} winner={'home' if w==0 else 'away'} {before}->{after} (tick {tick})")
        while ev_i < len(events) and events[ev_i][0] <= tick:
            t, kind, det = events[ev_i]
            print(f"    {t:6} {kind:9} {det}")
            ev_i += 1


if __name__ == "__main__":
    main()
