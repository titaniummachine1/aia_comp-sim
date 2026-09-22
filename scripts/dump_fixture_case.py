"""Dump every fixture case_id + full dims/input for the solver mismatches."""
import json, struct, os

FIX = os.path.join(r"C:\gitProjects\aia_comp-sim", "tests", "fixtures", "tennis-v014",
                   "shot_solver.jsonl")
ARGS = {0: "Topspin", 1: "Slice", 2: "Flat", 3: "Lob", 4: "Drop", 5: "CurveLeft",
        6: "CurveRight"}


def w(x):
    return struct.unpack("<f", struct.pack("<I", x))[0]


def words(v):
    return [w(i) for i in v] if isinstance(v, list) else v


seen = {}
with open(FIX, encoding="utf-8") as f:
    for line in f:
        if not line.strip():
            continue
        r = json.loads(line)
        cid = r.get("case_id")
        seen.setdefault(cid, 0)
        seen[cid] += 1

print("case_ids:", seen)

with open(FIX, encoding="utf-8") as f:
    for line in f:
        if not line.strip():
            continue
        r = json.loads(line)
        if r.get("case_id") != "normal":
            continue
        d = r["dimensions"]
        arg = ARGS.get(d.get("shot_type"), "?")
        if arg != "Lob":
            continue
        print("=== case_id", r.get("case_id"), "shot_type", arg)
        print("  all top keys:", sorted(r.keys()))
        print("  dimensions:", json.dumps({k: words(v) for k, v in d.items()}))
        print("  input:", json.dumps({k: words(v) for k, v in r["input"].items()}))
        break
