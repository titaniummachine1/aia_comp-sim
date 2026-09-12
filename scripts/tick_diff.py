"""Tick-by-tick sim-vs-game channel diff on a MATCHED match.

The game restart sweep leaves one named timeplot per (seed, first server)
(`..._srv<S>_seed<N>.json`). This replays the SAME (seed, first server) in the
sim and diffs the graph's TimePlot channels sample-by-sample — so divergence is
measured from the same serve stance / first server as the game, not across
unrelated matches. That is the fair comparison the channel *distribution* diff
cannot give.

Usage:
  python scripts/tick_diff.py --seed 1 --srv 1
  python scripts/tick_diff.py --seed 4 --srv 0 --points 8 --top 20
"""
from __future__ import annotations

import argparse
import glob
import io
import json
import os
import subprocess
import sys
import tempfile

sys.path.insert(0, r"C:\gitProjects\AIA_tennis\modhost")
from parse_timeplots import parse_timeplot_json  # noqa: E402

ROOT = r"C:\gitProjects\aia_comp-sim"
SAVES = os.path.join(os.path.expanduser("~"), "AppData", "LocalLow",
                     "Unicorn One", "AIComp", "Saves", "Tennis")
TP = os.path.join(SAVES, "Timeplots")
BIN = ["cargo", "run", "-q", "--bin", "tennis_tournament", "--"]
EPS = 1e-3


def game_series(seed: int, srv: int):
    pat = os.path.join(TP, f"*_srv{srv}_seed{seed}.json")
    fs = sorted(glob.glob(pat))
    if not fs:
        raise SystemExit(f"no game timeplot for seed {seed} srv {srv}")
    data = parse_timeplot_json(io.open(fs[-1], encoding="utf-8").read())
    return fs[-1], {s.get("name"): s.get("y", [])
                    for s in data.get("series", [])}


def sim_series(seed: int, srv: int, home: str, away: str, points: int):
    env = dict(os.environ)
    env["AIA_TRACE_CHANNELS"] = "1"
    env["AIA_FIRST_SERVER"] = "home" if srv == 0 else "away"
    with tempfile.NamedTemporaryFile(suffix=".jsonl", delete=False, mode="w") as tf:
        trace = tf.name
    try:
        subprocess.run(
            BIN + ["--home", home, "--away", away, "--seed", str(seed),
                   "--points", str(points), "--trace", trace],
            capture_output=True, text=True, timeout=600, env=env, cwd=ROOT)
        out: dict = {}
        for line in io.open(trace, encoding="utf-8"):
            if not line.strip():
                continue
            hch = json.loads(line).get("hch") or {}
            for k, v in hch.items():
                out.setdefault(k, []).append(v)
        return out
    finally:
        try:
            os.unlink(trace)
        except OSError:
            pass


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--seed", type=int, required=True)
    ap.add_argument("--srv", type=int, required=True)
    ap.add_argument("--home", default="titanium54")
    ap.add_argument("--away", default="aia3")
    ap.add_argument("--points", type=int, default=8)
    ap.add_argument("--top", type=int, default=14)
    ap.add_argument("--anchor", default="none",
                    help="channel aligned on its first value change ('none' = "
                         "raw tick 0; 'auto'; or e.g. v44_BallX)")
    a = ap.parse_args()

    gf, g = game_series(a.seed, a.srv)
    s = sim_series(a.seed, a.srv, a.home, a.away, a.points)
    common = [k for k in g if k in s and g[k] and s[k]]
    if not common:
        raise SystemExit("no shared channels")

    # Phase alignment: the game's graph starts before the ball is placed (its
    # positional channels sit at 0 while the sim already holds the ball at the
    # server's hand), so absolute tick 0 is NOT comparable. Align on the first
    # value change of an anchor channel (default `v44_BallX`: the moment the
    # ball leaves the hand / stance), then compare from there.
    def first_change(v):
        for i in range(1, len(v)):
            if abs(v[i] - v[0]) > EPS:
                return i
        return 0

    anchor = a.anchor
    if anchor == "auto":
        anchor = next((c for c in ("v44_BallX", "v44_SelfX") if c in common),
                      common[0])
    if anchor in ("none", ""):
        ga = sa = 0
        anchor = None
    else:
        ga, sa = first_change(g[anchor]), first_change(s[anchor])

    def mad_at() -> tuple[float, list[tuple[float, int, str]], int, int]:
        # shift = ga - sa: skip that many leading samples from the longer side.
        shift = ga - sa
        rows = []
        for k in common:
            if shift >= 0:
                gv, sv = g[k][shift:], s[k]
            else:
                gv, sv = g[k], s[k][-shift:]
            n = min(len(gv), len(sv))
            if n <= 0:
                continue
            first = -1
            for i in range(n):
                if abs(gv[i] - sv[i]) > EPS:
                    first = i
                    break
            mad = sum(abs(gv[i] - sv[i]) for i in range(n)) / n
            rows.append((mad, first, k))
        total = sum(r[0] for r in rows) / max(1, len(rows))
        return total, rows, shift, n

    total, rows, shift, n = mad_at()
    print(f"game {os.path.basename(gf)}  "
          f"game_n={len(g[common[0]])} sim_n={len(s[common[0]])} | "
          f"{len(common)} shared channels | anchor {anchor or 'none'} "
          f"(game first-change {ga}, sim {sa}) | shift {shift} | "
          f"aligned {n} | mean|diff| {total:.4f}\n")

    rows.sort(reverse=True)
    print(f"{'channel':18} {'first-diff tick':>15} {'mean|diff|':>11}")
    print("-" * 48)
    for mad, first, k in rows[: a.top]:
        print(f"{k:18} {first:>15} {mad:11.4f}")

    # Side-by-side samples for the top channels: distinguishes a *phase* gap
    # (positions/ball start elsewhere during serve setup) from a genuine value
    # drift, and shows whether they ever converge.
    probe = [k for _, _, k in rows[:4]]
    ticks = [0, 1, 2, 10, 25, 50, 100, 200, 400]
    print(f"\nsamples (game vs sim) at aligned ticks {ticks} (shift {shift}):")
    for k in probe:
        gv, sv = g[k], s[k]
        cells = []
        for t in ticks:
            gi, si = t + shift, t
            cells.append(
                f"{t}: {gv[gi]:.2f}/{sv[si]:.2f}"
                if 0 <= gi < len(gv) and si < len(sv) else f"{t}: -/-")
        print(f"  {k:16} " + "  ".join(cells))
    onset = [f for _, f, _ in rows if f >= 0]
    print(f"\nearliest divergence over {len(common)} channels: "
          f"tick {min(onset) if onset else 'none'} "
          f"(median {sorted(onset)[len(onset)//2] if onset else '-'})")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
