"""One-off: dump a graphc save's node census + TimePlot->source wiring.

Usage: python scripts/archive/_dump_save.py <save.txt> [--plots] [--types]
"""
from __future__ import annotations

import json
import os
import re
import sys
from collections import Counter

DECIMAL_COMMA = re.compile(r"(?<=[0-9]),(?=[0-9])")


def load(path: str) -> dict:
    text = open(path, encoding="utf-8").read()
    try:
        return json.loads(text)
    except json.JSONDecodeError:
        return json.loads(DECIMAL_COMMA.sub(".", text))


def build(d: dict):
    nodes = d["serializableNodes"]
    conns = d["serializableConnections"]
    owner = {}
    for n in nodes:
        for p in n["serializablePorts"]:
            owner[p["sID"]] = (n, p)
    # input-port sID -> (source node, source port id)
    ins = {}
    for c in conns:
        s0, s1 = c["port0SID"], c["port1SID"]
        n0, p0 = owner.get(s0, (None, None))
        n1, p1 = owner.get(s1, (None, None))
        if n0 is None or n1 is None:
            continue
        if p0["polarity"] == 1:
            ins[s1] = (n0, p0["id"])
    return nodes, owner, ins


def label(n) -> str:
    if n is None:
        return "?"
    data = n.get("data") or {}
    mod = n.get("modifier") or ""
    idx = data.get("value", data.get("name", ""))
    return f"{n['id']}" + (f"[{mod}]" if mod else "") + \
        (f"({idx})" if idx != "" else "")


def feeders(n, ins, owner, depth, out):
    for p in n["serializablePorts"]:
        if p["polarity"] != 0:
            continue
        src = ins.get(p["sID"])
        if src is None:
            out.append(f"{'  '*depth}{p['id']} <- (unwired)")
            continue
        sn, sp = src
        out.append(f"{'  '*depth}{p['id']} <- {label(sn)}.{sp}"
                   f"  [srcmod={sn.get('modifier')!r}]")
        if depth < 3:
            feeders(sn, ins, owner, depth + 1, out)


def main():
    path = sys.argv[1]
    d = load(path)
    nodes, owner, ins = build(d)
    print(f"=== {os.path.basename(path)} nodes={len(nodes)} "
          f"conns={len(d['serializableConnections'])} ===")
    if "--types" in sys.argv or "--plots" not in sys.argv:
        counts = Counter(n["id"] for n in nodes)
        for k, v in sorted(counts.items(), key=lambda kv: -kv[1]):
            print(f"  {v:4d}  {k}")
    if "--plots" in sys.argv:
        print("--- TimePlot channels ---")
        for n in nodes:
            if n["id"] == "TimePlot":
                out = []
                feeders(n, ins, owner, 1, out)
                print(f"  {label(n)}")
                for line in out:
                    print("    " + line)


if __name__ == "__main__":
    main()
