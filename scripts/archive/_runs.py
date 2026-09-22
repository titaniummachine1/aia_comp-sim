"""One-off: per-channel value-run summary for game TimePlot exports.

Usage: python scripts/archive/_runs.py <file> [<file> ...]
"""
from __future__ import annotations

import os
import sys
from collections import Counter

sys.path.insert(0, os.path.dirname(__file__))
import _diag_timeplot as dt  # noqa: E402


def main() -> None:
    for path in sys.argv[1:]:
        data = dt.load(path)
        print(f"\n=== {os.path.basename(path)} simTime={data.get('simTime')} "
              f"channels={len(data.get('series', []))} ===")
        for s in data.get("series", []):
            y = s.get("y", [])
            if not y:
                continue
            runs = []
            for v in y:
                r = round(v, 4)
                if runs and runs[-1][0] == r:
                    runs[-1][1] += 1
                else:
                    runs.append([r, 1])
            top = ", ".join(f"{v}x{n}" for v, n in sorted(
                runs, key=lambda x: -x[1])[:7])
            print(f"  {s['name']:14s} n={len(y):5d} runs={len(runs):4d}  {top}")


if __name__ == "__main__":
    main()
