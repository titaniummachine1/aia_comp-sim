"""One-off: event-view of select TimePlot channels.

Usage: python scripts/archive/_diag_events.py <timeplot.json> [every_n]
"""
from __future__ import annotations

import os
import sys

sys.path.insert(0, os.path.dirname(__file__))
import _diag_timeplot as tp  # noqa: E402

KEYS = [
    "D.bpx", "D.bpz", "D.bvx", "D.bvz", "D.spx", "D.spz",
    "D.opx", "D.opz", "D.incoming", "D.range",
    "U.serving", "U.swing", "U.charge",
]


def main() -> None:
    data = tp.load(sys.argv[1])
    every = int(sys.argv[2]) if len(sys.argv) > 2 else 12
    S = {s["name"]: s["y"] for s in data["series"]}
    keys = [k for k in KEYS if k in S]
    n = min(len(S[k]) for k in keys) if keys else 0
    print(" ".join(f"{k:>9s}" for k in keys))
    for i in range(n):
        if i % every:
            continue
        print(" ".join(f"{S[k][i]:9.3f}" for k in keys))


if __name__ == "__main__":
    main()
