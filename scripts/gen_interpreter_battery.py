"""Generate the interpreter semantics battery (a TimePlot-observable graph).

Goal (HANDOFF §14 follow-up): prove interpreter parity on BEHAVIOUR, not just
on corpus replay. Every ambiguous VM semantic is wired to a named TimePlot
channel, so one graph gives a channel-by-channel answer that is readable in
three places with the same file:

  reference  GraphBrain (src/graph/eval.rs)   -> tests/interpreter_semantics_battery.rs
  VM O0      Lowerer + no passes              -> same test (debug sinks kept)
  VM O1      Lowerer + PassManager::o1()      -> same test, TRACE arm only
  game       load the .txt as a team, export a TimePlot, read B.* channels

O1 strips debug sinks by design (`compiler_mode_parity`), so channels are only
comparable reference-vs-O0; O1 is covered by the Pass 1..8 var-commit trace.

Channels (all prefixed `B.`) — each is a case that has NOT been proven yet:

  op_0..15         every Operation dropdown index on (7,3)   (clamped +-1e30)
  cmp_0..4         CompareFloats index on (5,5)              (expect 1,0,0,1,1)
  cmp_lt           CompareFloats(3,5) index 1                (expect 1)
  sel_wired        ConditionalSetFloatV2, both arms wired    (alternates)
  hold_f           ConditionalSetFLOAT, false arm UNWIRED    <-- hold rule
  hold_b           ConditionalSetBOOL,  false arm UNWIRED    <-- hold rule
  hold_v           ConditionalSetVECTOR3, false arm UNWIRED  <-- hold rule
  coerce_b2f       Bool output into a Float slot             (expect 1)
  coerce_f2b       Float(0.5) into a Bool condition slot     (0.5 -> ?)
  div_p0/n0/00     1/0, -1/0, 0/0 (clamped)                  (inf/-inf/nan)
  null_add         AddFloats(3, <unwired>)                   (unwired policy)
  latch_x          x = x + 1 / tick                          (expect 1,2,3,..)
  same_tick        y = x after x = x + 1 in the same tick    <-- store order
  vec_rt_x/y/z     split(construct(1.5, 2.5, 3.5))           (expect 1.5/2.5/3.5)
  clamp_lo/hi/in   ClampFloat(5/-5/0.5, -1, 1)               (1 / -1 / 0.5)
  mod_neg          Modulo(-7, 3)                             (sign policy)
  mod_zero         Modulo(5, 0)                              (nan policy)
  pow_neg          Power(2, -1)                              (expect 0.5)
  not_chain        Not(Not(true))                            (expect 1)

Usage:
  python scripts/gen_interpreter_battery.py
"""

from __future__ import annotations

import io
import json
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
sys.path.insert(0, HERE)
import build_interpreter_probe as bip  # noqa: E402  (reuse the proven emitter)

OUT = os.path.join(ROOT, "data", "interpreter_probes", "semantics_battery.txt")
SIDECAR = os.path.join(ROOT, "data", "interpreter_probes",
                       "semantics_battery.channels.json")

# Channels recorded by this generator, in creation order (sidecar source).
CHANNELS: "list[str]" = []

# Policy channels: behaviour the battery exists to ANSWER (read from the game),
# not to assert. Maps channel -> the question it settles.
POLICY = {
    "B.div_p0": "div-by-zero policy (1/0)",
    "B.div_n0": "div-by-zero policy (-1/0)",
    "B.div_00": "div-by-zero policy (0/0)",
    "B.mod_zero": "modulo-by-zero policy",
    "B.hold_f": "ConditionalSetFloat unwired-false hold rule",
    "B.hold_b": "ConditionalSetBool unwired-false hold rule",
    "B.hold_v_1": "ConditionalSetVector3 unwired-false hold rule",
    "B.hold_v_2": "ConditionalSetVector3 unwired-false hold rule",
    "B.hold_v_3": "ConditionalSetVector3 unwired-false hold rule",
    "B.holdv_mag": "ConditionalSetVector3 unwired-false (Magnitude port, no split)",
    "B.holdv_ctrl_mag": "ConditionalSetVector3 both-arms-wired control",
    "B.null_add": "unwired arithmetic input policy",
    "B.latch_a": "variable read/write order",
    "B.latch_b": "variable initial value",
    "B.coerce_b2f": "bool->float coercion",
    "B.coerce_b2f_add": "Bool->Float coercion into arithmetic (AddFloats(bool,0))",
    "B.coerce_b2f_mul": "Bool->Float coercion into arithmetic (MultiplyFloats(bool,1))",
    "B.coerce_f2b": "float->bool condition coercion",
}

