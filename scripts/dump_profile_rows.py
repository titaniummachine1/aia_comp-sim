"""Dump every fixture row for a given (case_id, shot arg) with profile + inputs + game result."""
import json, struct, sys, os

FIX = os.path.join(r"C:\gitProjects\aia_comp-sim", "tests", "fixtures", "tennis-v014", "shot_solver.jsonl")

def w(x):
    return struct.unpack("<f", struct.pack("<I", x))[0]

def words(v):
    return [w(i) for i in v] if isinstance(v, list) else []

want_case = sys.argv[1] if len(sys.argv) > 1 else "high-pace"
want_arg = int(sys.argv[2]) if len(sys.argv) > 2 else 3  # 3=Lob
ARGN = {0: "Topspin", 1: "Slice", 2: "Flat", 3: "Lob", 4: "Drop", 5: "CurveL", 6: "CurveR"}

print(f"{'prof':>4} {'q':>5} {'from':>18} {'aim':>18} {'dist':>6} {'game':>26}")
with open(FIX, encoding="utf-8") as f:
    for line in f:
        if not line.strip():
            continue
        c = json.loads(line)
        d = c["dimensions"]
        if c["case_id"] != want_case or d["shot_type"] != want_arg:
            continue
        call = next((x for x in c["calls"] if x["operation"] == "ComputeShotVelocity"), None)
        if not call or call.get("invoke_ok") is not True:
            continue
        g = words(call["result_f32_words"])
        if len(g) != 3:
            continue
        fr = words(c["input"]["from_f32_words"])
        am = words(c["input"]["aim_target_f32_words"])
        q = words(d["power_f32_words"])[0]
        dist = ((am[0]-fr[0])**2 + (am[2]-fr[2])**2) ** 0.5
        print(f"{d['profile']:>4} {q:>5.2f} ({fr[0]:>6.1f},{fr[2]:>5.1f}) ({am[0]:>6.1f},{am[2]:>5.1f}) {dist:>6.1f} "
              f"({g[0]:>8.3f},{g[1]:>8.3f},{g[2]:>7.3f})")
