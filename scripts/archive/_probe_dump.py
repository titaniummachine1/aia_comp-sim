"""One-off: dump probe records of a given kind (first N).

Usage: python scripts/archive/_probe_dump.py <jsonl> <kind> [n]
"""
from __future__ import annotations

import json
import sys


def main() -> None:
    path = sys.argv[1]
    want = sys.argv[2]
    n = int(sys.argv[3]) if len(sys.argv) > 3 else 5
    shown = 0
    with open(path, encoding="utf-8", errors="replace") as fh:
        for line in fh:
            line = line.strip()
            if not line:
                continue
            try:
                rec = json.loads(line)
            except json.JSONDecodeError:
                continue
            if rec.get("kind") != want:
                continue
            print(json.dumps(rec)[:1500])
            print("-" * 60)
            shown += 1
            if shown >= n:
                break


if __name__ == "__main__":
    main()
