"""One-off: stream the paritymod probe jsonl and tally record kinds.

Also prints the first few records of any kind that mentions Ball Position /
TennisGetVector3 so we can see how the game resolves modifiers.

Usage: python scripts/archive/_probe_kinds2.py <jsonl> [needle]
"""
from __future__ import annotations

import json
import sys
from collections import Counter

def main() -> None:
    path = sys.argv[1]
    needle = sys.argv[2] if len(sys.argv) > 2 else "Ball Position"
    kinds = Counter()
    hits = []
    with open(path, encoding="utf-8", errors="replace") as fh:
        for line in fh:
            line = line.strip()
            if not line:
                continue
            try:
                rec = json.loads(line)
            except json.JSONDecodeError:
                kinds["<bad json>"] += 1
                continue
            k = rec.get("kind", "<none>")
            kinds[k] += 1
            if needle in line and len(hits) < 8:
                hits.append(rec)
    print(f"=== {path} kinds ===")
    for k, v in kinds.most_common():
        print(f"  {v:8d}  {k}")
    print(f"\n=== first {len(hits)} records containing {needle!r} ===")
    for r in hits:
        s = json.dumps(r)
        print("  " + s[:700])


if __name__ == "__main__":
    main()