# Golden channels: unambiguous math the fixture must always reproduce.
GOLDEN = {
    "B.cmp_0": 1.0, "B.cmp_1": 0.0, "B.cmp_2": 0.0, "B.cmp_3": 1.0,
    "B.cmp_4": 1.0, "B.cmp_lt": 1.0,
    "B.vec_1": 1.5, "B.vec_2": 2.5, "B.vec_3": 3.5,
    "B.clamp_hi": 1.0, "B.clamp_lo": -1.0, "B.clamp_in": 0.5,
    "B.pow_neg": 0.5, "B.not_chain": 1.0,
}


# Channels whose plotted value comes from a **Bool** node. A game TimePlot
# `Float1` input does not coerce a Bool, so the game reads 0 for these — the
# sim value is real but NOT game-observable through a float plot.
BOOL_KIND = (
    "B.cmp_0", "B.cmp_1", "B.cmp_2", "B.cmp_3", "B.cmp_4", "B.cmp_lt",
    "B.not_chain", "B.coerce_f2b", "B.hold_b",
)


def emit_sidecar() -> dict:
    """Write channel metadata so readers know policy questions vs goldens."""
    table = {}
    for ch in CHANNELS:
        info = {"kind": "bool" if ch in BOOL_KIND else "float"}
        if ch in POLICY:
            info["policy"] = True
            info["question"] = POLICY[ch]
        else:
            info["policy"] = False
        if ch in GOLDEN:
            info["golden"] = GOLDEN[ch]
        table[ch] = info
    os.makedirs(os.path.dirname(SIDECAR), exist_ok=True)
    with io.open(SIDECAR, "w", encoding="utf-8", newline="\n") as f:
        json.dump(table, f, indent=1, sort_keys=True)
    return table


CLAMP = 1e30  # keep inf/nan out of any draw path (game-safe), still readable


def node2(g, nid, a, aport, b, bport, out="Float1"):
    n = g.node(nid, ports=[("Float1", 0), ("Float2", 0), (out, 1)])
    g.link(g.port(a, aport, 1), g.port(n, "Float1", 0))
    g.link(g.port(b, bport, 1), g.port(n, "Float2", 0))
    return n


def cmp_f(g, a, aport, b, bport, mod):
    n = g.node("CompareFloats", modifier=str(mod),
               ports=[("Float1", 0), ("Float2", 0), ("Bool1", 1)])
    g.link(g.port(a, aport, 1), g.port(n, "Float1", 0))
    g.link(g.port(b, bport, 1), g.port(n, "Float2", 0))
    return n


def clamped(g, node_sid, port="Float1"):
    """Wrap a value in ClampFloat(-1e30, +1e30) so inf/nan never reaches a
    draw path (the game hangs on raw inf in v0.14) while staying readable."""
    cl = g.node("ClampFloat", ports=[("Float1", 0), ("Float2", 0),
                                     ("Float3", 0), ("Float1", 1)])
    g.link(g.port(node_sid, port, 1), g.port(cl, "Float1", 0))
    g.link(g.port(g.const_float(-CLAMP), "Float1", 1), g.port(cl, "Float2", 0))
    g.link(g.port(g.const_float(CLAMP), "Float1", 1), g.port(cl, "Float3", 0))
    return cl


def plot_f(g, ch, node_sid, port="Float1"):
    CHANNELS.append(ch)
    g.timeplot(ch, clamped(g, node_sid, port))


