"""One-off: find node kinds whose ports include X/Y/Z; full-diff a node kind.

Usage:
  python scripts/archive/_xyz.py                 # list kinds with X/Y/Z ports
  python scripts/archive/_xyz.py <kind>          # key-set + full diff, first node
"""
from __future__ import annotations

import json
import os
import sys

sys.path.insert(0, os.path.dirname(__file__))
import _dump_save as d  # noqa: E402

SAVES = os.path.expanduser(
    "~/AppData/LocalLow/Unicorn One/AIComp/Saves/Tennis")
PAIRS = [
    ("titanium54.txt", "diagbot.txt"),
    ("Adam.txt", "titanium57.txt"),
]


def kinds_with_xyz():
    from AIGamePyLibrary.data import ports  # type: ignore
    hits = []
    for kind, ps in ports.items():
        ids = {p.id if hasattr(p, "id") else p["id"] for p in ps}
        if {"X", "Y", "Z"} <= {str(i) for i in ids}:
            hits.append((kind, sorted(str(i) for i in ids)))
    print(f"kinds with X/Y/Z ports: {len(hits)}")
    for k, ids in hits:
        print("  ", k, ids)


def full(kind: str):
    for good, bad in PAIRS:
        for f in (good, bad):
            p = os.path.join(SAVES, f)
            if not os.path.exists(p):
                print(f"== {f} MISSING")
                continue
            s = d.load(p)
            n = next((x for x in s["serializableNodes"] if x["id"] == kind), None)
            if n is None:
                print(f"== {f} :: {kind} absent")
                continue
            ports = n.get("serializablePorts", [])
            print(f"== {f} :: {kind}")
            print("   keys:", sorted(n.keys()))
            print("   modifier:", repr(n.get("modifier")))
            print("   mod ids:", [p2["id"] for p2 in ports])
            print("   full:", json.dumps(n, sort_keys=True)[:900])


if __name__ == "__main__":
    if len(sys.argv) > 1:
        full(sys.argv[1])
    else:
        sys.path.insert(0, r"c:\gitProjects\AIA_tennis\AIGamePyLibrary")
        kinds_with_xyz()
