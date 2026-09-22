"""One-off: raw field dump for specific node ids in a save.

Usage: python scripts/archive/_nodes_raw.py <save.txt> Vector3Split ...
"""
from __future__ import annotations

import json
import os
import sys

sys.path.insert(0, os.path.dirname(__file__))
import _dump_save as d  # noqa: E402


def main() -> None:
    path = sys.argv[1]
    wanted = set(sys.argv[2:]) or {"Vector3Split"}
    s = d.load(path)
    print(f"=== {os.path.basename(path)} ===")
    for n in s["serializableNodes"]:
        if n["id"] in wanted:
            ports = [(p["id"], "IN" if p["polarity"] == 0 else "OUT",
                      p["sID"]) for p in n["serializablePorts"]]
            extra = {k: v for k, v in n.items()
                     if k not in ("serializablePorts", "serializableRectTransform",
                                  "defaultColor", "serializableDefaultColor")}
            print(f"{n['sID'][:8]} {json.dumps(extra)}")
            for pid, pol, sid in ports:
                print(f"     {pol:3s} {pid:10s} {sid[:8]}")
            print("---")


if __name__ == "__main__":
    main()
