"""One-off: node census + Tennis sensor usage of game saves.

Usage: python scripts/archive/_census.py
"""
from __future__ import annotations

import os
import sys
from collections import Counter

sys.path.insert(0, os.path.dirname(__file__))
import _dump_save as d  # noqa: E402

SAVES = os.path.expanduser(
    "~/AppData/LocalLow/Unicorn One/AIComp/Saves/Tennis")

FILES = [
    "underdog.txt", "Adam.txt", "Apex.txt", "PerfectController.txt",
    "titanium54.txt", "titanium57.txt", "diagbot.txt",
]


def main() -> None:
    for f in FILES:
        p = os.path.join(SAVES, f)
        if not os.path.exists(p):
            print(f"== {f} MISSING")
            continue
        s = d.load(p)
        ns = s["serializableNodes"]
        c = Counter(n["id"] for n in ns)
        interesting = {k: v for k, v in c.items()
                       if "Tennis" in k or "Controller" in k or "Auto" in k}
        print(f"== {f} nodes={len(ns)} conns={len(s['serializableConnections'])}")
        print(f"   {interesting}")
        for n in ns:
            if n["id"] in ("TennisGetVector3", "TennisGetFloat",
                           "TennisGetBool", "TennisGetTransform"):
                print(f"     {n['id']}[{n.get('modifier')}]")
        for n in ns:
            if n["id"] == "TennisController":
                print("     TennisController ports:",
                      [p2["id"] for p2 in n["serializablePorts"]])
            if n["id"] == "TennisAutoSwing":
                print("     TennisAutoSwing modifier:",
                      repr(n.get("modifier")))


if __name__ == "__main__":
    main()