def mag(g, vec_sid, port="Vector31"):
    """Magnitude of a vector output as a plain Float port (no Vector3Split)."""
    n = g.node("Magnitude", ports=[(port, 0), ("Float1", 1)])
    g.link(g.port(vec_sid, port, 1), g.port(n, port, 0))
    return n


def split3(g, vec_sid, ch):
    sp = g.node("Vector3Split", ports=[("Vector31", 0), ("Float1", 1),
                                       ("Float2", 1), ("Float3", 1)])
    g.link(g.port(vec_sid, "Vector31", 1), g.port(sp, "Vector31", 0))
    for port in ("Float1", "Float2", "Float3"):
        plot_f(g, f"{ch}_{port[-1]}", sp, port)


def make_vec(g, x, y, z):
    n = g.node("ConstructVector3", ports=[("Float1", 0), ("Float2", 0),
                                          ("Float3", 0), ("Vector31", 1)])
    for pid, v in (("Float1", x), ("Float2", y), ("Float3", z)):
        g.link(g.port(g.const_float(v), "Float1", 1), g.port(n, pid, 0))
    return n


def main() -> None:
    g = bip.G()

    # Controller so the graph loads as a tennis bot (idle, direct-wired:
    # deliberately NO TennisAuto* chain, so aim stays None and the battery
    # stays focused on pure-logic semantics).
    ctrl = g.node("TennisController", ports=[
        ("Vector31", 0), ("Bool1", 0), ("Float1", 0), ("Bool2", 0)])
    g.link(g.port(make_vec(g, 0.0, 0.0, 0.0), "Vector31", 1), g.port(ctrl, "Vector31", 0))
    g.link(g.port(g.const_bool(False), "Bool1", 1), g.port(ctrl, "Bool1", 0))
    g.link(g.port(g.const_float(2.0), "Float1", 1), g.port(ctrl, "Float1", 0))
    g.link(g.port(g.const_bool(False), "Bool1", 1), g.port(ctrl, "Bool2", 0))

    # Cross-tick latch n = n + 1, plus per-tick parity (n % 2 == 1) used to
    # drive every conditional so the battery is tick-varying.
    getn = g.node("GetVariable", modifier="B_n", ports=[("Any1", 1)])
    addn = node2(g, "AddFloats", getn, "Any1", g.const_float(1.0), "Float1")
    setn = g.node("SetVariable", modifier="B_n", ports=[("Any1", 0)])
    g.link(g.port(addn, "Float1", 1), g.port(setn, "Any1", 0))

    # Same-tick store order: b is written from a THIS tick — does the read see
    # the value committed earlier in the same tick, or the previous tick's?
    plot_f(g, "B.latch_a", getn, "Any1")
    getb = g.node("GetVariable", modifier="B_b", ports=[("Any1", 1)])
    setb = g.node("SetVariable", modifier="B_b", ports=[("Any1", 0)])
    g.link(g.port(getn, "Any1", 1), g.port(setb, "Any1", 0))
    plot_f(g, "B.latch_b", getb, "Any1")

    mod2 = node2(g, "Modulo", getn, "Any1", g.const_float(2.0), "Float1")
    parity = cmp_f(g, mod2, "Float1", g.const_float(1.0), "Float1", 0)

    # Operation dropdown: every index on (7,3).
    for i in range(16):
        op = g.node("Operation", modifier=str(i),
                    ports=[("Float1", 0), ("Float2", 0), ("Float1", 1)])
        g.link(g.port(g.const_float(7.0), "Float1", 1), g.port(op, "Float1", 0))
        g.link(g.port(g.const_float(3.0), "Float1", 1), g.port(op, "Float2", 0))
        plot_f(g, f"B.op_{i}", op)

    # CompareFloats dropdown on (5,5) => expect 1,0,0,1,1; plus a real (3<5).
    for i in range(5):
        c = cmp_f(g, g.const_float(5.0), "Float1", g.const_float(5.0), "Float1", i)
        plot_f(g, f"B.cmp_{i}", c, "Bool1")
    plot_f(g, "B.cmp_lt", cmp_f(g, g.const_float(3.0), "Float1",
                                g.const_float(5.0), "Float1", 1), "Bool1")

    # Conditional select with BOTH arms wired (the well-defined case).
    sel = g.node("ConditionalSetFloatV2", ports=[
        ("Bool1", 0), ("Float1", 0), ("Float2", 0), ("Float1", 1)])
    g.link(g.port(parity, "Bool1", 1), g.port(sel, "Bool1", 0))
    g.link(g.port(g.const_float(11.0), "Float1", 1), g.port(sel, "Float1", 0))
    g.link(g.port(g.const_float(22.0), "Float1", 1), g.port(sel, "Float2", 0))
    plot_f(g, "B.sel_wired", sel)

    # THE HOLD RULE: unwired false arm on each ConditionalSet kind. The VM does
    # the game's implicit previous-tick hold; GraphBrain currently returns
    # zero/false and alternates. If these channels ever agree, the rule is
    # either implemented in both or absent in both.
    hf = g.node("ConditionalSetFloatV2", ports=[
        ("Bool1", 0), ("Float1", 0), ("Float2", 0), ("Float1", 1)])
    g.link(g.port(parity, "Bool1", 1), g.port(hf, "Bool1", 0))
    g.link(g.port(g.const_float(7.0), "Float1", 1), g.port(hf, "Float1", 0))
    plot_f(g, "B.hold_f", hf)                       # Float2 UNWIRED

    hb = g.node("ConditionalSetBool", ports=[
        ("Bool1", 0), ("Bool2", 0), ("Bool3", 0), ("Bool1", 1)])
    g.link(g.port(parity, "Bool1", 1), g.port(hb, "Bool1", 0))
    g.link(g.port(g.const_bool(True), "Bool1", 1), g.port(hb, "Bool2", 0))
    plot_f(g, "B.hold_b", hb, "Bool1")              # Bool3 UNWIRED

    hv = g.node("ConditionalSetVector3", ports=[
        ("Bool1", 0), ("Vector31", 0), ("Vector32", 0), ("Vector31", 1)])
    g.link(g.port(parity, "Bool1", 1), g.port(hv, "Bool1", 0))
    g.link(g.port(make_vec(g, 1.0, 2.0, 3.0), "Vector31", 1), g.port(hv, "Vector31", 0))
    split3(g, hv, "B.hold_v")                       # Vector32 UNWIRED

    # ---- Phase-B disambiguation (settle the two game-sourced divergences) ----
    # The vector hold above is read through `Vector3Split`; `Magnitude` reads the
    # SAME node through an ordinary Float port, ruling out a split-path artifact.
    plot_f(g, "B.holdv_mag", mag(g, hv))
    # Control: the very same node type with BOTH arms wired — proves the
    # conditional + Magnitude path is sound (|(1,2,3)| == |(-1,-2,-3)|).
    hv_ctl = g.node("ConditionalSetVector3", ports=[
        ("Bool1", 0), ("Vector31", 0), ("Vector32", 0), ("Vector31", 1)])
    g.link(g.port(parity, "Bool1", 1), g.port(hv_ctl, "Bool1", 0))
    g.link(g.port(make_vec(g, 1.0, 2.0, 3.0), "Vector31", 1),
           g.port(hv_ctl, "Vector31", 0))
    g.link(g.port(make_vec(g, 4.0, 5.0, 6.0), "Vector31", 1),
           g.port(hv_ctl, "Vector32", 0))
    plot_f(g, "B.holdv_ctrl_mag", mag(g, hv_ctl))

    # Coercions: bool -> Float slot, and a float used as a condition.
    plot_f(g, "B.coerce_b2f", parity, "Bool1")
    cb = g.node("ConditionalSetBool", ports=[
        ("Bool1", 0), ("Bool2", 0), ("Bool3", 0), ("Bool1", 1)])
    g.link(g.port(g.const_float(0.5), "Float1", 1), g.port(cb, "Bool1", 0))
    g.link(g.port(g.const_bool(True), "Bool1", 1), g.port(cb, "Bool2", 0))
    g.link(g.port(g.const_bool(False), "Bool1", 1), g.port(cb, "Bool3", 0))
    plot_f(g, "B.coerce_f2b", cb, "Bool1")

    # Bool -> Float coercion, isolated from the TimePlot: feed the Bool into an
    # arithmetic node first. If the game coerces, Add(bool,0) == 1; if not, 0.
    ba = g.node("AddFloats", ports=[("Float1", 0), ("Float2", 0), ("Float1", 1)])
    g.link(g.port(parity, "Bool1", 1), g.port(ba, "Float1", 0))
    g.link(g.port(g.const_float(0.0), "Float1", 1), g.port(ba, "Float2", 0))
    plot_f(g, "B.coerce_b2f_add", ba)
    plot_f(g, "B.coerce_b2f_mul",
           node2(g, "MultiplyFloats", parity, "Bool1", g.const_float(1.0), "Float1"))

    # Division edge cases (clamped so they stay observable).
    for ch, a in (("B.div_p0", 1.0), ("B.div_n0", -1.0), ("B.div_00", 0.0)):
        plot_f(g, ch, node2(g, "DivideFloats", g.const_float(a), "Float1",
                            g.const_float(0.0), "Float1"))

    # Unwired arithmetic input.
    an = g.node("AddFloats", ports=[("Float1", 0), ("Float2", 0), ("Float1", 1)])
    g.link(g.port(g.const_float(3.0), "Float1", 1), g.port(an, "Float1", 0))
    plot_f(g, "B.null_add", an)

    # Vector construct/split round trip.
    split3(g, make_vec(g, 1.5, 2.5, 3.5), "B.vec")

    # Clamp behaviour (hi / lo / pass-through).
    for ch, v in (("B.clamp_hi", 5.0), ("B.clamp_lo", -5.0), ("B.clamp_in", 0.5)):
        inner = g.node("ClampFloat", ports=[("Float1", 0), ("Float2", 0),
                                            ("Float3", 0), ("Float1", 1)])
        g.link(g.port(g.const_float(v), "Float1", 1), g.port(inner, "Float1", 0))
        g.link(g.port(g.const_float(-1.0), "Float1", 1), g.port(inner, "Float2", 0))
        g.link(g.port(g.const_float(1.0), "Float1", 1), g.port(inner, "Float3", 0))
        plot_f(g, ch, inner)

    # Modulo / power edges.
    plot_f(g, "B.mod_neg", node2(g, "Modulo", g.const_float(-7.0), "Float1",
                                 g.const_float(3.0), "Float1"))
    plot_f(g, "B.mod_zero", node2(g, "Modulo", g.const_float(5.0), "Float1",
                                  g.const_float(0.0), "Float1"))
    plot_f(g, "B.pow_neg", node2(g, "Power", g.const_float(2.0), "Float1",
                                 g.const_float(-1.0), "Float1"))

    # Double negation.
    n1 = g.node("Not", ports=[("Bool1", 0), ("Bool1", 1)])
    g.link(g.port(g.const_bool(True), "Bool1", 1), g.port(n1, "Bool1", 0))
    n2 = g.node("Not", ports=[("Bool1", 0), ("Bool1", 1)])
    g.link(g.port(n1, "Bool1", 1), g.port(n2, "Bool1", 0))
    plot_f(g, "B.not_chain", n2, "Bool1")

    os.makedirs(os.path.dirname(OUT), exist_ok=True)
    with io.open(OUT, "w", encoding="utf-8", newline="\n") as f:
        json.dump({"serializableNodes": g.nodes,
                   "serializableConnections": g.conns}, f, indent=1)
    chans = sum(1 for n in g.nodes if n["id"] == "TimePlot")
    table = emit_sidecar()
    print(f"wrote {OUT} ({len(g.nodes)} nodes, {len(g.conns)} links, "
          f"{chans} TimePlot channels)")
    print(f"wrote {SIDECAR} ({len(table)} channels, "
          f"{len(POLICY)} policy / {len(GOLDEN)} golden)")


if __name__ == "__main__":
    main()

