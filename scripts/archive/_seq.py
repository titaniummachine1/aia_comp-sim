"""One-off: print selected TimePlot channels as a time-ordered table.

Usage: python scripts/archive/_seq.py <timeplot.json> [name-prefix ...]
"""
from __future__ import annotations

import os
import sys

sys.path.insert(0, os.path.dirname(__file__))
import _diag_timeplot as d  # noqa: E402

TP = os.path.expanduser(
    "~/AppData/LocalLow/Unicorn One/AIComp/Saves/Tennis/Timeplots")


def main() -> None:
    path = sys.argv[1]
    if not os.path.isabs(path):
        path = os.path.join(TP, path)
    wants = sys.argv[2:]
    data = d.load(path)
    series = data["series"]
    if wants:
        series = [s for s in series if any(w in s["name"] for w in wants)]
    names = [s["name"] for s in series]
    n = max(len(s["y"]) for s in series)
    print("tick  " + " ".join(f"{nm:>10s}" for nm in names))
    step = max(1, n // 40)
    for i in range(0, n, step):
        row = []
        for s in series:
            y = s["y"]
            row.append(f"{y[i]:10.4f}" if i < len(y) else " " * 10)
        print(f"{i:5d} " + " ".join(row))


if __name__ == "__main__":
    main()
