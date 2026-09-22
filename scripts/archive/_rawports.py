"""One-off: raw Vector3Split port ids + TennisGet* modifiers per save.

Usage: python scripts/archive/_rawports.py
"""
from __future__ import annotations

import io
import json
import os
from collections import Counter

SAVES = os.path.expanduser(
    "~/AppData/LocalLow/Unicorn One/AIComp/Saves/Tennis")
FILES = ["titanium54.txt", "Adam.txt", "Apex.txt", "underdog.txt",
         "diagbot.txt", "titanium57.txt"]


def main() -> None:
    for f in FILES:
        p = os.path.join(SAVES, f)
        if not os.path.exists(p):
            print(f"== {f} MISSING")
            continue
        d = json.load(io.open(p, encoding="utf-8"))
        print(f"== {f}")
        seen_split = False
        mods: dict[str, Counter] = {}
        for n in d["serializableNodes"]:
            if n["id"] == "Vector3Split" and not seen_split:
                print("   Vector3Split ports:",
                      [(x["id"], x["polarity"]) for x in n["serializablePorts"]])
                seen_split = True
            if n["id"].startswith("TennisGet") or n["id"] == "TennisAutoSwing":
                mods.setdefault(n["id"], Counter())[n.get("modifier", "")] += 1
        for k in sorted(mods):
            print(f"   {k}: {dict(mods[k])}")


if __name__ == "__main__":
    main()
