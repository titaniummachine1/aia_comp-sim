"""One-off: census of vector-ish node ids + their port ids across saves.

Usage: python scripts/archive/_vecnodes.py
"""
from __future__ import annotations

import os
import sys
from collections import Counter, defaultdict

sys.path.insert(0, os.path.dirname(__file__))
import _dump_save as d  # noqa: E402

SAVES = os.path.expanduser(
    "~/AppData/LocalLow/Unicorn One/AIComp/Saves/Tennis")
FILES = ["titanium54.txt", "titanium57.txt", "diagbot.txt", "underdog.txt",
         "Adam.txt", "Apex.txt"]


def main() -> None:
    for f in FILES:
        p = os.path.join(SAVES, f)
        if not os.path.exists(p):
            print(f"== {f} MISSING")
            continue
        s = d.load(p)
        nodes = s["serializableNodes"]
        vec = [n for n in nodes
               if "Vector" in n["id"] or "Split" in n["id"]]
        counts = Counter(n["id"] for n in vec)
        print(f"== {f}: vector-ish nodes = {dict(counts)}")
        ports = defaultdict(set)
        for n in vec:
            ports[n["id"]].add(tuple(p2["id"] for p2 in n["serializablePorts"]))
        for k, v in ports.items():
            print(f"     {k}: {sorted(v)}")
        print(f"     ALL port ids used: "
              f"{sorted({p2['id'] for n in nodes for p2 in n['serializablePorts']})}")


if __name__ == "__main__":
    main()
