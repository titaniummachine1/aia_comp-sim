"""Is the strike range a sphere, a cylinder, or does the racket track the ball's
height? Answers the user's hypothesis:

  (A) swing radius differs vertically vs horizontally  -> range is an ellipsoid
      / cylinder: RacketDist reported by the game = horizontal (XZ) distance,
      and the vertical gap does not shrink the reachable radius.
  (B) the racket moves up/down with the ball height     -> RacketY tracks BallY.

Method: read the game's own RacketX/Y/Z + BallX/Y/Z + RacketDist channels and
compare reported RacketDist against the horizontal and full-3D distances.
"""
import json, re, os, glob, math, statistics as st

TP = os.path.join(os.environ["USERPROFILE"], "AppData", "LocalLow",
                  "Unicorn One", "AIComp", "Saves", "Tennis", "Timeplots")

def load_series(path):
    with open(path, encoding="utf-8") as f:
        raw = f.read()
    raw = re.sub(r"(?<=\d),(?=\d)", ".", raw)
    S = json.loads(raw)["series"]
    inst = {}
    for s in S:
        nm = s.get("name", "?")
        nm = re.sub(r"^v\d+_", "", nm)  # strip v44_/v49_ prefixes
        if nm not in inst:
            inst[nm] = s["y"]
    return inst

files = sorted(glob.glob(os.path.join(TP, "*.json")), key=os.path.getmtime)[-6:]
print("files:")
for f in files:
    print("  ", os.path.basename(f))

NEED = ["RacketX", "RacketY", "RacketZ", "BallX", "BallY", "BallZ",
        "RacketDist", "InSwingRange", "DNow"]
rows = []
for path in files:
    try:
        inst = load_series(path)
    except Exception as e:
        print("skip", os.path.basename(path), e)
        continue
    if any(k not in inst for k in NEED):
        print("skip (missing channel):", os.path.basename(path),
              [k for k in NEED if k not in inst])
        continue
    n = min(len(inst[k]) for k in NEED)
    RX, RY, RZ = inst["RacketX"], inst["RacketY"], inst["RacketZ"]
    BX, BY, BZ = inst["BallX"], inst["BallY"], inst["BallZ"]
    RD, INR = inst["RacketDist"], inst["InSwingRange"]
    for i in range(n):
        if INR[i] < 0.5:
            continue
        dx, dy, dz = BX[i] - RX[i], BY[i] - RY[i], BZ[i] - RZ[i]
        horiz = math.hypot(dx, dz)
        full = math.sqrt(dx * dx + dy * dy + dz * dz)
        rows.append({"ry": RY[i], "by": BY[i], "dy": dy,
                     "rd": RD[i], "horiz": horiz, "full": full})

print("\nin-swing samples:", len(rows))
if not rows:
    raise SystemExit(0)

# --- Which distance does RacketDist report? ---
def rmse(a, b):
    return math.sqrt(sum((x - y) ** 2 for x, y in zip(a, b)) / len(a))

rd = [r["rd"] for r in rows]
print(f"RacketDist   vs reported : rmse={rmse(rd, rd):.4f} (identity)")
print(f"RacketDist   vs horizontal: rmse={rmse(rd, [r['horiz'] for r in rows]):.4f}")
print(f"RacketDist   vs full 3D   : rmse={rmse(rd, [r['full'] for r in rows]):.4f}")

# --- Does the racket track the ball's height? ---
def pearson(a, b):
    ma, mb = st.mean(a), st.mean(b)
    num = sum((x - ma) * (y - mb) for x, y in zip(a, b))
    da = math.sqrt(sum((x - ma) ** 2 for x in a))
    db = math.sqrt(sum((y - mb) ** 2 for y in b))
    return num / (da * db) if da and db else 0.0

rys = [r["ry"] for r in rows]
bys = [r["by"] for r in rows]
dys = [r["dy"] for r in rows]
print(f"\nRacketY: mean={st.mean(rys):.3f} sd={st.pstdev(rys):.3f} "
      f"min={min(rys):.3f} max={max(rys):.3f}")
print(f"BallY  : mean={st.mean(bys):.3f} sd={st.pstdev(bys):.3f} "
      f"min={min(bys):.3f} max={max(bys):.3f}")
print(f"RacketY-BallY (dy): mean={st.mean(dys):+.3f} sd={st.pstdev(dys):.3f} "
      f"min={min(dys):+.3f} max={max(dys):+.3f}")
print(f"corr(RacketY, BallY) = {pearson(rys, bys):+.3f}")
print(f"corr(RacketY, BallY | only high balls >2.0) = ", end="")
hi = [(r["ry"], r["by"]) for r in rows if r["by"] > 2.0]
if len(hi) > 4:
    print(f"{pearson([a for a,_ in hi],[b for _,b in hi]):+.3f} (n={len(hi)})")
else:
    print(f"n={len(hi)} too few")

# --- Reach geometry: radius of contact vs vertical gap ---
# Bucket by |dy| and report the max horizontal distance seen (the reach edge).
print("\nreach edge: max horizontal dist vs |dy| bucket (in-swing samples)")
for lo, hi2 in [(0, .25), (.25, .5), (.5, 1.0), (1.0, 1.5), (1.5, 2.0), (2.0, 9)]:
    g = [r for r in rows if lo <= abs(r["dy"]) < hi2]
    if not g:
        continue
    hmax = max(r["horiz"] for r in g)
    fmax = max(r["full"] for r in g)
    print(f"  |dy| {lo:.2f}-{hi2:.2f}: n={len(g):5d}  max horiz={hmax:5.2f}  "
          f"max 3D={fmax:5.2f}  mean RD={st.mean([r['rd'] for r in g]):5.2f}")
