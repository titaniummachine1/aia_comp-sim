"""One-off: exact plot-source wiring (source node + SOURCE OUTPUT PORT id).

Usage: python scripts/archive/_plotwiring.py <save.txt> [<save.txt> ...]
"""
from __future__ import annotations

import os
import sys

sys.path.insert(0, os.path.dirname(__file__))
import _dump_save as d  # noqa: E402


def walk(n, ins, owner, out, depth=0):
    for p in n["serializablePorts"]:
        if p["polarity"] != 0:
            continue
        src = ins.get(p["sID"])
        if src is None:
            out.append(f"{'  '*depth}{p['id']} <- UNWIRED")
            continue
        sn, sp = src
        sdata = (sn.get("data") or {})
        out.append(
            f"{'  '*depth}{p['id']} <- {sn['id']}"
            f"[mod={sn.get('modifier')!r}] .{sp['id']}")
        if depth < 2:
            walk(sn, ins, owner, out, depth + 1)


def main():
    for path in sys.argv[1:]:
        s = d.load(path)
        nodes, owner, ins = d.build(s)
        print(f"=== {os.path.basename(path)} ===")
        for n in nodes:
            if n["id"] != "TimePlot":
                continue
            name = "?"
            for p in n["serializablePorts"]:
                if p["id"] != "String1":
                    continue
                src = ins.get(p["sID"])
                if src:
                    name = src[0].get("modifier")
            out = []
            walk(n, ins, owner, out)
            print(f"  -- {name} --")
            for line in out:
                print("    " + line)


if __name__ == "__main__":
    main()
