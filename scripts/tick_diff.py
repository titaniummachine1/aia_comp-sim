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
# Ball RELEASE ("serve strike") threshold: a per-tick positional step big enough
# to be a launched ball (>0.3 m/tick = 15 m/s), well above the placement settle
# and the in-hand hover drift (<0.15). Without this, the ball's teleport onto the
# server's hand false-fires as a "release" one tick later.
REL_EPS = 0.3
# The game emits 999.0 for "time to intercept = unreachable". Treat that as a
# categorical sentinel: equal sentinels match; a sentinel vs a real value is one
# unit of "wrong state" (not a 999-wide numeric gap, which would swamp the mean).
SENTINEL = 900.0


def _absdiff(a: float, b: float) -> float:
    if a >= SENTINEL and b >= SENTINEL:
        return 0.0
    if a >= SENTINEL or b >= SENTINEL:
        return 1.0
    return abs(a - b)


def _diverge(a: float, b: float) -> bool:
    if a >= SENTINEL or b >= SENTINEL:
        return (a >= SENTINEL) != (b >= SENTINEL)
    return abs(a - b) > EPS



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
    ap.add_argument("--game-timeplot", default=None,
                    help="explicit game TimePlot export path (an un-renamed "
                         "quit export); default = the sweep-named file")
    ap.add_argument("--anchor", default="strike",
                    help="fairness anchor: 'strike' (ball release = same serve "
                         "stance, default), 'ball' (ball placed at server), "
                         "'none' (raw tick 0), 'auto', or a channel name")
    ap.add_argument("--json", action="store_true",
                    help="also emit a JSON map of per-channel mean|diff|")
    a = ap.parse_args()

    def _release_after_place(v):
        p = next((i for i in range(len(v)) if abs(v[i]) > EPS), None)
        if p is None:
            return None
        return next((i for i in range(p + 1, len(v))
                     if abs(v[i] - v[i - 1]) > REL_EPS), None)

    if a.game_timeplot:
        gf = a.game_timeplot
        data = parse_timeplot_json(io.open(gf, encoding="utf-8").read())
        g = {s.get("name"): s.get("y", []) for s in data.get("series", [])}
        # AUTO-DETECT the game's first-serve side from the data itself: at the
        # strike the ball sits at the SERVER's stance, so sign(BallX at the
        # release) says which side served. Never trust the file label -- a
        # mismatched first server silently compares MIRRORED matches (the
        # game's serve from -X vs the sim's from +X), which explodes the diff.
        if "v44_BallX" in g:
            rel = _release_after_place(g["v44_BallX"])
            at = (rel if rel is not None else 0)
            vals = [x for x in g["v44_BallX"][max(0, at - 2):at + 3] if x]
            if vals:
                a.srv = 0 if (sum(vals) / len(vals)) < 0 else 1
                print(f"serve side auto-detected from the game export: "
                      f"BallX@strike {sum(vals)/len(vals):+.2f} -> "
                      f"{'home' if a.srv == 0 else 'away'} serves")
    else:
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

    # Serve-stance anchor. The game holds the ball at the ORIGIN (0,0,0)
    # during the pre-serve setup, then *teleports* it to the server's hand and
    # serves almost immediately (placed@t, released@t+1). The sim holds it at
    # the hand from tick 0 and spends a walk-in/toass window before releasing.
    # The fair "same serve stance" instant is therefore the ball RELEASE (the
    # serve strike), where both sides are about to launch the same point.
    def first_nonzero(v):
        for i in range(len(v)):
            if abs(v[i]) > EPS:
                return i
        return 0

    anchor = a.anchor
    if anchor == "auto":
        anchor = next((c for c in ("v44_BallX", "v44_SelfX") if c in common),
                      common[0])
    if anchor in ("none", ""):
        ga = sa = 0
        anchor = None
    elif anchor in ("ball", "strike", "release"):
        anchor = next((c for c in ("v44_BallX", "v44_BallY", "v44_BallZ")
                       if c in common), common[0])
        if anchor in ("ball",):
            ga, sa = first_nonzero(g[anchor]), first_nonzero(s[anchor])
        else:
            ga = _release_after_place(g[anchor]) or first_nonzero(g[anchor])
            sa = _release_after_place(s[anchor]) or first_nonzero(s[anchor])
    else:
        ga, sa = first_change(g[anchor]), first_change(s[anchor])

    def mad_at() -> tuple[float, list[tuple[float, int, str]], int, int]:
        # Slice BOTH sides at their own serve onset (ga / sa) so tick i compares
        # the same rally instant -- the game's pre-serve lead-in (ball at origin)
        # and the sim's walk-in/toass window are both discarded.
        rows = []
        n = 0
        for k in common:
            gv, sv = g[k][ga:], s[k][sa:]
            n = min(len(gv), len(sv))
            if n <= 0:
                continue
            first = -1
            for i in range(n):
                if _diverge(gv[i], sv[i]):
                    first = i
                    break
            mad = sum(_absdiff(gv[i], sv[i]) for i in range(n)) / n
            rows.append((mad, first, k))
        total = sum(r[0] for r in rows) / max(1, len(rows))
        return total, rows, n

    total, rows, n = mad_at()
    print(f"game {os.path.basename(gf)}  "
          f"game_n={len(g[common[0]])} sim_n={len(s[common[0]])} | "
          f"{len(common)} shared channels | anchor {anchor or 'none'} "
          f"(game onset {ga}, sim onset {sa}) | "
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
    print(f"\nsamples (game vs sim) at aligned ticks {ticks} "
          f"(game onset {ga}, sim onset {sa}):")
    for k in probe:
        gv, sv = g[k], s[k]
        cells = []
        for t in ticks:
            gi, si = ga + t, sa + t
            cells.append(
                f"{t}: {gv[gi]:.2f}/{sv[si]:.2f}"
                if 0 <= gi < len(gv) and si < len(sv) else f"{t}: -/-")
        print(f"  {k:16} " + "  ".join(cells))
    onset = [f for _, f, _ in rows if f >= 0]
    print(f"\nearliest divergence over {len(common)} channels: "
          f"tick {min(onset) if onset else 'none'} "
          f"(median {sorted(onset)[len(onset)//2] if onset else '-'})")

    # Serve-phase timing: when is the ball PLACED at the server (first non-zero)
    # and when is it RELEASED (first positional move after placement)? This is
    # the fair "same serve stance" clock; a gap here is a serve-setup timing
    # difference, not a rally difference.
    bc = next((c for c in ("v44_BallX", "v44_BallY", "v44_BallZ")
               if c in common), None)
    if bc:
        def place_release(v):
            p = next((i for i in range(len(v)) if abs(v[i]) > EPS), None)
            if p is None:
                return None, None
            r = next((i for i in range(p + 1, len(v))
                      if abs(v[i] - v[i - 1]) > REL_EPS), None)
            return p, r
        gp, gr = place_release(g[bc])
        sp, sr = place_release(s[bc])
        setup = lambda a, b: (b - a) if (a is not None and b is not None) else None
        print(f"serve phase ({bc}): game placed@{gp} released@{gr} "
              f"setup={setup(gp, gr)} | sim placed@{sp} released@{sr} "
              f"setup={setup(sp, sr)}")

    if a.json:
        blob = {k: round(mad, 4) for mad, _, k in rows}
        blob["__aligned__"] = n
        blob["__onset__"] = [ga, sa]
        print("JSON " + json.dumps(blob))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
