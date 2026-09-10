"""Build the interpreter-semantics probe graph (sim_probe.txt).

One graph that computes every ambiguous VM semantic case and TimePlots each
result on a named channel. Run it in the SIM (tennis_probe bin) and in the
GAME (modctl home=sim_probe), then diff channel-by-channel with
compare_probe_timeplots.py.

Channels:
  T_const7        constant 7
  T_null_add      AddFloats(3, <unwired>)      <- unwired-input policy
  T_div_pos0      1/0                          <- +inf expected
  T_div_neg0      -1/0                         <- -inf
  T_div_00        0/0                          <- nan
  T_bool_coerce   Bool(true) -> Float port     <- coercion
  T_float_cond    ConditionalSetBool(cond=Float(0.5) coerced, t=1, f=0)
  T_cmp_eq        CompareFloats(5, 5, op=0/eq)
  T_cmp_lt        CompareFloats(3, 5, op=1/lt)
  T_latch_x       SetVariable x = GetVariable(x) + 1 per tick  <- tick/latch
  T_court_w       TennisGetFloat idx 21 (Court Width)   -> 12
  T_net_h         TennisGetFloat idx 22 (Net Height)    -> 0.95
  T_fdt           TennisGetFloat idx 47 (Fixed Delta Time)
  T_op_empty      Operation with empty modifier on 4    <- game policy?
"""
from __future__ import annotations

import io
import json
import os

SAVES = os.path.join(
    os.path.expanduser("~"),
    "AppData", "LocalLow", "Unicorn One", "AIComp", "Saves", "Tennis",
)
OUT = os.path.join(SAVES, "sim_probe.txt")

UID = [0]


def sid(prefix: str) -> str:
    UID[0] += 1
    return f"{prefix}_{UID[0]:04x}"


class G:
    def __init__(self) -> None:
        self.nodes = []
        self.conns = []

    def node(self, nid: str, modifier=None, ports=(), owner=""):
        s = sid(nid)
        plist = []
        for pid, pol in ports:
            plist.append({
                "id": pid, "sID": sid(f"{nid}_{pid}"),
                "nodeSID": s, "polarity": pol,
            })
        self.nodes.append({
            "id": nid, "sID": s,
            "modifier": modifier if modifier is not None else "",
            "ownerFunctionSID": owner,
            "serializablePorts": plist,
        })
        return s

    def port(self, node_sid: str, pid: str, polarity: int | None = None) -> str:
        for n in self.nodes:
            if n["sID"] == node_sid:
                for p in n["serializablePorts"]:
                    if p["id"] == pid and (polarity is None or p["polarity"] == polarity):
                        return p["sID"]
        raise KeyError(f"{node_sid}.{pid}@{polarity}")

    def link(self, out_sid: str, in_sid: str) -> None:
        self.conns.append({"port0SID": out_sid, "port1SID": in_sid})

    # helpers ------------------------------------------------------------
    def const_float(self, v: float):
        n = self.node("Float", modifier=repr(float(v)), ports=[("Float1", 1)])
        return n

    def const_bool(self, v: bool):
        # game convention: modifier "1" == false, anything else true
        n = self.node("Bool", modifier="0" if v else "1", ports=[("Bool1", 1)])
        return n

    def const_string(self, v: str):
        return self.node("String", modifier=v, ports=[("String1", 1)])

    def op2(self, nid: str, a, b, out="Float1"):
        n = self.node(nid, ports=[("Float1", 0), ("Float2", 0), (out, 1)])
        self.link(self.port(a, "Float1"), self.port(n, "Float1"))
        self.link(self.port(b, "Float1"), self.port(n, "Float2"))
        return n

    def timeplot(self, channel: str, value_node_sid: str, value_port="Float1"):
        tp = self.node("TimePlot", ports=[
            ("String1", 0), ("Color1", 0), ("String2", 0), ("Float1", 0)])
        self.link(self.port(self.const_string(channel), "String1", 1), self.port(tp, "String1", 0))
        self.link(self.port(value_node_sid, value_port, 1), self.port(tp, "Float1", 0))
        return tp


