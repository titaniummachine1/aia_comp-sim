"""One-off: stream a parity probe jsonl and report message kinds.

Usage: python scripts/archive/_probe_kinds.py <jsonl> [max_lines] [kind_filter]
"""
from __future__ import annotations

import io
import json
import sys
from collections import Counter


def main() -> None:
    path = sys.argv[1]
    limit = int(sys.argv[2]) if len(sys.argv) > 2 else 400000
    flt = sys.argv[3] if len(sys.argv) > 3 else None
    kinds = Counter()
    shown = Counter()
    with io.open(path, "r", encoding="utf-8", errors="replace") as fh:
        for i, line in enumerate(fh):
            if i > limit:
                break
            line = line.strip()
            if not line:
                continue
            try:
                obj = json.loads(line)
            except json.JSONDecodeError:
                kinds["<bad json>"] += 1
                continue
            k = obj.get("kind", "?")
            kinds[k] += 1
            if flt and flt in k and shown[k] < 4:
                shown[k] += 1
                print(f"[{i}] {line[:700]}")
    print("--- kinds (first %d lines) ---" % limit)
    for k, v in kinds.most_common():
        print(f"  {v:8d}  {k}")


if __name__ == "__main__":
    main()
