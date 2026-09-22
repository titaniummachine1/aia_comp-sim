"""Measure early/late Z deviation from the newest native timeplots.

For each swing release: aim (AimX,AimZ) at release, first bounce landing,
off_z = land_z - aim_z, RacketDist at release (timing proxy), side sign.
Groups: clean (RD<=1.05) vs early-ish (first 2.6 tick) vs late-ish.
Prints distribution of off_z per group + sign law check.
"""
import json, re, glob, os

tp = os.path.join(os.environ["USERPROFILE"], "AppData", "LocalLow",
                  "Unicorn One", "AIComp", "Saves", "Tennis", "Timeplots")
files = sorted(glob.glob(os.path.join(tp, "*.json")), key=os.path.getmtime)[-8:]
print("files:", [os.path.basename(f) for f in files])

def load(path):
    with open(path, encoding="utf-8") as f:
        raw = f.read()
    raw = re.sub(r"(?<=\d),(?=\d)", ".", raw)
    return json.loads(raw)["series"]

all_rows = []
for path in files:
    try:
        S = load(path)
    except Exception as e:
        print("skip", os.path.basename(path), e)
        continue
    times = S[0]["x"]
    n = len(times)
    inst = {}
    for s in S:
        nm = s.get("name", "?")
        if nm not in inst:
            inst[nm] = s["y"]
    need = ["AimX", "AimZ", "ChargePct", "SwingHeld", "RacketDist", "SelfX",
            "BallX", "BallY", "BallZ", "DNow"]
    if any(k not in inst for k in need):
        print("skip (channels):", os.path.basename(path),
              [s.get("name") for s in S][:8])
        continue
    AIMX, AIMZ = inst["AimX"], inst["AimZ"]
    CHG, HELD = inst["ChargePct"], inst["SwingHeld"]
    RD, DNOW = inst["RacketDist"], inst["DNow"]
    BX, BY, BZ = inst["BallX"], inst["BallY"], inst["BallZ"]
    SX = inst["SelfX"]
    for i in range(1, n):
        if HELD[i - 1] > 0.5 and HELD[i] < 0.5:
            chg = CHG[i - 1]
            rd = RD[i]
            aimx, aimz = AIMX[i], AIMZ[i]
            if abs(aimx) <= 7.3 and chg > 0.6:
                continue  # serve
            j = i + 1
            while j < n - 1 and not (BY[j] < BY[j - 1] and BY[j] <= BY[j + 1] and BY[j] < 1.0):
                j += 1
                if j - i > 260:
                    break
            if j >= n - 1 or j - i > 260:
                continue
            sgn = 1.0 if SX[i] >= 0 else -1.0
            all_rows.append({"rd": rd, "off_z": BZ[j] - aimz,
                             "off_x": BX[j] - aimx, "sgn": sgn, "chg": chg})

print("releases with bounce:", len(all_rows))
import statistics as st
for lo, hi, name in [(0, 1.05, "PERFECT rd<=1.05"), (1.05, 1.85, "GOOD 1.05-1.85"),
                     (1.85, 2.6, "WIDE 1.85-2.6"), (2.6, 99, "OUT >2.6")]:
    g = [r for r in all_rows if lo < r["rd"] <= hi] if lo else [r for r in all_rows if r["rd"] <= hi]
    if not g:
        print(f"{name}: n=0"); continue
    oz = sorted(r["off_z"] for r in g)
    med = oz[len(oz) // 2]
    signed = [r["off_z"] * -r["sgn"] for r in g]  # law A: early -> -sgn
    mn = sum(signed) / len(signed)
    print(f"{name}: n={len(g)} off_z med={med:+.2f} p25={oz[len(oz)//4]:+.2f} "
          f"p75={oz[3*len(oz)//4]:+.2f} max|.|={max(abs(v) for v in oz):.2f} "
          f"lawA-mean={mn:+.2f}")
