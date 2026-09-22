"""Inspect the shot_solver fixture: field name + shape of one case."""
import json

rows = [json.loads(l) for l in open("tests/fixtures/tennis-v014/shot_solver.jsonl") if l.strip()]
print("rows:", len(rows))
r = rows[0]
print("top keys:", sorted(r.keys()))
print(json.dumps(r, indent=1)[:2000])
from collections import Counter
print(Counter(x.get("case_id") for x in rows))
print("case_ids sample:", [x.get("case_id") for x in rows[:12]])
