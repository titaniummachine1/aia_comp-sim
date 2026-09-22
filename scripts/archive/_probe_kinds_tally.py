"""One-off: tally the `kind` field of probe JSONL records.

Usage: python scripts/archive/_probe_kinds_tally.py <jsonl>
"""
from __future__ import annotations

import json
import re
import sys
from collections import Counter

KIND = re.compile(r'"kind"\s*:\s*"([^"]+)"')


def main() -> None:
    path = sys.argv[1]
    kinds = Counter()
    total = 0
    with open(path, encoding="utf-8", errors="replace") as fh:
        for line in fh:
            total += 1
            m = KIND.search(line)
            if m:
                kinds[m.group(1)] += 1
            elif total <= 3:
                print("SAMPLE:", line[:300])
    print(f"lines={total}")
    for k, n in kinds.most_common(40):
        print(f"  {n:8d}  {k}")


if __name__ == "__main__":
    main()
