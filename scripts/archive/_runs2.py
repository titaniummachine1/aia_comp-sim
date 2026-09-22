"""One-off: print a TimePlot channel's value runs (compressed).

Usage: python scripts/archive/_runs2.py <file> <channel> [<channel> ...]
"""
from __future__ import annotations

import os
import re
import sys

sys.path.insert(0, os.path.dirname(__file__))
import _dump_save as d  # noqa: E402

DEC = re.compile(r"(?<=[0-9]),(?=[0-9])")


def load(path):
    import io
    import json
    return json.loads(DEC.sub(".", io.open(path, encoding="utf-8").read()))


def runs(y, max_runs=40):
    out = []
    start = 0
    for i in range(1, len(y)):
        if y[i] != y[i - 1]:
            out.append((start, i - 1, y[i - 1]))
            start = i
    out.append((start, len(y) - 1, y[-1]))
    return out


def main():
    path = sys.argv[1]
    want = sys.argv[2:]
    data = load(path)
    for s in data["series"]:
        if want and s["name"] not in want:
            continue
        y = s["y"]
        r = runs(y)
        print(f"\n=== {s['name']} n={len(y)} runs={len(r)} ===")
        for (a, b, v) in r[:40]:
            print(f"  ticks {a:5d}..{b:5d}  ({b-a+1:5d})  v={v}")


if __name__ == "__main__":
    main()
