"""One-off: per-channel constancy stats for a game TimePlot export.

Decimal-comma JSON (see modhost/parse_timeplots.py). Prints, per channel:
n samples, min, max, first, last, #distinct(6dp) and a CONST flag.

Usage: python scripts/archive/_diag_timeplot.py <file> [<file> ...]
"""
from __future__ import annotations

import io
import json
import os
import re
import sys

DECIMAL_COMMA = re.compile(r"(?<=[0-9]),(?=[0-9])")


def load(path: str) -> dict:
    text = io.open(path, encoding="utf-8").read()
    return json.loads(DECIMAL_COMMA.sub(".", text))


def main() -> None:
    for path in sys.argv[1:]:
        data = load(path)
        series = data.get("series", [])
        print(f"\n=== {os.path.basename(path)}  simTime={data.get('simTime')} "
              f"channels={len(series)} ===")
        for s in series:
            y = s.get("y", [])
            name = s.get("name", "?")
            if not y:
                x = s.get("x", [])
                y = x
                name += " (x?)"
            n = len(y)
            lo, hi = min(y), max(y)
            distinct = len({round(v, 6) for v in y})
            const = "CONST" if distinct <= 1 else "     "
            print(f"  {const} {name:16s} n={n:5d} min={lo:12.4f} "
                  f"max={hi:12.4f} first={y[0]:10.4f} last={y[-1]:10.4f} "
                  f"distinct={distinct}")


if __name__ == "__main__":
    main()
