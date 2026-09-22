import json, re, glob, os, sys
sys.path.insert(0, r"C:\gitProjects\AIA_tennis\modhost")
from parse_timeplots import parse_timeplot_json
tp = r"C:\Users\Terminatort8000\AppData\LocalLow\Unicorn One\AIComp\Saves\Tennis\Timeplots"
files = sorted(glob.glob(os.path.join(tp, "*.json")), key=os.path.getmtime)[-8:]
tiers = {"PERFECT": [], "EARLY": [], "LATE": [], "NONE": []}
nrel = nb = 0
for path in files:
    try:
        data = parse_timeplot_json(open(path, encoding="utf-8").read())
    except Exception as e:
        print("skip", os.path.basename(path), e)
        continue
    inst = {}
    for s in data.get("series", []):
        if s.get("name", "?") not in inst:
            inst[s["name"]] = s.get("y", [])
    K = ["AimX", "AimZ", "ChargePct", "SwingHeld", "RacketDist",
         "BallX", "BallY", "BallZ", "SelfX"]
    if any(k not in inst for k in K):
        print("skip channels", os.path.basename(path))
        continue
    n = len(inst["AimX"])
    AX, AZ = inst["AimX"], inst["AimZ"]
    CH, HD = inst["ChargePct"], inst["SwingHeld"]
    RD = inst["RacketDist"]
    BX, BY, BZ = inst["BallX"], inst["BallY"], inst["BallZ"]
    SX = inst["SelfX"]
    zt = pt = 0
    for i in range(n):
        d = RD[i]
        zt = zt + 1 if d <= 2.6 else 0
        pt = pt + 1 if d <= 1.0 else 0
        if i > 0 and HD[i - 1] > 0.5 and HD[i] < 0.5:
            nrel += 1
            tier = ("PERFECT" if pt == 1 else
                    "LATE" if (pt >= 2 or zt >= 2) else
                    "EARLY" if zt == 1 else "NONE")
            if abs(AX[i]) <= 7.3 and CH[i - 1] > 0.6:
                continue
            j = i + 1
            while j < n - 1 and not (BY[j] < BY[j - 1] and BY[j] <= BY[j + 1] and BY[j] < 1.0):
                j += 1
                if j - i > 260:
                    break
            if j >= n - 1 or j - i > 260:
                continue
            sgn = 1.0 if SX[i] >= 0 else -1.0
            nb += 1
            tiers[tier].append((BZ[j] - AZ[i], BX[j] - AX[i], sgn))
print("releases:", nrel, "with bounce:", nb)
for t, rows in tiers.items():
    if not rows:
        print(t, "n=0")
        continue
    oz = sorted(r[0] for r in rows)
    sup_early = sum(1 for o, x, s in rows if o * -s > 0.3)
    sup_late = sum(1 for o, x, s in rows if o * s > 0.3)
    print("%s n=%d med=%+.2f p25=%+.2f p75=%+.2f max=%.2f early-law=%d late-law=%d" % (
        t, len(rows), oz[len(oz)//2], oz[len(oz)//4], oz[3*len(oz)//4],
        max(abs(v) for v in oz), sup_early, sup_late))
