"""Walk a tennis save's aim chain and dump the node that feeds it.

Read-only structural probe for the interpreter-certification finding
(HANDOFF §14): the reference `GraphBrain` and the compiled VM disagree on
`TennisCommand.aim` for titanium54 / bat / sim_titanium31, with the reference's
x stuck at 0.0. The chain is

    TennisController(Vector31) <- TennisAutoMove(Vector32)
        <- TennisAutoAim(Vector31) <- <request>

and this prints, for a given save, the chain plus a shallow tree of the request
source node so its node type/modifier can be compared against what each
interpreter implements for that type.

Usage:
  python scripts/analyze_aim_chain.py titanium54
  python scripts/analyze_aim_chain.py bat --depth 4
"""
from __future__ import annotations

import argparse
import io
import json
import os

SAVES = os.path.join(
    os.path.expanduser("~"),
    "AppData", "LocalLow", "Unicorn One", "AIComp", "Saves", "Tennis",
)


class Graph:
    def __init__(self, doc: dict) -> None:
        self.nodes = {n["sID"]: n for n in doc.get("serializableNodes", [])}
        self.port = {}          # port sID -> (node sID, port id, polarity)
        self.by_node_port = {}  # (node sID, port id, polarity) -> port sID
        for n in self.nodes.values():
            for p in n.get("serializablePorts", []):
                key = (n["sID"], p["id"], p.get("polarity"))
                self.port[p["sID"]] = key
                self.by_node_port[key] = p["sID"]
        # input port sID -> source (output) port sID
        self.source = {}
        for c in doc.get("serializableConnections", []):
            self.source[c["port1SID"]] = c["port0SID"]

    def node_of_port(self, port_sid: str) -> dict | None:
        key = self.port.get(port_sid)
        return self.nodes.get(key[0]) if key else None

    def in_port_sid(self, node_sid: str, port_id: str):
        return self.by_node_port.get((node_sid, port_id, 0)) or \
            self.by_node_port.get((node_sid, port_id, None))

    def src_of(self, node_sid: str, port_id: str):
        """Return (source_node, source_port_id, source_port_sid) or None."""
        p = self.in_port_sid(node_sid, port_id)
        if not p:
            return None
        out = self.source.get(p)
        if not out:
            return None
        key = self.port.get(out)
        if not key:
            return None
        return self.nodes.get(key[0]), key[1], out


def label(node: dict | None) -> str:
    if node is None:
        return "<none>"
    mod = node.get("modifier", "")
    return f"{node['id']}#{node['sID'][-6:]} mod={mod!r}"


def walk_aim(g: Graph, controller_sid: str):
    """Mirror of GraphBrain::aim_request_port / Lowerer::aim_request_reg."""
    cur_sid, cur_port = controller_sid, "Vector31"
    chain = []
    for _ in range(8):
        r = g.src_of(cur_sid, cur_port)
        if r is None:
            return chain, None, None, None
        node, out_port_id, out_sid = r
        chain.append(f"{cur_sid[-6:]}.{cur_port} <- {label(node)}.{out_port_id}")
        if node["id"] == "TennisAutoMove":
            cur_sid, cur_port = node["sID"], "Vector32"
        elif node["id"] == "TennisAutoAim":
            cur_sid, cur_port = node["sID"], "Vector31"
        else:
            return chain, node, out_port_id, out_sid
    return chain, None, None, None


def dump_inputs(g: Graph, node: dict, depth: int, prefix: str) -> None:
    if depth <= 0:
        return
    ins = [p for p in node.get("serializablePorts", [])
           if p.get("polarity") == 0]
    for p in ins:
        r = g.src_of(node["sID"], p["id"])
        if r is None:
            print(f"{prefix}  .{p['id']} = <unwired>")
            continue
        child = r[0]
        print(f"{prefix}  .{p['id']} = {label(child)}.{r[1]}")
        dump_inputs(g, child, depth - 1, prefix + "    ")


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("bot", help="save name (no .txt)")
    ap.add_argument("--depth", type=int, default=3)
    a = ap.parse_args()

    path = os.path.join(SAVES, f"{a.bot}.txt")
    if not os.path.exists(path):
        raise SystemExit(f"no save: {path}")
    doc = json.loads(io.open(path, encoding="utf-8", errors="replace").read())
    g = Graph(doc)
    print(f"{a.bot}: {len(g.nodes)} nodes, {len(g.source)} wires")

    controllers = [n for n in g.nodes.values() if n["id"] == "TennisController"]
    if not controllers:
        # Some saves name it differently; fall back to any *Controller node.
        controllers = [n for n in g.nodes.values() if n["id"].endswith("Controller")]
    print(f"controllers: {[label(c) for c in controllers]}")

    for c in controllers:
        print(f"\n== {label(c)} ==")
        chain, src, out_port_id, _ = walk_aim(g, c["sID"])
        for step in chain:
            print(f"  {step}")
        if src is None:
            print("  (no aim chain / unwired)")
            continue
        print(f"  AIM SOURCE: {label(src)}  (out port .{out_port_id})")
        print(f"  source inputs (depth {a.depth}):")
        dump_inputs(g, src, a.depth, "  ")


if __name__ == "__main__":
    main()
