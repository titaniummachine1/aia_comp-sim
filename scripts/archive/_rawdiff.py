"""One-off: print full raw node JSON for one node kind in two saves.

Usage: python scripts/archive/_rawdiff.py <kind> <saveA> <saveB>
"""
from __future__ import annotations

import json
import os
import sys


def load(path: str) -> dict:
    return json.loads(open(path, encoding="utf-8").read())


def main() -> None:
    kind = sys.argv[1]
    for path in sys.argv[2:]:
        if not os.path.exists(path):
            print(f"== {os.path.basename(path)} MISSING")
            continue
        d = load(path)
        nodes = [n for n in d["serializableNodes"] if n["id"] == kind]
        print(f"\n=== {os.path.basename(path)} :: {kind} x{len(nodes)} ===")
        for n in nodes[:4]:
            print(json.dumps(n, indent=1))


if __name__ == "__main__":
    main()
