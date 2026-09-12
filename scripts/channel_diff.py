"""Per-tick sensor/channel parity: sim graph channels vs a game native timeplot.

The game exports the home graph's TimePlot channels (`v44_*` etc.) as a JSON
timeplot; the sim can emit the SAME channel names from the SAME graph
(`tennis_tournament --trace` + `AIA_TRACE_CHANNELS=1` -> `hch`). This compares,
per channel, the sim distribution against the game's — no same-seed replay
needed, so it isolates *model* drift (how often a sensor fires) from outcome
drift.

Usage:
  python scripts/channel_diff.py                      # default capture + sim
  python scripts/channel_diff.py <capture.json>
  python scripts/channel_diff.py --env AIA_SWING_MODEL=hold
  python scripts/channel_diff.py --json
"""
from __future__ import annotations

import argparse
import io
import json
import os
import subprocess
import sys
import tempfile

sys.path.insert(0, r"C:\gitProjects\AIA_tennis\modhost")
from parse_timeplots import parse_timeplot_json  # noqa: E402

ROOT = r"C:\gitProjects\aia_comp-sim"
SIM_BIN = ["cargo", "run", "-q", "--bin", "tennis_tournament", "--"]
DEFAULT_CAP = (r"C:\gitProjects\AIA_tennis\modhost\captures\timeplots-20260910"
               r"\timeplot_2026-09-10_15-27-03.json")
CH = ["v44_InSwingRange", "v44_MustWait", "v44_Bounced", "v44_Incoming",
      "v44_OnSelfSide", "v44_Chase", "v44_Mode", "v44_SwingHeld",
      "v44_RacketDist", "v44_DNow", "v44_TIntercept", "v44_Desperate",
      "v44_LateExpect", "v44_ChargePct", "v44_AimX", "v44_AimZ",
      "v44_SelfX", "v44_BallX"]


def game_channels(cap: str) -> dict:
    data = parse_timeplot_json(io.open(cap, encoding="utf-8").read())
    s = {x.get("name"): x.get("y", []) for x in data.get("series", [])}
    return {c: s.get(c, []) for c in CH}


def sim_channels(home: str, away: str, seed: int, points: int,
                 env_over: dict) -> dict:
    env = dict(os.environ)
    env.pop("AIA_AIM_MODEL", None)
    env["AIA_TRACE_CHANNELS"] = "1"
    env.update(env_over)
    with tempfile.NamedTemporaryFile(suffix=".jsonl", delete=False, mode="w") as tf:
        trace = tf.name
    try:
        subprocess.run(
            SIM_BIN + ["--home", home, "--away", away, "--seed", str(seed),
                       "--points", str(points), "--trace", trace],
            capture_output=True, text=True, timeout=600, env=env, cwd=ROOT)
        out = {c: [] for c in CH}
        for line in io.open(trace, encoding="utf-8"):
            if not line.strip():
                continue
            row = json.loads(line)
            hch = row.get("hch") or {}
            for c in CH:
                v = hch.get(c)
                if v is not None:
                    out[c].append(v)
        return out
    finally:
        try:
            os.unlink(trace)
        except OSError:
            pass


def stats(vals) -> dict:
    if not vals:
        return {"n": 0, "mean": 0.0, "frac": 0.0}
    n = len(vals)
    return {"n": n, "mean": sum(vals) / n,
            "frac": sum(1 for v in vals if v > 0.5) / n}


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("capture", nargs="?", default=DEFAULT_CAP)
    ap.add_argument("--home", default="titanium54")
    ap.add_argument("--away", default="aia3")
    ap.add_argument("--seed", type=int, default=7)
    ap.add_argument("--points", type=int, default=4)
    ap.add_argument("--env", action="append", default=[],
                    help="K=V sim env override (repeatable)")
    ap.add_argument("--json", action="store_true")
    a = ap.parse_args()

    env_over = {}
    for kv in a.env:
        k, _, v = kv.partition("=")
        env_over[k] = v

    g = {c: stats(v) for c, v in game_channels(a.capture).items()}
    s = {c: stats(v) for c, v in sim_channels(a.home, a.away, a.seed, a.points,
                                             env_over).items()}

    if a.json:
        print(json.dumps({"capture": a.capture, "env": env_over,
                          "game": g, "sim": s}, indent=1))
        return 0

    print(f"capture: {os.path.basename(a.capture)}   "
          f"sim: {a.home} vs {a.away} seed={a.seed} points={a.points} "
          f"env={env_over or '{}'}\n")
    print(f"{'channel':18} {'GAME mean':>10} {'SIM mean':>10} {'d-mean':>8} "
          f"{'GAME f>0.5':>11} {'SIM f>0.5':>10} {'d-frac':>8}")
    print("-" * 80)
    worst = []
    for c in CH:
        gm, sm = g[c]["mean"], s[c]["mean"]
        gf, sf = g[c]["frac"], s[c]["frac"]
        print(f"{c:18} {gm:10.3f} {sm:10.3f} {sm - gm:8.3f} "
              f"{gf:11.3f} {sf:10.3f} {sf - gf:8.3f}")
        worst.append((abs(sf - gf), c, gm, sm, gf, sf))
    worst.sort(reverse=True)
    print("\nlargest |d-frac| (sim vs game):")
    for d, c, gm, sm, gf, sf in worst[:6]:
        print(f"  {c:18} game={gf:.3f} sim={sf:.3f}  (d={d:.3f})")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
