"""Dump selected channels of a game TimePlot export over a tick window.

Usage:
  python scripts/dump_timeplot_window.py <timeplot.json> <t0> <t1>
         [--channels "A,B,C"] [--list]

Handles the game's decimal-comma locale. `--list` prints every channel with a
min/max summary so the right channel names can be picked without guessing.
"""
from __future__ import annotations

import glob
import json
import os
import re
import sys


def default_timeplot() -> str:
    tp = os.path.join(os.environ["USERPROFILE"], "AppData", "LocalLow",
                      "Unicorn One", "AIComp", "Saves", "Tennis", "Timeplots")
    return sorted(glob.glob(os.path.join(tp, "*.json")), key=os.path.getmtime)[-1]


def load(path: str):
    raw = open(path, encoding="utf-8", errors="replace").read()
    raw = re.sub(r"(?<=\d),(?=\d)", ".", raw)
    doc = json.loads(raw)
    return {s["name"]: (s.get("y") or []) for s in doc.get("series", []) if "name" in s}


def main() -> None:
    args = sys.argv[1:]
    path = None
    t0 = t1 = None
    want = None
    do_list = False
    i = 0
    while i < len(args):
        a = args[i]
        if a == "--channels":
            want = args[i + 1].split(","); i += 2
        elif a == "--list":
            do_list = True; i += 1
        else:
            if path is None:
                path = a
            elif t0 is None:
                t0 = int(a)
            elif t1 is None:
                t1 = int(a)
            i += 1
    if path is None:
        path = default_timeplot()
    chans = load(path)
    n = max((len(v) for v in chans.values()), default=0)
    if do_list:
        print(f"file={os.path.basename(path)} ticks={n} channels={len(chans)}")
        for nm, ys in chans.items():
            vals = [v for v in ys if isinstance(v, (int, float))]
            if vals:
                print(f"  {nm:38} n={len(ys):5} min={min(vals):+.3f} "
                      f"max={max(vals):+.3f}")
            else:
                print(f"  {nm:38} n={len(ys):5} (non-numeric)")
        return
    if t0 is None:
        t0, t1 = 0, min(n, 40)
    if want is None:
        want = ["Ball X", "Ball Z", "Self X", "Self Z", "Opponent X", "Opponent Z",
                "T.aim_x", "T.aim_z", "Aim X", "Aim Z", "Move X", "Move Z",
                "Shot", "Mode 0 recover 1 receive 2 out 3 serve", "Sprint",
                "Self stamina", "Opponent stamina", "Attack score best",
                "Attack score saved", "Landing forecast X", "Landing forecast Z",
                "Selected contact seconds", "T.struck", "T.shot"]
    print(f"file={os.path.basename(path)} window={t0}..{t1} ticks={n}")
    hdr = f"{'tick':>6} " + " ".join(f"{w[:10]:>10}" for w in want)
    print(hdr)
    for t in range(max(0, t0), min(n, t1 + 1)):
        cells = []
        for w in want:
            ys = chans.get(w)
            v = ys[t] if ys and t < len(ys) else None
            if isinstance(v, (int, float)):
                cells.append(f"{v:>10.3f}")
            elif v is None:
                cells.append(f"{'-':>10}")
            else:
                cells.append(f"{str(v)[:10]:>10}")
        print(f"{t:>6} " + " ".join(cells))


if __name__ == "__main__":
    main()