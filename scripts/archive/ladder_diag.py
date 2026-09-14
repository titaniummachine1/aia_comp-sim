"""Diagnose ladder losses: per-row points won, faults, double faults, aces."""
import io
import json
import os

HERE = os.path.dirname(os.path.abspath(__file__))
IN = os.path.join(HERE, "..", "..", "data", "tennis", "titanium_ladder.jsonl")

with io.open(IN, encoding="utf-8") as fh:
    rows = [json.loads(l) for l in fh if l.strip()]

for r in rows:
    pw = r.get("point_winners", [])
    tw = sum(1 for w in pw if w == 0)
    mark = "W" if r.get("winner") == "titanium58" else "L"
    print(f"{mark} {r['away']:32} ticks={r.get('ticks'):>6} sets={r.get('sets')} "
          f"npts={len(pw):>3} Ti_pts={tw:>3} faults={r.get('faults')} "
          f"df={r.get('double_faults')} aces={r.get('aces')}")
