"""One-off: Vector3Split port ids + wiring of a save.

Usage: python scripts/archive/_splits.py [save.txt ...]
"""
from __future__ import annotations

import os
import sys

sys.path.insert(0, os.path.dirname(__file__))
import _dump_save as d  # noqa: E402

SAVES = os.path.expanduser(
    "~/AppData/LocalLow/Unicorn One/AIComp/Saves/Tennis")
DEFAULT = ["diagbot.txt", "underdog.txt", "titanium54.txt", "titanium57.txt",
           "Adam.txt", "Apex.txt"]


def main() -> None:
    files = sys.argv[1:] or [os.path.join(SAVES, f) for f in DEFAULT]
    for p in files:
        if not os.path.exists(p):
            print(f"== {os.path.basename(p)} MISSING")
            continue
        s = d.load(p)
        nodes, owner, ins = d.build(s)
        print(f"== {os.path.basename(p)}")
        for n in s["serializableNodes"]:
            if n["id"] != "Vector3Split":
                continue
            ports = n["serializablePorts"]
            src = ins.get(ports[0]["sID"])
            src_lbl = d.label(src[0]) if src else "(unwired)"
            print(f"   in<-{src_lbl:38s} outs="
                  f"{[pp['id'] for pp in ports if pp['polarity'] == 1]}")


if __name__ == "__main__":
    main()
