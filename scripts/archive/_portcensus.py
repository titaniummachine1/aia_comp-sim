"""One-off: every port id ever seen, per node kind, across all saves in the
Saves/Tennis folder. Answers "which node has X/Y/Z ports?" decisively.

Usage: python scripts/archive/_portcensus.py
"""
from __future__ import annotations

import os
import sys
from collections import defaultdict

sys.path.insert(0, os.path.dirname(__file__))
import _dump_save as d  # noqa: E402

SAVES = os.path.expanduser(
    "~/AppData/LocalLow/Unicorn One/AIComp/Saves/Tennis")


def main() -> None:
    files = [f for f in os.listdir(SAVES)
             if f.endswith(".txt") or f.endswith(".json")]
    ports: dict[str, set[str]] = defaultdict(set)
    for f in files:
        p = os.path.join(SAVES, f)
        try:
            s = d.load(p)
        except Exception as exc:  # noqa: BLE001
            print(f"  !! {f}: {exc}")
            continue
        for n in s["serializableNodes"]:
            for prt in n["serializablePorts"]:
                ports[n["id"]].add(prt["id"])
        print(f"  read {f}: nodes={len(s['serializableNodes'])}")

    print("\n=== node kind -> port ids (union over all saves) ===")
    for kind in sorted(ports):
        ids = sorted(ports[kind])
        mark = "  <<< X/Y/Z" if {"X", "Y", "Z"} <= set(ids) else ""
        interesting = [i for i in ids if i in ("X", "Y", "Z")]
        print(f"  {kind:34s} {ids}{mark if interesting else ''}")


if __name__ == "__main__":
    main()
