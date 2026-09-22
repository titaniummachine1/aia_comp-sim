"""One-off: pull v014_getter_gate records out of the big probe jsonl.

Usage: python scripts/archive/_probe_gates.py [gate-prefix ...]
"""
from __future__ import annotations

import io
import json
import os
import sys

LOG = os.path.expanduser(
    "~/../../gitProjects/AIA_tennis/modhost/v0.15f/probe-startup.jsonl")
LOG = r"c:\gitProjects\AIA_tennis\modhost\v0.15f\probe-startup.jsonl"


def main() -> None:
    wants = sys.argv[1:] or ["TennisGetVector3Gate"]
    found: dict[str, int] = {}
    shown: set[tuple[str, str]] = set()
    with io.open(LOG, encoding="utf-8", errors="replace") as f:
        for line in f:
            if "v014_getter_gate" not in line:
                continue
            try:
                rec = json.loads(line)
            except json.JSONDecodeError:
                continue
            gate = str(rec.get("gate", ""))
            if not any(w in gate for w in wants):
                continue
            found[gate] = found.get(gate, 0) + 1
            key = (gate, str(rec.get("selected_label")))
            if key in shown or len(shown) > 40:
                continue
            shown.add(key)
            print(json.dumps({k: v for k, v in rec.items()
                              if k not in ("options",)}, ensure_ascii=False))
            opts = rec.get("options")
            if opts:
                print(f"    options[{len(opts)}] first5="
                      f"{[o for o in opts[:5]]}")
    print("\ncounts:", found)


if __name__ == "__main__":
    main()
