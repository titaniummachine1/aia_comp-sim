"""One-off: full JSON of one sensor/split node per save + key diff.

Usage: python scripts/archive/_nodejson.py [kind]
"""
from __future__ import annotations

import json
import os
import sys

sys.path.insert(0, os.path.dirname(__file__))
import _dump_save as d  # noqa: E402

SAVES = os.path.expanduser(
    "~/AppData/LocalLow/Unicorn One/AIComp/Saves/Tennis")
FILES = ["titanium54.txt", "diagbot.txt", "underdog.txt", "titanium57.txt"]


def main() -> None:
    kind = sys.argv[1] if len(sys.argv) > 1 else "TennisGetVector3"
    for f in FILES:
        p = os.path.join(SAVES, f)
        if not os.path.exists(p):
            print(f"== {f} MISSING")
            continue
        s = d.load(p)
        n = next((x for x in s["serializableNodes"] if x["id"] == kind), None)
        print(f"== {f} :: {kind}")
        print(json.dumps(n, indent=1)[:1600])


if __name__ == "__main__":
    main()
