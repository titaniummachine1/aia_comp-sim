"""Dump dimensions (profile, flight_pace, curve, power) per case_id/shot_type."""
import json, struct, os
FIX = os.path.join(r"C:\gitProjects\aia_comp-sim", "tests", "fixtures", "tennis-v014", "shot_solver.jsonl")
def w(x): return struct.unpack("<f", struct.pack("<I", x))[0]
def words(v): return [w(i) for i in v] if isinstance(v, list) else []
ARGN = {0:"Topspin",1:"Slice",2:"Flat",3:"Lob",4:"Drop",5:"CurveL",6:"CurveR"}
seen = set()
with open(FIX, encoding="utf-8") as f:
    for line in f:
        if not line.strip(): continue
        c = json.loads(line); d = c["dimensions"]
        key = (c["case_id"], d["shot_type"], d["profile"],
               w(d["flight_pace_f32_words"][0]), w(d["curve_f32_words"][0]),
               w(d["power_f32_words"][0]), d["team"])
        if key in seen: continue
        seen.add(key)
        print(f"{c['case_id']:>12} arg={ARGN.get(d['shot_type'],d['shot_type']):>8} prof={d['profile']} "
              f"pace={w(d['flight_pace_f32_words'][0]):.3f} curve={w(d['curve_f32_words'][0]):.3f} "
              f"q={w(d['power_f32_words'][0]):.2f} team={d['team']}")
