"""One-off: tally v0.14/v0.15 getter-gate resolution from a probe log.

Streams the (large) probe JSONL and reports, per gate class, the
(option_index -> selected_label) pairs the GAME resolved at runtime.
That tells us whether numeric modifiers matched a label.

Usage: python scripts/archive/_probe_gates_tally.py <jsonl> [max_lines]
"""
from __future__ import annotations

import json
import sys
from collections import Counter, defaultdict


def main() -> None:
    path = sys.argv[1]
    gate_lines = 0
    per_gate = defaultdict(Counter)
    per_gate_names = defaultdict(Counter)
    kinds = Counter()
    with open(path, encoding="utf-8", errors="replace") as fh:
        for line in fh:
            if '"v014_getter_gate"' not in line and \
               '"getter_gate"' not in line:
                continue
            gate_lines += 1
            try:
                obj = json.loads(line)
            except json.JSONDecodeError:
                continue
            kinds[obj.get("kind", "?")] += 1
            if obj.get("kind") != "v014_getter_gate":
                continue
            gate = obj.get("gate", "?")
            idx = obj.get("option_index")
            label = obj.get("selected_label")
            per_gate[gate][(idx, label)] += 1
            for item in obj.get("items", []):
                if item.get("label"):
                    per_gate_names[gate][item["label"]] += 1
    print(f"gate lines={gate_lines} kinds={dict(kinds)}")
    for gate, tally in sorted(per_gate.items()):
        print(f"\n== {gate} ==")
        for (idx, label), n in tally.most_common(12):
            print(f"   option_index={idx!r:>6}  selected_label={label!r}  x{n}")
        names = per_gate_names.get(gate)
        if names:
            print("   items seen:", list(names)[:24])


if __name__ == "__main__":
    main()
