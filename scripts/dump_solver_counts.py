"""Dump solver replay mismatches (mirrors tests/tennis_v014_solver_parity.rs)."""
import json, struct, os

FIX = os.path.join(os.environ.get("CARGO_MANIFEST_DIR", r"C:\gitProjects\aia_comp-sim"),
                   "tests", "fixtures", "tennis-v014", "shot_solver.jsonl")

def w(x):
    return struct.unpack("<f", struct.pack("<I", x))[0]

def words(v):
    return [w(i) for i in v] if isinstance(v, list) else []

n_total = 0
n_fb = 0
mismatch = []
with open(FIX, encoding="utf-8") as f:
    for line in f:
        line = line.strip()
        if not line:
            continue
        case = json.loads(line)
        calls = case.get("calls", [])
        call = next((c for c in calls if c.get("operation") == "ComputeShotVelocity"), None)
        if call is None or call.get("invoke_ok") is not True:
            continue
        game = words(call.get("result_f32_words"))
        if len(game) != 3:
            continue
        n_total += 1
        if case.get("case_id") == "fallback":
            n_fb += 1
print("total replayable:", n_total, "fallback rows:", n_fb, "non-fallback:", n_total - n_fb)