def main() -> None:
    g = G()

    # Controller so the graph is a valid tennis bot (idle commands).
    ctrl = g.node("TennisController", ports=[
        ("Vector31", 0), ("Bool1", 0), ("Float1", 0), ("Bool2", 0)])
    zero_vec = g.node("ConstructVector3", ports=[
        ("Float1", 0), ("Float2", 0), ("Float3", 0), ("Vector31", 1)])
    for fp, v in (("Float1", 0.0), ("Float2", 0.0), ("Float3", 0.0)):
        c = g.const_float(v)
        g.link(g.port(c, "Float1"), g.port(zero_vec, fp))
    g.link(g.port(zero_vec, "Vector31"), g.port(ctrl, "Vector31"))
    g.link(g.port(g.const_bool(False), "Bool1"), g.port(ctrl, "Bool1"))
    flat = g.const_float(2.0)
    g.link(g.port(flat, "Float1"), g.port(ctrl, "Float1"))
    g.link(g.port(g.const_bool(False), "Bool1"), g.port(ctrl, "Bool2"))

    # 1 constant
    g.timeplot("T_const7", g.const_float(7.0))
    # 2 unwired addend
    add_null = g.node("AddFloats", ports=[("Float1", 0), ("Float2", 0), ("Float1", 1)])
    three = g.const_float(3.0)
    g.link(g.port(three, "Float1"), g.port(add_null, "Float1"))
    g.timeplot("T_null_add", add_null)
    # 3-5 division semantics
    # SAFETY: the game's v0.14 engine breaks on inf/nan in draw/UI paths
    # (reproduced: full-black screen + hang). Plot CLAMPED results instead:
    # +inf -> 1e30, -inf -> -1e30, nan passes through clamp per Unity
    # Mathf.Clamp semantics — the channel value still discriminates the
    # three cases (finite normal results are never anywhere near 1e30).
    one, zero, none = g.const_float(1.0), g.const_float(0.0), g.const_float(-1.0)
    clamp = 1e30
    for ch, a, b in (
        ("T_div_pos0", one, zero),
        ("T_div_neg0", none, zero),
        ("T_div_00", zero, zero),
    ):
        div = g.op2("DivideFloats", a, b)
        cl = g.node("ClampFloat", ports=[
            ("Float1", 0), ("Float2", 0), ("Float3", 0), ("Float1", 1)])
        lo, hi = g.const_float(-clamp), g.const_float(clamp)
        g.link(g.port(div, "Float1", 1), g.port(cl, "Float1", 0))
        g.link(g.port(lo, "Float1", 1), g.port(cl, "Float2", 0))
        g.link(g.port(hi, "Float1", 1), g.port(cl, "Float3", 0))
        g.timeplot(ch, cl)
    # 6 bool -> float coercion (Bool node has Bool1 out; TimePlot takes Float1)
    g.timeplot("T_bool_coerce", g.const_bool(True), "Bool1")
    # 7 conditional with coerced float condition (0.5 -> ?)
    cond = g.node("ConditionalSetBool", ports=[
        ("Bool1", 0), ("Bool2", 0), ("Bool3", 0), ("Bool1", 1)])
    half = g.const_float(0.5)
    g.link(g.port(half, "Float1"), g.port(cond, "Bool1"))
    g.link(g.port(g.const_bool(True), "Bool1"), g.port(cond, "Bool2"))
    g.link(g.port(g.const_bool(False), "Bool1"), g.port(cond, "Bool3"))
    g.timeplot("T_float_cond", cond, "Bool1")
    # 8-9 comparisons
    cmp_eq = g.node("CompareFloats", modifier="0", ports=[
        ("Bool1", 1), ("Float1", 0), ("Float2", 0)])
    a5 = g.const_float(5.0)
    g.link(g.port(a5, "Float1"), g.port(cmp_eq, "Float1"))
    g.link(g.port(a5, "Float1"), g.port(cmp_eq, "Float2"))
    g.timeplot("T_cmp_eq", cmp_eq, "Bool1")
    cmp_lt = g.node("CompareFloats", modifier="1", ports=[
        ("Bool1", 1), ("Float1", 0), ("Float2", 0)])
    a3 = g.const_float(3.0)
    g.link(g.port(a3, "Float1"), g.port(cmp_lt, "Float1"))
    g.link(g.port(a5, "Float1"), g.port(cmp_lt, "Float2"))
    g.timeplot("T_cmp_lt", cmp_lt, "Bool1")
    # 10 latch: x = x + 1 every tick (SetVariable runs in settle)
    getx = g.node("GetVariable", modifier="probe_x", ports=[("Any1", 1)])
    addx = g.node("AddFloats", ports=[("Float1", 0), ("Float2", 0), ("Float1", 1)])
    g.link(g.port(getx, "Any1"), g.port(addx, "Float1"))
    one = g.const_float(1.0)
    g.link(g.port(one, "Float1"), g.port(addx, "Float2"))
    setx = g.node("SetVariable", modifier="probe_x", ports=[("Any1", 0)])
    g.link(g.port(addx, "Float1"), g.port(setx, "Any1"))
    g.timeplot("T_latch_x", getx, "Any1")
    # 11-13 sensors by INDEX (exercises dropdown resolution)
    for idx, ch in ((21, "T_court_w"), (23, "T_net_h"), (47, "T_delta"), (48, "T_fdt")):
        s = g.node("TennisGetFloat", modifier=str(idx), ports=[("Float1", 1)])
        g.timeplot(ch, s)
    # 14 empty-modifier Operation
    op_empty = g.node("Operation", modifier="", ports=[
        ("Float1", 0), ("Float2", 0), ("Float1", 1)])
    a4 = g.const_float(4.0)
    g.link(g.port(a4, "Float1"), g.port(op_empty, "Float1"))
    g.timeplot("T_op_empty", op_empty)

    io.open(OUT, "w", encoding="utf-8").write(json.dumps({
        "serializableNodes": g.nodes,
        "serializableConnections": g.conns,
    }, indent=1))
    print("wrote", OUT, f"({len(g.nodes)} nodes, {len(g.conns)} links)")


if __name__ == "__main__":
    main()
