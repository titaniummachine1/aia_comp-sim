"""One-off: tally Tennis sensor modifier encoding (label vs numeric) over all
saves, and cross-check whether each name is a valid option label.

Usage: python scripts/archive/_tally_modifiers.py
"""
from __future__ import annotations

import os
import re
import sys
from collections import Counter, defaultdict

sys.path.insert(0, os.path.dirname(__file__))
import _dump_save as d  # noqa: E402

SAVES = os.path.expanduser(
    "~/AppData/LocalLow/Unicorn One/AIComp/Saves/Tennis")
KINDS = ("TennisGetBool", "TennisGetFloat", "TennisGetVector3",
         "TennisGetTransform", "TennisAutoSwing")


def options():
    sys.path.insert(0, r"c:\gitProjects\AIA_tennis\AIGamePyLibrary")
    from AIGamePyLibrary.data import DROPDOWN_OPTIONS  # type: ignore
    return DROPDOWN_OPTIONS


def main() -> None:
    opts = options()
    files = sorted(f for f in os.listdir(SAVES) if f.endswith(".txt"))
    per_file = {}
    for f in files:
        try:
            s = d.load(os.path.join(SAVES, f))
        except Exception as e:  # noqa: BLE001
            print(f"!! {f}: {e}")
            continue
        agg = defaultdict(Counter)
        for n in s["serializableNodes"]:
            if n["id"] in KINDS:
                mod = str(n.get("modifier"))
                kind = "num" if re.fullmatch(r"-?\d+", mod) else "label"
                agg[n["id"]][kind] += 1
        per_file[f] = agg

    print(f"{'file':38s} " + " ".join(f"{k[10:]:>14s}" for k in KINDS))
    for f in files:
        agg = per_file[f]
        cells = []
        for k in KINDS:
            c = agg.get(k)
            if not c:
                cells.append("-")
            else:
                cells.append("+".join(f"{kk}:{vv}" for kk, vv in sorted(c.items())))
        print(f"{f:38s} " + " ".join(f"{c:>14s}" for c in cells))

    print("\n-- numeric-modifier saves: do their names exist as labels? --")
    for f in files:
        same = per_file[f]
        if not any("num" in c for c in same.values()):
            continue
        s = d.load(os.path.join(SAVES, f))
        kinds = defaultdict(Counter)
        for n in s["serializableNodes"]:
            if n["id"] in KINDS:
                kinds[n["id"]][str(n.get("modifier"))] += 1
        for k, c in kinds.items():
            if k == "TennisAutoSwing":
                continue
            bad = []
            for mod in c:
                try:
                    idx = int(mod) % len(opts.get(k, ()))
                except (TypeError, ValueError):
                    bad.append(mod)
                    continue
                if not opts.get(k):
                    bad.append(mod)
            print(f"  {f:34s} {k:20s} numeric mods={sorted(c)}"
                  f"{'  OUT-OF-RANGE' if bad else ''}")


if __name__ == "__main__":
    main()
