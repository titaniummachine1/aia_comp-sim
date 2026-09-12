"""Phase B: read the GAME'S answer for every policy channel in the battery.

Usage:
  python scripts/read_battery_from_game.py "<timeplot export>.json" [--json]

The battery graph (`data/interpreter_probes/semantics_battery.txt`) wires every
ambiguous interpreter semantic to a `B.*` TimePlot channel. Loading it as a
tennis bot in the live game, letting it tick, and letting `quit` auto-export a
TimePlot yields the authoritative value for each policy question (div-by-zero,
ConditionalSet hold rule, initial variable value, unwired input, coercion).

The game writes TimePlots with decimal COMMAS (Windows locale). Rather than
re-derive the rule, this reuses the proven
`modhost/parse_timeplots.py` disambiguation when it is reachable (set
`MODHOST_DIR`, else the sibling checkout is tried); the identical regex is the
fallback so the script never silently mis-parses.

Channel metadata (which channels are policy questions, and their goldens) comes
from `data/interpreter_probes/semantics_battery.channels.json`, emitted by
`scripts/gen_interpreter_battery.py`.
"""
from __future__ import annotations

import argparse
import importlib.util
import io
import json
import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
SIDECAR = os.path.join(ROOT, "data", "interpreter_probes",
                       "semantics_battery.channels.json")
DECIMAL_COMMA = re.compile(r"(?<=[0-9]),(?=[0-9])")


def _game_parser():
    """Return a `text -> dict` parser, preferring modhost's proven one."""
    cands = [
        os.environ.get("MODHOST_DIR"),
        r"C:\gitProjects\AIA_tennis\modhost",
    ]
    for c in cands:
        if c and os.path.isfile(os.path.join(c, "parse_timeplots.py")):
            spec = importlib.util.spec_from_file_location(
                "parse_timeplots", os.path.join(c, "parse_timeplots.py"))
            mod = importlib.util.module_from_spec(spec)
            spec.loader.exec_module(mod)
            return mod.parse_timeplot_json
    return lambda text: json.loads(DECIMAL_COMMA.sub(".", text))


def load_export(path: str) -> dict:
    parse = _game_parser()
    return parse(io.open(path, encoding="utf-8", errors="replace").read())


def series_of(export: dict) -> "dict[str, tuple[list, list]]":
    out = {}
    for s in export.get("series", []):
        out[s.get("name")] = (s.get("x") or [], s.get("y") or [])
    return out


def load_sidecar() -> dict:
    if not os.path.isfile(SIDECAR):
        return {}
    return json.load(io.open(SIDECAR, encoding="utf-8"))


def verdict(channel: str, ys: "list[float]", info: dict) -> str:
    """One-line reading of a policy channel's game values."""
    if not ys:
        return "no samples"
    if info.get("kind") == "bool":
        return "bool source — game Float plot reads 0 (not game-comparable)"
    first, last = ys[0], ys[-1]
    uniq = []
    for v in ys:
        if not uniq or uniq[-1] != v:
            uniq.append(v)
    if channel.startswith("B.div_") or channel == "B.mod_zero":
        if last != last:  # NaN
            return "NaN (IEEE 0/0 or x%0)"
        if abs(last) > 1e29:
            return f"{'IEEE inf' if channel != 'B.mod_zero' else 'IEEE nan-clamped'}"
        return f"guarded/zero ({last})"
    if channel.startswith("B.hold_"):
        held = last == first or len(uniq) <= 2
        return ("HOLD (previous-tick)" if held else f"NOT held (alternates: {uniq[:6]})")
    if channel == "B.latch_b":
        return f"initial={first} (0 => vars start at 0/false)"
    if channel == "B.latch_a":
        return f"series {uniq[:6]} (expect 1,2,3,...)"
    if info.get("golden") is not None:
        ok = abs(last - info["golden"]) <= 1e-4
        return f"{'golden OK' if ok else 'GOLDEN MISMATCH'} (want {info['golden']})"
    return f"first={first} last={last}"


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("export", help="game TimePlot export (.json) or a dir")
    ap.add_argument("--json", action="store_true", help="emit machine JSON")
    a = ap.parse_args()

    path = a.export
    if os.path.isdir(path):
        files = sorted(f for f in os.listdir(path) if f.endswith(".json"))
        if not files:
            print(f"no .json in {path}", file=sys.stderr)
            return 2
        path = os.path.join(path, files[-1])

    export = load_export(path)
    sers = series_of(export)
    side = load_sidecar()
    battery = {k: v for k, v in sers.items() if k.startswith("B.")}

    if not battery:
        print(f"{os.path.basename(path)}: NO B.* channels; series are:")
        print("  " + ", ".join(sorted(sers))[:2000])
        return 1

    table = {}
    for ch in sorted(battery):
        ys = battery[ch][1]
        info = side.get(ch, {})
        table[ch] = {
            "first": ys[0] if ys else None,
            "last": ys[-1] if ys else None,
            "ticks": len(ys),
            "policy": bool(info.get("policy")),
            "question": info.get("question"),
            "verdict": verdict(ch, ys, info),
        }

    if a.json:
        print(json.dumps({"file": path, "channels": table}, indent=1))
        return 0

    print(f"{os.path.basename(path)}: {len(sers)} series, "
          f"{len(battery)} B.* battery channels\n")
    print(f"{'channel':14} {'game value':>16}  {'ticks':>5}  reading")
    print("-" * 78)
    for ch in sorted(battery):
        t = table[ch]
        mark = "*" if t["policy"] else " "
        print(f"{mark}{ch:13} {str(t['last']):>16}  {t['ticks']:>5}  {t['verdict']}")
    print("\n* = policy question (the battery exists to answer this from the game)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
