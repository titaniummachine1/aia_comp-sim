"""Generate compiler optimization-mode parity fixtures.

Run:  python scripts/run_compiler_mode_parity.py

Compiles ONE bot source at every optimize mode (o0/o1/o2) and writes
``data/compiler_probes/mode_parity/{mode}.txt`` (+ ``.desc.json``). The sim
test ``tests/compiler_mode_parity.rs`` runs all three head-to-head tick by
tick and asserts the controller output is byte-identical: the modes may only
drop debug sinks (o1) and visual chrome (o2), never change behavior. o0 is
the default and must keep its TimePlot debug sinks.

The bot deliberately drives ``api.move`` from latch/loop/if state (not a
constant) so parity is observed on the controller wire — no TimePlots needed
for the o1/o2 comparison (they are stripped there by design).
"""
from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).parent
SIM_ROOT = HERE.parent
GRAPH_C_ROOT = Path(r"C:\gitProjects\aia_graphc")

sys.path.insert(0, str(HERE))
sys.path.insert(0, str(GRAPH_C_ROOT))

from run_compiler_probe import backend_exe  # noqa: E402
from graphc.ast_fe import compile_source  # noqa: E402

OUT_DIR = SIM_ROOT / "data" / "compiler_probes" / "mode_parity"

BOT_SOURCE = '''
def tick(api):
    n = api.var("n")
    n2 = n + 1.0
    api.set_var("n", n2)

    # identity-heavy arithmetic: the o0 algebraic folds must not move a bit
    a = n2 * 1.0
    b = a / 1.0
    c = b - 0.0
    h = (3.0 * c + 2.0) * c + 1.0
    h = h * c + 5.0

    # cross-tick shift register (fib) with a first-tick seed
    ft = api.var("ft")
    fa = api.var("fa")
    fb = api.var("fb")
    api.set_var("ft", fa)
    api.set_var("fa", fb)
    nb = ft + fb
    if n2 < 1.5:
        nb = 1.0
    api.set_var("fb", nb)

    # bounded search with conditional break + bool accumulator
    pw = 1.0
    found = False
    for i in range(10):
        pw = pw * 2.0
        if pw > 100.0:
            found = True
            break
    f = 0.0
    if found:
        f = 1.0

    # debug sinks (o0 keeps these; o1/o2 drop them). Read fresh latches so
    # every channel is uniformly post-settle (tick N -> N), like the probe.
    api.plot("MP.n", api.var("n"))
    api.plot("MP.h", h)
    api.plot("MP.fib", api.var("fb"))

    x = h + api.var("fb")
    y = pw + f
    api.move(x, y)
'''


def build(mode: str) -> None:
    desc = compile_source(BOT_SOURCE, ("soccer", "v0.12"), mode)
    desc["bot_name"] = f"mode_parity_{mode}"
    OUT_DIR.mkdir(parents=True, exist_ok=True)
    out_txt = OUT_DIR / f"{mode}.txt"
    out_desc = OUT_DIR / f"{mode}.desc.json"
    with open(out_desc, "w", encoding="utf-8", newline="\n") as f:
        json.dump(desc, f, separators=(",", ":"))
    proc = subprocess.run(
        [backend_exe(), str(out_desc), str(out_txt)],
        check=True,
        capture_output=True,
        text=True,
    )
    print(proc.stdout.strip())


def main() -> None:
    for mode in ("o0", "o1", "o2"):
        build(mode)


if __name__ == "__main__":
    main()
