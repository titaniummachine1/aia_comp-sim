"""Aggregate the fair tick-by-tick diff across the matched restart sweep.

`restart_sweep.py` leaves one named game timeplot per (seed, first server). This
runs `tick_diff.py` (serve-strike anchored) for each of those seeds in the sim
and reports the per-seed aligned window, serve-setup delta, and mean|diff|, plus
the aggregate. That turns the single-seed fair comparison into a stable parity
number over the whole first-server-matched set.

Usage:
  python scripts/tick_diff_sweep.py --seeds 1-16 --srv 1 --points 8
"""
from __future__ import annotations

import argparse
import os
import re
import subprocess
import sys

ROOT = r"C:\gitProjects\aia_comp-sim"
HEAD = re.compile(
    r"aligned (\d+) \| mean\|diff\| ([\d.]+)")
ONSET = re.compile(r"game onset (\d+), sim onset (\d+)")
SETUP = re.compile(
    r"game placed@(\d+) released@(\d+) setup=(\d+) \| "
    r"sim placed@(\d+) released@(\d+) setup=(\d+)")


def parse_seeds(spec: str) -> list[int]:
    out: list[int] = []
    for part in spec.split(","):
        if "-" in part:
            a, b = part.split("-", 1)
            out.extend(range(int(a), int(b) + 1))
        else:
            out.append(int(part))
    return out


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--seeds", default="1-15")
    ap.add_argument("--home", default="titanium54")
    ap.add_argument("--away", default="aia3")
    ap.add_argument("--points", type=int, default=8)
    ap.add_argument("--top", type=int, default=6)
    a = ap.parse_args()

    saves = os.path.join(os.path.expanduser("~"), "AppData", "LocalLow",
                         "Unicorn One", "AIComp", "Saves", "Tennis", "Timeplots")
    import json
    rows = []
    chan: dict[str, list[float]] = {}
    for seed in parse_seeds(a.seeds):
        # Newest named export for this seed; the file name carries the server.
        import glob
        fs = sorted(glob.glob(os.path.join(saves, f"*_seed{seed}.json")),
                    key=os.path.getmtime)
        if not fs:
            print(f"seed {seed}: no named timeplot", flush=True)
            continue
        gf = fs[-1]
        m = re.search(r"_srv(\d+)_seed", os.path.basename(gf))
        srv = int(m.group(1)) if m else 0
        cmd = [sys.executable, os.path.join(ROOT, "scripts", "tick_diff.py"),
               "--seed", str(seed), "--srv", str(srv),
               "--home", a.home, "--away", a.away,
               "--points", str(a.points), "--top", str(a.top),
               "--anchor", "strike", "--game-timeplot", gf, "--json"]
        p = subprocess.run(cmd, capture_output=True, text=True, cwd=ROOT,
                           timeout=900)
        out = p.stdout + p.stderr
        h, o, s = HEAD.search(out), ONSET.search(out), SETUP.search(out)
        if not h:
            tail = (out.strip().splitlines() or ["<no output>"])[-1]
            print(f"seed {seed}: no result ({tail})", flush=True)
            continue
        n, mad = int(h.group(1)), float(h.group(2))
        go, so = (int(o.group(1)), int(o.group(2))) if o else (0, 0)
        gsu, ssu = (int(s.group(3)), int(s.group(6))) if s else (0, 0)
        rows.append((seed, n, mad, go, so, gsu, ssu))
        jl = next((ln for ln in out.splitlines() if ln.startswith("JSON ")),
                  None)
        if jl:
            for k, v in json.loads(jl[5:]).items():
                if not k.startswith("__"):
                    chan.setdefault(k, []).append(v)
        print(f"seed {seed:>2} srv{srv}: aligned {n:>3} mean|diff| {mad:7.3f} "
              f"onset g/s {go}/{so} setup g/s {gsu}/{ssu}", flush=True)

    if rows:
        mad = [r[2] for r in rows]
        mad.sort()
        print(f"\naggregate over {len(rows)} matched seeds: "
              f"mean {sum(mad)/len(mad):.3f}  median {mad[len(mad)//2]:.3f}  "
              f"min {mad[0]:.3f}  max {mad[-1]:.3f}")

    if chan:
        worst = sorted(((sum(v) / len(v), k, len(v))
                        for k, v in chan.items()), reverse=True)
        print(f"\nper-channel mean|diff| across {len(rows)} seeds "
              f"(worst {a.top}):")
        for m, k, c in worst[: a.top]:
            print(f"  {k:18} {m:8.3f}  (n={c})")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
