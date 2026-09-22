"""Replicate the Rust shot solver in Python and diff against all 210 game rows.

Goal: explain the 10 mismatches (5 mirrored cases: high-pace Slice, high-pace
Lob x3, normal Lob) so the solver can be corrected to 210/210.
"""
import json
import struct
import collections
import math

FIX = "tests/fixtures/tennis-v014/shot_solver.jsonl"
G = 28.0
MAX_SPEED = 46.0
FLOOR_Y = 0.31
CHARGE_POWER_MIN = 0.85
CHARGE_OTHER_FRACTION = 0.45

TABLE = {0: (24.0, 0.08), 1: (16.0, 0.04), 2: (28.0, 0.02),
         3: (12.0, 6.0), 4: (7.6, 0.27), 5: (24.0, 0.08), 6: (24.0, 0.08)}
FULL = {0, 2}
MIN_TIMES = [0.0] * 7  # filled below


def w(x):
    return struct.unpack("<f", struct.pack("<I", x))[0]


def ws(v):
    return [w(i) for i in v] if isinstance(v, list) else []


def solve(from3, aim3, arg, q):
    base, lift = TABLE[arg]
    fx, fz = from3[0], from3[2]
    tx, tz = aim3[0], aim3[2]
    dx, dz = tx - fx, tz - fz
    dist = math.hypot(dx, dz)
    high = 1.3 if arg in FULL else 0.85 + CHARGE_OTHER_FRACTION * 0.45
    factor = CHARGE_POWER_MIN + (high - CHARGE_POWER_MIN) * q
    speed = factor * base
    if dist < 1e-3:
        return None
    dirx, dirz = dx / dist, dz / dist
    if arg == 1:            # Slice
        t = 0.5 + 0.12 * q + lift * 0.08
    elif arg == 3:          # Lob
        t = max(0.82 + 0.14 * q + lift * 0.14, 0.8 + 0.08 * q + 0.08)
    elif arg == 4:          # Drop
        t = 0.63 + 0.13 * q + lift * 0.13
    else:
        t = max(MIN_TIMES[arg], dist / max(speed, 1e-4))
    t_cap = max(2.4, t + 0.35)
    it = 0
    while True:
        vx, vz = dirx * (dist / t), dirz * (dist / t)
        vy = (FLOOR_Y - from3[1]) / t + 0.5 * G * t
        tot = math.sqrt(vx * vx + vz * vz + vy * vy)
        if tot <= MAX_SPEED or it >= 12 or t >= t_cap:
            break
        t += 0.05
        it += 1
    return (dirx * (dist / t), (FLOOR_Y - from3[1]) / t + 0.5 * G * t, dirz * (dist / t))


rows = [json.loads(l) for l in open(FIX) if l.strip()]
buckets = collections.defaultdict(list)
for r in rows:
    d = r["dimensions"]
    inp = r["input"]
    call = next((c for c in r["calls"] if c["operation"] == "ComputeShotVelocity"), None)
    if call is None or call.get("invoke_ok") is not True:
        continue
    game = ws(call["result_f32_words"])
    if len(game) != 3:
        continue
    fp = ws(d.get("flight_pace_f32_words"))
    q = ws(d.get("power_f32_words"))
    vel = ws(inp.get("velocity_f32_words"))
    buckets[(r["case_id"], d.get("profile"))].append(
        dict(from3=ws(inp["from_f32_words"]), aim=ws(inp["aim_target_f32_words"]),
             arg=d.get("shot_type"), q=q[0] if q else 0.0,
             fp=fp[0] if fp else None, vel=vel, game=game, seed=r["case_index"]))

for k in sorted(buckets, key=str):
    v = buckets[k]
    print(k, "n=", len(v), "fp=", v[0]["fp"], "sample aim=", [round(x, 2) for x in v[0]["aim"]],
          "from=", [round(x, 2) for x in v[0]["from3"]])
