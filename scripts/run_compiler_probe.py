"""Generate the compiler probe: plain-Python bot -> graphc -> game save.

Run:  python scripts/run_compiler_probe.py

Output: data/compiler_probes/compiler_probe.txt (+ .desc.json for review)

What it is
----------
A game save with NO game sensors — only pure logic (float math, compares,
selects, cross-tick latches) plus one idle api.move so the game accepts it
as a team AI. Every computed value is observable through api.plot channels
only, which makes the same file verifiable in 2 places:

  game  — load the .txt as a team, record/export a TimePlot, read CC.*
  sim   — tests/compiler_probe.rs replays it headless, asserts goldens

Channels (golden values assert tick-by-tick in the Rust test):

  CC.const   2 + 3*4 = 14                  ConstFold canary (must fold to one
                                           const; O1 ops < O0 ops)
  CC.cse_a   12 }  (7+5 plotted twice —    frontend CSE shares the add;
  CC.cse_b   12 }  both must read 12)
  CC.tick    1,2,3,...                     latch sanity (cnt += 1 per tick)
  CC.pow2    1,2,4,8,...                   single-latch iteration
  CC.fact    1,2,6,24,...                  cross-latch read (fact *= new cnt)
  CC.horner  11,39,107,233,...             state-dependent compute: ((3n+2)n+1)n+5
                                           must NOT fold, must match the game
                                           bit-identically
  CC.gcd     6                             unrolled Euclid: 48%18=12, 18%12=6
  CC.fib20   6765 every tick               this-tick iterative fib(20): exact
                                           19 trips unrolled (~20 transitions,
                                           every tick)
  CC.fib_lat 1,1,2,3,5,...                 SAME math as a latch shift register
                                           across ticks (~6 transitions/tick,
                                           converges over ticks). The cost
                                           comparison CC.fib20 vs CC.fib_lat
                                           is the transitions-primary demo:
                                           unrolling never runs fewer
                                           transitions than the work itself.
  CC.pw      128 every tick                break-search: first 2^i > 100.
  CC.found   1.0 every tick                bool accumulator under conditional
                                           break (gated select on bool — game
                                           parity experiment)

Cost model (the point of this file)
-----------------------------------
* size      = node/connection count in the .txt (what the game loads).
* inference = node transitions per tick (edges the VM walks). Unrolling a
  loop NEVER reduces inference (N trips x body cost either way — there is
  no rolled form in the graph language, and both select arms evaluate, so
  inactive trips still fire). The optimizer's job is to keep the flat form
  at exactly the work cost (CSE invariants, fold trip-consts, drop dead
  trips); the author picks this-tick (for/while-unrolled) vs across-ticks
  (latches) by latency needs. graphc-rs prints both per compile; the Rust
  test asserts O1 < O0 and pins the exact transition count.

Bounded control flow (for/while/recursion unroll inline — flat graph out)
-------------------------------------------------------------------------
for i in range(literal) unrolls exact trips (transitions = trips x body,
optimal — nothing inactive). while unrolls to 128 trips + overflow canary
and burns all 128 every tick: use latches across ticks when trips are few
and transitions matter. Recursion inlines to depth 32 + canary; narrow and
shallow only (a branching tree costs the WHOLE tree per tick — untaken
arms evaluate too). Bodies must be sink-free: assign locals, sink after
(set_var/plot/move inside loops or recursive bodies fail loudly).
"""
from __future__ import annotations

import json
import os
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).parent
SIM_ROOT = HERE.parent
GRAPH_C_ROOT = Path(r"C:\gitProjects\aia_graphc")

sys.path.insert(0, str(GRAPH_C_ROOT))
from graphc.ast_fe import compile_source  # noqa: E402

OUT_TXT = SIM_ROOT / "data" / "compiler_probes" / "compiler_probe.txt"
OUT_DESC = SIM_ROOT / "data" / "compiler_probes" / "compiler_probe.desc.json"
LIVE_TXT = SIM_ROOT / "data" / "compiler_probes" / "live_position_fib.txt"
LIVE_DESC = SIM_ROOT / "data" / "compiler_probes" / "live_position_fib.desc.json"

