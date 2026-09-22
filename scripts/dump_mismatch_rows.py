"""Dump the full input row for every non-fallback solver mismatch."""
import json, struct, os

FIX = os.path.join(r"C:\gitProjects\aia_comp-sim", "tests", "fixtures", "tennis-v014", "shot_solver.jsonl")


def w(x):
    return struct.unpack("<f", struct.pack("<I", x))[0]


def words(v):
    return [w(i) for i in v] if isinstance(v, list) else []


with open(FIX, encoding="utf-8") as f:
    rows = [json.loads(l) for l in f if l.strip()]

for r in rows:
    cid = r.get("case_id")
    call = next((c for c in r.get("calls", []) if c.get("operation") == "ComputeShotVelocity"), None)
    if call is None or call.get("invoke_ok") is not True:
        continue
    if cid == "fallback":
        continue
    if cid not in ("high-pace Slice q=0", "high-pace Lob q=0", "normal Lob q=1",
                   "high-pace Lob q=0.5"):
        continue
    print("===", cid)
    print("  dimensions:", json.dumps({k: words(v) if isinstance(v, list) else v
                                        for k, v in r["dimensions"].items()}))
    print("  input:", json.dumps({k: words(v) if isinstance(v, list) else v
                                  for k, v in r["input"].items()}))
    print("  game:", words(call["result_f32_words"]))
    print("  keys:", sorted(r.keys()))
