"""Pin the racket-height tracking law: how does RacketY follow BallY?

Game-measured (798 in-swing samples): corr(RacketY,BallY)=+0.961 and
RacketDist == horizontal (XZ) distance exactly (rmse 0.0000 vs 0.2856 3D).
This script fits racket_y = a + b*ball_y and finds the clamp band.
"""
import json, re, os, glob, math, statistics as st

TP = os.path.join(os.environ["USERPROFILE"], "AppData", "LocalLow",
                  "Unicorn One", "AIComp", "Saves", "Tennis", "Timeplots")

def load_series(path):
    with open(path, encoding="utf-8") as f:
        raw = re.sub(r"(?<=\d),(?=\d)", ".", f.read())
    S = json.loads(raw)["series"]
    inst = {}
    for s in S:
        nm = re.sub(r"^v\d+_", "", s.get("name", "?"))
        if nm not in inst:
            inst[nm] = s["y"]
    return inst

files = sorted(glob.glob(os.path.join(TP, "*.json")), key=os.path.getmtime)[-6:]
xs, ys = [], []
maxr = -9.0
for path in files:
    try:
        inst = load_series(path)
    except Exception:
        continue
    NEED = ["RacketY", "BallY", "InSwingRange"]
    if any(k not in inst for k in NEED):
        continue
    n = min(len(inst[k]) for k in NEED)
    RY, BY, INR = inst["RacketY"], inst["BallY"], inst["InSwingRange"]
    maxr = max(maxr, max(RY[:n]))
    for i in range(n):
        if INR[i] >= 0.5:
            xs.append(BY[i]); ys.append(RY[i])

n = len(xs)
mx, my = st.mean(xs), st.mean(ys)
sxx = sum((x - mx) ** 2 for x in xs)
sxy = sum((x - mx) * (y - my) for x, y in zip(xs, ys))
b = sxy / sxx if sxx else 0.0
a = my - b * mx
resid = [y - (a + b * x) for x, y in zip(xs, ys)]
print(f"n={n}  racketY = {a:+.3f} + {b:.3f} * ballY  "
      f"resid sd={st.pstdev(resid):.3f} max|r|={max(abs(r) for r in resid):.3f}")
print(f"racketY range {min(ys):.3f}..{max(ys):.3f}  ballY range {min(xs):.3f}..{max(xs):.3f}")
for bmax in (2.6, 2.783, 2.9, 3.0):
    hi = [(x, y) for x, y in zip(xs, ys) if x > bmax]
    if hi:
        print(f"  ballY>{bmax}: n={len(hi)} meanRacketY={st.mean([y for _,y in hi]):.3f}")
print("racketY histogram (top):")
import collections
c = collections.Counter(round(y, 1) for y in ys)
for k in sorted(c)[-6:]:
    print(f"  {k:.1f}: {c[k]}")
