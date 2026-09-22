"""One-off: dump port ids of selected node kinds across saves.

Usage: python scripts/archive/_ports.py <kind> <save> [<save> ...]
"""
from __future__ import annotations

import os
import sys

sys.path.insert(0, os.path.dirname(__file__))
import _dump_save as d  # noqa: E402

SAVES = os.path.expanduser(
    "~/AppData/LocalLow/Unicorn One/AIComp/Saves/Tennis")


def main() -> None:
    kind = sys.argv[1]
    for name in sys.argv[2:]:
        path = name if os.path.exists(name) else os.path.join(SAVES, name)
        if not os.path.exists(path):
            print(f"== {name} MISSING")
            continue
        s = d.load(path)
        seen = {}
        for n in s["serializableNodes"]:
            if n["id"] != kind:
                continue
            key = tuple(p["id"] for p in n["serializablePorts"])
            seen[key] = seen.get(key, 0) + 1
        print(f"== {os.path.basename(path)}  kind={kind}")
        for key, count in seen.items():
            print(f"   x{count} {key}")


if __name__ == "__main__":
    main()
