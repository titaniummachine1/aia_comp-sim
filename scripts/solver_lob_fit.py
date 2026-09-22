"""Fit the Lob/Slice flight rule against EVERY fixture row (not just mismatches).

Hypothesis to test: for Lob the game launches at the *charged speed* along the
aim direction (x,z = dir*speed) and lets the flight time fall out of the
distance, instead of using a family-constant time and scaling x by dist/t.
Also prints, per row, the game-implied time t = dist/|vxz| so a formula for it
can be read off directly.
"""
import json, struct, math, collections

FIX = "tests/fixtures/tennis-v014/shot_solver.jsonl"
G = 28.0
FLOOR_Y = 0.31
CPM, COF = 0.85, 0.45
TABLE = {0: (24.0, 0.08), 1: (16.0, 0.04), 2: (28.0, 0.02),
         3: (12.0, 6.0), 4: (7.6, 0.27), 5: (24.0, 0.08), 6: (24.0, 0.08)}
FULL = {0, 2}
NAMES = {0: "Topspin", 1: "Slice", 2: "Flat", 3: "Lob", 4: "Drop",
         5: "CurveL", 6: "CurveR"}


def w(x):
    return struct.unpack("<f", struct.pack("<I", x))[0]


def ws(v):
    return [w(i) for i in v] if isinstance(v, list) else []


rows = []
for line in open(FIX, encoding="utf-8"):
    line = line.strip()
    if not line:
        continue
    r = json.loads(line)
    d, inp = r["dimensions"], r["input"]
    call = next((c for c in r["calls"] if c["operation"] == "ComputeShotVelocity"), None)
    if call is None or call.get("invoke_ok") is not True:
        continue
    game = ws(call["result_f32_words"])
    if len(game) != 3:
        continue
    q = ws(d.get("power_f32_words"))
    rows.append(dict(case=r["case_id"], profile=d.get("profile"),
                     arg=d.get("shot_type"), q=(q[0] if q else 0.0),
                     from3=ws(inp["from_f32_words"]),
                     aim=ws(inp["aim_target_f32_words"]),
                     game=game))

print("rows:", len(rows))


def speed_for(arg, q):
    base, _ = TABLE[arg]
    high = 1.3 if arg in FULL else CPM + COF * 0.45
    return (CPM + (high - CPM) * q) * base


for arg in (3, 1):
    print(f"\n==== {NAMES[arg]} ====")
    print(f"{'case':10} {'prof':10} {'q':>5} {'dist':>6} {'gx':>8} {'spd':>7} "
          f"{'gspd':>7} {'t_impl':>7} {'t_now':>7} {'gy':>8} {'vy_ball(28)':>11}")
    for r in rows:
        if r["arg"] != arg:
            continue
        fx, fz = r["from3"][0], r["from3"][2]
        tx, tz = r["aim"][0], r["aim"][2]
        dx, dz = tx - fx, tz - fz
        dist = math.hypot(dx, dz)
        gx, gy, gz = r["game"]
        if dist < 1e-3:
            continue
        gspd = math.hypot(gx, gz)
        t_impl = dist / gspd if gspd > 1e-6 else float("nan")
        spd = speed_for(arg, r["q"])
        lift = TABLE[arg][1]
        if arg == 3:
            t_now = max(0.82 + 0.14 * r["q"] + lift * 0.14, 0.88 + 0.08 * r["q"])
        else:
            t_now = 0.5 + 0.12 * r["q"] + lift * 0.08
        vy_ball = (FLOOR_Y - r["from3"][1]) / t_impl + 0.5 * G * t_impl
        print(f"{r['case']:10} {str(r['profile']):10} {r['q']:5.2f} {dist:6.1f} "
              f"{gx:8.3f} {spd:7.3f} {gspd:7.3f} {t_impl:7.4f} {t_now:7.4f} "
              f"{gy:8.3f} {vy_ball:11.3f}")
