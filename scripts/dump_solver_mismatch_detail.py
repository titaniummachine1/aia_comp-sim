"""Dump WHICH non-fallback solver rows mismatch (mirrors the replay test)."""
import json, struct, os

FIX = os.path.join(r"C:\gitProjects\aia_comp-sim",
                   "tests", "fixtures", "tennis-v014", "shot_solver.jsonl")

def w(x):
    return struct.unpack("<f", struct.pack("<I", x))[0]

def words(v):
    return [w(i) for i in v] if isinstance(v, list) else []

with open(FIX, encoding="utf-8") as f:
    rows = [json.loads(l) for l in f if l.strip()]
print("fixture rows:", len(rows))
n_fb = sum(1 for r in rows if r.get("case_id") == "fallback")
print("fallback rows:", n_fb)
from collections import Counter
c = Counter(r.get("case_id", "?").split()[0] if isinstance(r.get("case_id"), str) else "?" for r in rows)
print("by case prefix:", dict(c))
# show 3 sample non-fallback case ids + whether they replay
n = 0
for r in rows:
    if r.get("case_id") == "fallback":
        continue
    calls = r.get("calls", [])
    call = next((cc for cc in calls if cc.get("operation") == "ComputeShotVelocity"), None)
    if call is None or call.get("invoke_ok") is not True:
        continue
    n += 1
print("replayable non-fallback:", n)
