"""Game-vs-sim compiler-probe check (read-only scoring).

Usage: python scripts/compare_compiler_probe.py <game_timeplot_export.json>

The game export comes from loading data/compiler_probes/compiler_probe.txt
(+ live_position_fib.txt) as a team AI, recording a TimePlot, and exporting
it. Unity exports use comma decimals — handled like compare_timeplots.py.

What it checks (tick-aligned via the CC.tick channel itself):
  static   CC.const/cse_a/cse_b/gcd/pw/found/fib20 — exact vs goldens.
           Pure compute: any mismatch is a COMPILER issue (game evaluates
           the nodes differently from the sim).
  order    CC.tick/pow2/fact/horner/fib_lat — exact vs goldens. These pin
           latch init (0) AND settle order (cnt before fact; fib t,a,b).
           A mismatch here with static channels green = evaluation ORDER
           differs in game, not the math.
  live     LIVE.x/mi/fib — self-consistency per tick (fib(mi) == table,
           mi == floor(mod(x, 20))). Holds in ANY world; checks the
           sensor->loop pipeline on live game data.

Exit 0 iff everything passes. Tolerances: 1e-2 abs (export rounding).
"""
from __future__ import annotations

import json
import math
import re
import sys
from pathlib import Path

TOL = 1e-2

STATIC_GOLDENS = {
    "CC.const": 14.0, "CC.cse_a": 12.0, "CC.cse_b": 12.0,
    "CC.gcd": 6.0, "CC.pw": 128.0, "CC.found": 1.0, "CC.fib20": 6765.0,
}
LATCH_GOLDENS = {
    # tick: (tick, pow2, fact, horner, fib_lat)
    "CC.tick": lambda n: float(n),
    "CC.pow2": lambda n: float(2 ** (n - 1)),
    "CC.fact": lambda n: float(math.factorial(n)),
    "CC.horner": lambda n: ((3 * n + 2) * n + 1) * n + 5,
    "CC.fib_lat": lambda n: [1, 1, 2, 3, 5, 8, 13, 21, 34, 55][n - 1]
    if n <= 10 else None,
}
FIB_TABLE = [0, 1, 1, 2, 3, 5, 8, 13, 21, 34, 55, 89, 144, 233, 377,
             610, 987, 1597, 2584, 4181]


def load_eu_json(path: Path) -> dict:
    text = path.read_text(encoding="utf-8-sig")
    text = re.sub(r"(?<=\d),(?=\d)", ".", text)
    return json.loads(text)


def series_map(doc: dict) -> dict[str, list[float]]:
    out = {}
    for s in doc.get("series") or []:
        if s.get("name"):
            out[s["name"]] = [float(v) for v in s.get("y", [])]
    return out


def main() -> int:
    if len(sys.argv) < 2:
        print(__doc__)
        return 2
    doc = load_eu_json(Path(sys.argv[1]))
    sm = series_map(doc)
    print(f"game export: simTime={doc.get('simTime')} channels={len(sm)}")

    fails = []

    def check(name, cond, detail):
        flag = "ok  " if cond else "FAIL"
        print(f"{flag} {name}: {detail}")
        if not cond:
            fails.append(name)

    # Align by CC.tick: find first sample reading tick 1.
    tick = sm.get("CC.tick")
    if not tick:
        print("FAIL CC.tick channel missing — is the probe loaded?")
        return 1
    try:
        t0 = next(i for i, v in enumerate(tick) if abs(v - 1.0) < TOL)
    except StopIteration:
        print("FAIL no tick-1 sample in CC.tick — latch init differs?")
        return 1
    n_ticks = min(10, len(tick) - t0)
    print(f"aligned at sample {t0}, checking {n_ticks} ticks")

    for ch, want in STATIC_GOLDENS.items():
        ys = sm.get(ch)
        if not ys:
            check(ch, False, "missing channel")
            continue
        errs = [abs(ys[t0 + k] - want) for k in range(min(n_ticks, len(ys) - t0))]
        check(ch, max(errs) <= TOL, f"want {want}, maxerr {max(errs):.4f}")

    for ch, fn in LATCH_GOLDENS.items():
        if ch == "CC.tick":
            continue
        ys = sm.get(ch)
        if not ys:
            check(ch, False, "missing channel")
            continue
        errs = []
        for k in range(min(n_ticks, len(ys) - t0)):
            want = fn(k + 1)
            if want is None:
                continue
            errs.append(abs(ys[t0 + k] - want))
        if not errs:
            check(ch, False, "no comparable ticks")
        else:
            hint = "" if max(errs) <= TOL else \
                "  <-- order/init differs? static channels above tell which"
            check(ch, max(errs) <= TOL, f"maxerr {max(errs):.4f}{hint}")

    # Live self-consistency (any world, game or sim).
    lx, lmi, lf = sm.get("LIVE.x"), sm.get("LIVE.mi"), sm.get("LIVE.fib")
    if lx is None or lmi is None or lf is None:
        print("info LIVE.* channels absent (live probe not loaded?) — skipped")
    else:
        n = min(len(lx), len(lmi), len(lf))
        bad = 0
        for i in range(n):
            m = lx[i] % 20.0
            if m < 0:
                m += 20.0
            want_mi = m - (m % 1.0)
            if abs(lmi[i] - want_mi) > TOL:
                bad += 1
                continue
            if not (0 <= lmi[i] <= 19) or \
                    abs(lf[i] - FIB_TABLE[int(lmi[i])]) > TOL:
                bad += 1
        check("LIVE.self-consistent", bad == 0, f"{n - bad}/{n} ticks")

    print("VERDICT:", "PASS" if not fails else f"FAIL {fails}")
    return 0 if not fails else 1


if __name__ == "__main__":
    sys.exit(main())
