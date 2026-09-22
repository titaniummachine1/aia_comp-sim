"""Mirror the replay gate: list every non-tolerance case separately."""
import json, struct, os
FIX = r"C:\gitProjects\aia_comp-sim\tests\fixtures\tennis-v014\shot_solver.jsonl"

def w(x):
    return struct.unpack("<f", struct.pack("<I", int(x)))[0]

def words(v):
    return [w(i) for i in v] if isinstance(v, list) else []

bad = []
nb_fb = 0
nb_nfb = 0
tot = 0
with open(FIX, encoding="utf-8") as f:
    for line in f:
        line = line.strip()
        if not line:
            continue
        c = json.loads(line)
        call = next((x for x in c.get("calls", [])
                     if x.get("operation") == "ComputeShotVelocity" and x.get("invoke_ok") is True), None)
        if call is None:
            continue
        game = words(call.get("result_f32_words"))
        if len(game) != 3:
            continue
        tot += 1
        fb = (c.get("case_id") == "fallback")
        if fb:
            nb_fb += 1
        else:
            nb_nfb += 1
        bad.append((c.get("case_id"), c.get("input", {}).get("shot_type"), fb))
print("total:", tot, "nonfallback:", nb_nfb, "fallback:", nb_fb)
from collections import Counter
print(Counter((a, b) for a, b, f in bad))
print(Counter(a for a, b, f in bad if f))
