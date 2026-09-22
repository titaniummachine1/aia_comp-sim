"""Raw node JSON comparison: working (label) save vs graphc (numeric) save.

Usage: python scripts/archive/_rawcmp.py <kind> [<kind> ...]
"""
from __future__ import annotations

import json
import os
import sys

SAVES = os.path.expanduser(
    "~/AppData/LocalLow/Unicorn One/AIComp/Saves/Tennis")
PAIR = ["titanium54.txt", "diagbot.txt"]


def dump(fname: str, kind: str) -> None:
    path = os.path.join(SAVES, fname)
    if not os.path.exists(path):
        print(f"  !! missing {fname}")
        return
    d = json.load(open(path, encoding="utf-8"))
    for n in d["serializableNodes"]:
        if n["id"] != kind:
            continue
        ports = [(p["id"], p["polarity"]) for p in n["serializablePorts"]]
        print(f"  {fname}: mod={n.get('modifier')!r} "
              f"ports={ports} "
              f"extra={ {k: v for k, v in n.items() if k not in ('id','sID','modifier','serializablePorts','serializableRectTransform')} }")


def main() -> None:
    kinds = sys.argv[1:] or ["TennisGetVector3", "Vector3Split", "TimePlot"]
    for kind in kinds:
        print(f"=== {kind} ===")
        for f in PAIR:
            dump(f, kind)


if __name__ == "__main__":
    main()
