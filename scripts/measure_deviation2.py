"""Measure early/late Z deviation on v44-prefixed native channels.

For each SwingHeld 1->0 release of the HOME bot (v44_*): aim (AimX,AimZ),
first-ball-bounce landing, off_z = land_z - aim_z, RacketDist at release
(timing proxy), side sign from SelfX. Prints the grading calibration +
sign/magnitude law per your description.
"""
import json, re, glob, os

tp = os.path.join(os.environ["USERPROFILE"], "AppData", "LocalLow",
                  "Unicorn One", "AIComp", "Saves", "Tennis", "Timeplots")
files = sorted(glob.glob(os.path.join(tp, "*seed1*.json")), key=os.path.getmtime)[-8:]
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
        print("skip", os.path.basename(path), str(e)[:80])
        continue
    times = S[0]["x"]
    n = len(times)
    inst = {}
    for s in S:
        nm = s.get("name", "?")
        if nm.startswith("v44_") and nm not in inst:
            inst[nm] = s["y"]
    need = ["v44_AimX", "v44_AimZ", "v44_ChargePct", "v44_SwingHeld",
            "v44_RacketDist", "v44_SelfX", "v44_BallX", "v44_BallY", "v44_BallZ"]
    if any(k not in inst for k in need):
        print("skip (channels):", os.path.basename(path),
              sorted(inst)[:6])
        continue
    AIMX, AIMZ = inst["v44_AimX"], inst["v44_AimZ"]
    CHG, HELD = inst["v44_ChargePct"], inst["v44_SwingHeld"]
    RD = inst["v44_RacketDist"]
    BX, BY, BZ = inst["v44_BallX"], inst["v44_BallY"], inst["v44_BallZ"]
    SX = inst["v44_SelfX"]
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
for lo, hi, name in [(None, 1.05, "PERFECT rd<=1.05"), (1.05, 1.85, "GOOD 1.05-1.85"),
                     (1.85, 2.6, "WIDE 1.85-2.6"), (2.6, 99, "OUT >2.6")]:
    g = [r for r in all_rows if (lo is None or r["rd"] > lo) and r["rd"] <= hi]
    if not g:
        print("%s: n=0" % name)
        continue
    oz = sorted(r["off_z"] for r in g)
    med = oz[len(oz) // 2]
    signed = [r["off_z"] * -r["sgn"] for r in g]
    mn = sum(signed) / len(signed)
    early = sum(1 for v in signed if v > 0.3)
    late = sum(1 for v in signed if v < -0.3)
    print("%s: n=%d off_z med=%+.2f p25=%+.2f p75=%+.2f max|.|=%.2f "
          "early-law=%d late-law=%d" % (
              name, len(g), med, oz[len(oz)//4], oz[3*len(oz)//4],
              max(abs(v) for v in oz), early, late))