BOT_SOURCE = '''
def bot(api):
    cnt = api.var("cnt")
    nxt = cnt + 1
    api.set_var("cnt", nxt)

    # ConstFold canary: pure constants, must fold to one const downstream.
    api.plot("CC.const", 2 + 3 * 4)

    # Shared add plotted twice: both channels must read 12.
    s = 7 + 5
    api.plot("CC.cse_a", s)
    api.plot("CC.cse_b", s)

    # Latch sanity + single-latch iteration (one step per tick).
    # NOTE: plots read latches through FRESH api.var() calls, never through
    # the pre-settle expression (nxt/np/nf): TimePlots lower in the
    # controllers phase, i.e. post-settle, so an expression over a latch
    # would re-evaluate with the NEW latch state (tick 1 would read 2).
    # Fresh reads make every channel uniformly post-settle: tick N -> N.
    api.plot("CC.tick", api.var("cnt"))
    p = api.var("pow2")
    np = 1.0
    if nxt > 1.5:
        np = p * 2
    api.set_var("pow2", np)
    api.plot("CC.pow2", api.var("pow2"))

    # Cross-latch read: fact uses the NEW cnt (settle order: cnt first).
    f = api.var("fact")
    nf = 1.0
    if nxt > 1.5:
        nf = f * nxt
    api.set_var("fact", nf)
    api.plot("CC.fact", api.var("fact"))

    # State-dependent compute: Horner ((3n+2)n+1)n+5 over a fresh cnt read.
    # Must NOT fold (depends on the latch) and must match the game
    # bit-identically.
    cn = api.var("cnt")
    h = (3 * cn + 2) * cn + 1
    h = h * cn + 5
    api.plot("CC.horner", h)

    # Unrolled Euclid on a fixed pair: 48%18=12, 18%12=6.
    m1 = 48 % 18
    m2 = 18 % m1
    api.plot("CC.gcd", m2)

    # This-tick iterative fib(20): exact 19 trips, pure locals, no latches.
    # Transitions ~= trips x body every tick; correct from tick 1.
    fa = 0.0
    fb = 1.0
    for i in range(19):
        t = fa + fb
        fa = fb
        fb = t
    api.plot("CC.fib20", fb)

    # SAME math as a latch shift register across ticks (order t,a,b —
    # same-tick visibility makes the write order load-bearing; the probe
    # pins settle order too). Converges over ticks, ~6 transitions/tick.
    ft = api.var("fib_t")
    fa2 = api.var("fib_a")
    fb2 = api.var("fib_b")
    api.set_var("fib_t", fa2)
    api.set_var("fib_a", fb2)
    nb = ft + fb2
    if nxt < 1.5:
        nb = 1.0
    api.set_var("fib_b", nb)
    api.plot("CC.fib_lat", api.var("fib_b"))

    # Break-search with a bool accumulator: first 2^i > 100.
    # Conditional break -> gated assignments; pw=128, found=True every tick
    # (stateless: no tick alignment needed for the game check).
    pw = 1.0
    found = False
    for i in range(10):
        pw = pw * 2
        if pw > 100.0:
            found = True
            break
    api.plot("CC.pw", pw)
    ff = 0.0
    if found:
        ff = 1.0
    api.plot("CC.found", ff)

    # Idle drive target so the game accepts the file as a team AI.
    api.move(0, 0)
'''


def backend_exe() -> str:
    exe = os.environ.get("GRAPHC_BACKEND", str(GRAPH_C_ROOT / "target" / "release" / "graphc-rs.exe"))
    if not os.path.exists(exe):
        subprocess.run(["cargo", "build", "--release", "-q"], cwd=str(GRAPH_C_ROOT), check=True)
    return exe


# Live-data probe: fib(position.x) -> debug. Reads the real Team Player 1
# transform every tick (game sensor, not a constant), extracts x, folds it
# into 0..19, and runs the same iterative fib table as CC.fib20 over LIVE
# data. Nothing here is golden-deterministic across worlds — the checkable
# property is SELF-CONSISTENCY per tick: LIVE.fib == fib(int(LIVE.mi)) and
# LIVE.mi == floor(mod(LIVE.x, 20)). Same save in game and sim; the
# relation must hold in both (compare_compiler_probe.py checks it).
LIVE_BOT_SOURCE = '''
def tick(api):
    t = api.soccer_get_transform("Team Player 1")
    p = api.pos_of(t)
    x = api.vec_split(p, 0)
    api.plot("LIVE.x", x)
    m = x % 20
    if m < 0.0:
        m = m + 20
    mi = m - m % 1
    api.plot("LIVE.mi", mi)
    a = 0.0
    b = 1.0
    r = 0.0
    for k in range(20):
        if k == mi:
            r = a
        tt = a + b
        a = b
        b = tt
    api.plot("LIVE.fib", r)
    api.move(0, 0)
'''


def build(bot_source: str, bot_name: str, out_txt: Path, out_desc: Path) -> None:
    desc = compile_source(bot_source, ("soccer", "v0.12"))
    desc["bot_name"] = bot_name
    out_txt.parent.mkdir(parents=True, exist_ok=True)
    with open(out_desc, "w", encoding="utf-8", newline="\n") as f:
        json.dump(desc, f, separators=(",", ":"))
    proc = subprocess.run(
        [backend_exe(), str(out_desc), str(out_txt)],
        check=True,
        capture_output=True,
        text=True,
    )
    print(proc.stdout.strip())
    print(f"desc: {out_desc}")


def main() -> None:
    build(BOT_SOURCE, "compiler_probe", OUT_TXT, OUT_DESC)
    build(LIVE_BOT_SOURCE, "live_position_fib", LIVE_TXT, LIVE_DESC)


if __name__ == "__main__":
    main()
